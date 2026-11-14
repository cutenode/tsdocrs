// The npm package against @microsoft/tsdoc, in the same Node process: for each comment of
// tests/fixtures/corpus.json, the same lines, messages (ids, texts and UTF-16 ranges) and tree,
// or the same error. Run tools/build-npm.sh first.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { test } from "node:test";
import { DocExcerpt, TSDocConfiguration, TSDocParser } from "@microsoft/tsdoc";
import { TSDocConfigFile } from "@microsoft/tsdoc-config";

const require = createRequire(import.meta.url);
const tsdocrs = require("./pkg/tsdocrs.js");
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

/** The same of tsdocrs's tree, leaving out the fields TSDoc's snapshot has no place for. */
function tsdocrsTree(node) {
	const item = { kind: node.kind };
	for (const key of ["excerptKind", "content", "pos", "end"]) {
		if (key in node) {
			item[key] = node[key];
		}
	}
	if (node.nodes) {
		item.nodes = node.nodes.map(tsdocrsTree);
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
		return { ...result, docComment: tsdocrsTree(result.docComment) };
	} catch (error) {
		return { throws: error.message };
	}
}

for (const entry of corpus) {
	test(JSON.stringify(entry.comment), () => {
		assert.deepEqual(mine(entry), microsoft(entry));
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
});
