// The npm package against @microsoft/tsdoc, in the same Node process: for each comment of
// tests/fixtures/corpus.json, the same lines, messages (ids, texts and UTF-16 ranges) and tree,
// or the same error, from parse() and, but for the lines and tree, from check(). parseTree() and
// parseMany() give what parse() does. Run tools/build-npm.sh first.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { test } from "node:test";
import { DocExcerpt, TSDocConfiguration, TSDocParser } from "@microsoft/tsdoc";
import { TSDocConfigFile } from "@microsoft/tsdoc-config";

const require = createRequire(import.meta.url);
const tsdocrs = require("./index.js");
const corpus = JSON.parse(
	readFileSync(new URL("../../tests/fixtures/corpus.json", import.meta.url), "utf8"),
);

/** The tree under `node` as kinds and excerpts, which both trees have. */
function microsoftTree(node) {
	const item = { kind: node.kind };
	if (node instanceof DocExcerpt) {
		const range = node.content.getContainingTextRange();
		Object.assign(item, {
			excerptKind: node.excerptKind,
			content: node.content.toString(),
			pos: range.pos,
			end: range.end,
		});
	}
	const children = node.getChildNodes();
	if (children.length > 0) {
		item.nodes = children.map(microsoftTree);
	}
	return item;
}

/**
 * The same of tsdocrs's tree, leaving out the fields TSDoc's snapshot has no place for. An excerpt
 * on one line has no `content`, which is then `comment`'s text from its `pos` to its `end`.
 */
function tsdocrsTree(node, comment) {
	const item = { kind: node.kind };
	if ("excerptKind" in node) {
		Object.assign(item, {
			excerptKind: node.excerptKind,
			content: node.content ?? comment.slice(node.pos, node.end),
			pos: node.pos,
			end: node.end,
		});
	}
	if (node.nodes) {
		item.nodes = node.nodes.map((child) => tsdocrsTree(child, comment));
	}
	return item;
}

function microsoft({ comment, config }) {
	const configuration = new TSDocConfiguration();
	if (config) {
		TSDocConfigFile.loadFromObject({
			$schema: TSDocConfigFile.CURRENT_SCHEMA_URL,
			...config,
		}).configureParser(configuration);
	}
	try {
		const context = new TSDocParser(configuration).parseString(comment);
		return {
			lines: context.lines.map((line) => line.toString()),
			messages: context.log.messages.map((m) => ({
				messageId: m.messageId,
				text: m.text,
				unformattedText: m.unformattedText,
				pos: m.textRange.pos,
				end: m.textRange.end,
			})),
			docComment: microsoftTree(context.docComment),
		};
	} catch (error) {
		return { throws: error.message };
	}
}

function mine({ comment, config }) {
	try {
		const result = tsdocrs.parse(comment, config);
		return { ...result, docComment: tsdocrsTree(result.docComment, comment) };
	} catch (error) {
		return { throws: error.message };
	}
}

for (const entry of corpus) {
	test(JSON.stringify(entry.comment), () => {
		assert.deepEqual(mine(entry), microsoft(entry));
	});
}

for (const entry of corpus) {
	test(`check ${JSON.stringify(entry.comment)}`, () => {
		const { messages, throws } = microsoft(entry);
		let checked;
		try {
			checked = tsdocrs.check(entry.comment, entry.config);
		} catch (error) {
			assert.deepEqual({ throws: error.message }, { throws });
			return;
		}
		assert.deepEqual(checked.messages, messages);
		for (const tag of checked.tags) {
			assert.equal(entry.comment.slice(tag.pos, tag.end), tag.tagName);
		}
	});
}

test("a tag's fields come through", () => {
	const { docComment } = tsdocrs.parse("/**\n * @param x - The {@link Foo | foo}.\n */");
	const block = docComment.nodes.find((node) => node.kind === "ParamCollection").nodes[0];
	assert.equal(block.parameterName, "x");
	const link = JSON.stringify(block);
	assert.match(link, /"linkText":"foo"/);
});

test("settings that can't be used throw", () => {
	assert.throws(
		() => tsdocrs.parse("/** */", { tagDefinitions: [{ tagName: "x", syntaxKind: "block" }] }),
		/tsdoc-config-invalid-tag-name/,
	);
});

test("offsets count UTF-16 code units", () => {
	const { messages } = tsdocrs.parse("/** 😀 @bogus */");
	assert.equal(messages[0].pos, "/** 😀 ".length);
	assert.equal(tsdocrs.check("/** 😀 @bogus */").tags[0].pos, "/** 😀 ".length);
});

test("emit writes each comment out as @microsoft/tsdoc does", () => {
	const benchmarked = ["small", "medium", "large"].flatMap((name) =>
		JSON.parse(readFileSync(new URL(`../../bench/corpus/${name}.json`, import.meta.url), "utf8")),
	);
	for (const comment of [...corpus.map((entry) => entry.comment), ...benchmarked]) {
		let expected;
		try {
			expected = new TSDocParser().parseString(comment).docComment.emitAsTsdoc();
		} catch (error) {
			assert.throws(() => tsdocrs.emit(comment), { message: error.message }, comment);
			continue;
		}
		assert.equal(tsdocrs.emit(comment), expected, comment);
	}
});

/** parse()'s tree under `node` with every excerpt's `content`, as a lazy tree's `toJSON()` has it. */
function withContent(node, comment) {
	const item = { ...node };
	if (item.excerptKind !== undefined) {
		item.content ??= comment.slice(item.pos, item.end);
	}
	if (item.nodes) {
		item.nodes = item.nodes.map((child) => withContent(child, comment));
	}
	return item;
}

/** What parse() gives for `entry`, every excerpt's `content` filled in, or its error. */
function parsed({ comment, config }) {
	try {
		const result = tsdocrs.parse(comment, config);
		return { ...result, docComment: withContent(result.docComment, comment) };
	} catch (error) {
		return { throws: error.message };
	}
}

/** A lazy result as plain objects. */
function plain(result) {
	return result instanceof Error
		? { throws: result.message }
		: { ...result, docComment: result.docComment.toJSON() };
}

for (const entry of corpus) {
	test(`parseTree ${JSON.stringify(entry.comment)}`, () => {
		let tree;
		try {
			tree = tsdocrs.parseTree(entry.comment, entry.config);
		} catch (error) {
			tree = error;
		}
		assert.deepEqual(plain(tree), parsed(entry));
	});
}

test("a lazy node's properties are parse()'s fields", () => {
	const compare = (lazy, json) => {
		for (const [key, value] of Object.entries(json)) {
			if (key !== "nodes") {
				assert.equal(lazy[key], value, key);
			}
		}
		assert.equal(lazy.nodes?.length, json.nodes?.length);
		json.nodes?.forEach((child, at) => compare(lazy.nodes[at], child));
	};
	for (const entry of corpus) {
		const json = parsed(entry);
		if (json.throws === undefined) {
			compare(tsdocrs.parseTree(entry.comment, entry.config).docComment, json.docComment);
		}
	}
});

test("parseMany gives each comment's parse, and an error in place of one that throws", () => {
	const entries = corpus.filter((entry) => !entry.config);
	const many = tsdocrs.parseMany(entries.map((entry) => entry.comment));
	assert.deepEqual(many.map(plain), entries.map(parsed));
	assert.ok(many.some((result) => result instanceof Error));
});
