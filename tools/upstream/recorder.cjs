// Records what each upstream test does (parses, snapshots, other matchers) into
// $UPSTREAM_RECORDS/<file>.json, for tools/upstream/write.mjs. Jest runs it before each test file.
const fs = require("node:fs");
const path = require("node:path");

const { UPSTREAM_CHECKOUT: checkout, UPSTREAM_RECORDS: records } = process.env;
const src = path.join(checkout, "tsdoc/src");
// Required here, in the test file's module registry, so the patches below reach the classes it
// imports. Through index.ts, the order the published package loads them in.
const { TSDocParser, TSDocConfiguration, TSDocTagSyntaxKind, TextRange } = require(path.join(src, "index.ts"));
const { TestHelpers } = require(path.join(src, "parser/__tests__/TestHelpers.ts"));

const file = { path: path.relative(checkout, expect.getState().testPath), tests: [] };
const problems = [];
let current;
let parsing = 0;
let helper = null;

// Thrown from afterAll, since a throw inside a test could be caught by the test's own code.
function problem(message) {
	problems.push(`${current?.name ?? "outside any test"}: ${message}`);
}

function record(event) {
	if (current) current.events.push(event);
	else problem(`${event.type} event`);
}

const syntaxKinds = {
	[TSDocTagSyntaxKind.InlineTag]: "inline",
	[TSDocTagSyntaxKind.BlockTag]: "block",
	[TSDocTagSyntaxKind.ModifierTag]: "modifier",
};

function docNodes(configuration) {
	const definitions = configuration.docNodeManager._docNodeDefinitionsByKind.values();
	return JSON.stringify([...definitions].map((d) => [d.packageName, d.docNodeKind, [...d.allowedChildKinds]]));
}
const builtInDocNodes = docNodes(new TSDocConfiguration());

function serializeConfiguration(configuration) {
	if (docNodes(configuration) !== builtInDocNodes) problem("custom DocNode definitions aren't recorded");
	const { validation } = configuration;
	return {
		tagDefinitions: configuration.tagDefinitions.map((tag) => ({
			tagName: tag.tagName,
			syntaxKind: syntaxKinds[tag.syntaxKind],
			allowMultiple: tag.allowMultiple,
		})),
		supportedTagDefinitions: configuration.supportedTagDefinitions.map((tag) => tag.tagName),
		supportedHtmlElements: configuration.supportedHtmlElements,
		validation: {
			ignoreUndefinedTags: validation.ignoreUndefinedTags,
			reportUnsupportedTags: validation.reportUnsupportedTags,
			reportUnsupportedHtmlElements: validation.reportUnsupportedHtmlElements,
		},
	};
}

// JSON for a .each row, with tagged objects for what JSON can't hold.
function encode(value) {
	if (value === undefined) return { $undefined: true };
	if (typeof value === "number" && (!Number.isFinite(value) || Object.is(value, -0))) {
		return { $number: Object.is(value, -0) ? "-0" : String(value) };
	}
	if (value === null || ["string", "number", "boolean"].includes(typeof value)) return value;
	if (Array.isArray(value)) return value.map(encode);
	if (typeof value === "object") {
		const fields = Object.fromEntries(Object.entries(value).map(([k, v]) => [k, encode(v)]));
		const prototype = Object.getPrototypeOf(value);
		const isPlain = prototype === null || prototype === Object.prototype;
		return isPlain ? fields : { $instance: value.constructor.name, fields };
	}
	problem(`can't encode a ${typeof value} in a .each row`);
	return null;
}

const { parseString, parseRange } = TSDocParser.prototype;
TSDocParser.prototype.parseString = function (text) {
	record({ type: "parse", text, configuration: serializeConfiguration(this.configuration) });
	parsing++;
	try {
		return parseString.call(this, text);
	} finally {
		parsing--;
	}
};
TSDocParser.prototype.parseRange = function (range) {
	if (!parsing) problem("parseRange called directly, which has no event");
	return parseRange.call(this, range);
};

const { fromString } = TextRange;
TextRange.fromString = function (text) {
	// parseString makes one too.
	if (!parsing) record({ type: "textRange", text });
	return fromString.call(this, text);
};

for (const name of Object.getOwnPropertyNames(TestHelpers)) {
	if (!name.startsWith("parseAndMatch")) continue;
	const original = TestHelpers[name];
	TestHelpers[name] = function (...args) {
		const outer = helper;
		helper ??= name;
		try {
			return original.apply(this, args);
		} finally {
			helper = outer;
		}
	};
}

// State.match is where both snapshot matchers end up, with the key and the inline snapshot as
// Jest compares it (indentation stripped). write.mjs fills in a snapshot's text from its .snap file.
const snapshotState = expect.getState().snapshotState;
const { match } = snapshotState;
snapshotState.match = function (options) {
	const result = match.call(this, options);
	record(
		options.isInline
			? { type: "inlineSnapshot", expected: options.inlineSnapshot }
			: { type: "snapshot", key: result.key, helper, expected: null },
	);
	return result;
};

const snapshotMatchers = new Set(["toMatchSnapshot", "toMatchInlineSnapshot"]);
function recordMatchers(matchers, prefix) {
	return new Proxy(matchers, {
		get(target, key, receiver) {
			const value = Reflect.get(target, key, receiver);
			if (typeof key !== "string") return value;
			if (["not", "resolves", "rejects"].includes(key)) return recordMatchers(value, `${prefix}${key}.`);
			if (typeof value !== "function" || snapshotMatchers.has(key)) return value;
			return function (...args) {
				record({ type: "assertion", matcher: prefix + key });
				return value.apply(this, args);
			};
		},
	});
}
globalThis.expect = new Proxy(expect, {
	apply: (target, self, args) => recordMatchers(Reflect.apply(target, self, args), ""),
});

// A test's template has its describe names, which only exist while Jest collects the tests.
const describes = [];
function trackDescribe(describe) {
	return function (name, body, ...rest) {
		if (typeof name !== "string") throw new Error("recorder: describe() with a non-string name");
		const tracked = function (...args) {
			describes.push(name);
			try {
				return body.apply(this, args);
			} finally {
				describes.pop();
			}
		};
		return describe.call(this, name, tracked, ...rest);
	};
}
globalThis.describe = Object.assign(trackDescribe(describe), describe, {
	only: trackDescribe(describe.only),
	skip: trackDescribe(describe.skip),
	each() {
		throw new Error("recorder: describe.each isn't supported");
	},
});

// test and it are the same function in jest-circus.
for (const variant of new Set([test, test.only, test.skip, it, it.only, it.skip])) {
	const { each } = variant;
	variant.each = function (table, ...taggedValues) {
		const bound = each.call(this, table, ...taggedValues);
		const isTagged = taggedValues.length > 0;
		return function (title, body, ...rest) {
			const template = [...describes, title].join(" ");
			const recorded = function (...row) {
				current.template = template;
				current.each = encode(isTagged ? row[0] : row);
				return body.apply(this, row);
			};
			return bound.call(this, title, recorded, ...rest);
		};
	};
}

beforeEach(() => {
	const name = expect.getState().currentTestName;
	current = { name, template: name, each: null, events: [] };
	file.tests.push(current);
});

afterEach(() => {
	current = undefined;
});

afterAll(() => {
	if (problems.length) throw new Error(`recorder:\n${problems.join("\n")}`);
	const name = `${file.path.replaceAll("/", "__")}.json`;
	fs.writeFileSync(path.join(records, name), JSON.stringify(file));
});
