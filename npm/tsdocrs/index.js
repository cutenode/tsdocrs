// The tsdocrs npm package: what the WebAssembly module exports, and parseTree and parseMany, which
// read parseTrees's numbers through lazy nodes, so a caller pays only for the nodes it reads.
"use strict";

const wasm = require("./pkg/tsdocrs.js");

// The layout src/tree.rs writes: RECORD numbers per node, four per field.
const RECORD = 6;
const NONE = 0xffffffff;
const STRING = 0;
const SLICE = 1;

const TREE = Symbol("tree");
const AT = Symbol("at");
const CHILDREN = Symbol("children");

/** A node, read from its parse's buffers when its properties are. */
class LazyNode {
	constructor(tree, index) {
		this[TREE] = tree;
		this[AT] = index * RECORD;
		// Set here, not when first read, so every node keeps one shape, which V8 reads faster.
		this[CHILDREN] = null;
	}

	get kind() {
		return this[TREE].strings[this[TREE].nodes[this[AT]]];
	}

	get nodes() {
		if (this[CHILDREN] === null) {
			const tree = this[TREE];
			let child = tree.nodes[this[AT] + 3];
			this[CHILDREN] = child === NONE ? undefined : [];
			for (; child !== NONE; child = tree.nodes[child * RECORD + 4]) {
				this[CHILDREN].push(new LazyNode(tree, child));
			}
		}
		return this[CHILDREN];
	}

	get pos() {
		return isExcerpt(this) ? this[TREE].nodes[this[AT] + 1] : undefined;
	}

	get end() {
		return isExcerpt(this) ? this[TREE].nodes[this[AT] + 2] : undefined;
	}

	/** An excerpt's text: its own when it runs across lines, else the comment's from pos to end. */
	get content() {
		if (!isExcerpt(this)) {
			return undefined;
		}
		return field(this, "content") ?? this[TREE].text.slice(this.pos, this.end);
	}

	/** It and the nodes under it as plain objects, every field read. */
	toJSON() {
		const json = { kind: this.kind };
		forEachField(this, (name, value) => {
			json[name] = value;
		});
		if (isExcerpt(this)) {
			Object.assign(json, { content: this.content, pos: this.pos, end: this.end });
		}
		if (this.nodes) {
			json.nodes = this.nodes.map((node) => node.toJSON());
		}
		return json;
	}
}

/** Whether `node` is an excerpt, by its kind's index in the strings, found once per parse. */
function isExcerpt(node) {
	const tree = node[TREE];
	tree.excerpt ??= tree.strings.indexOf("Excerpt");
	return tree.nodes[node[AT]] === tree.excerpt;
}

/** Calls `f` with the name and value of each of `node`'s fields. */
function forEachField(node, f) {
	const { fields, strings, text } = node[TREE];
	const at = node[TREE].nodes[node[AT] + 5];
	for (let field = at + 1; field < at + 1 + 4 * fields[at]; field += 4) {
		const how = fields[field + 1];
		const a = fields[field + 2];
		f(
			strings[fields[field]],
			how === STRING ? strings[a] : how === SLICE ? text.slice(a, fields[field + 3]) : a === 1,
		);
	}
}

/** The value of `node`'s field `name`, or undefined when its kind has none. */
function field(node, name) {
	let found;
	forEachField(node, (each, value) => {
		if (each === name) {
			found = value;
		}
	});
	return found;
}

for (const name of [
	"excerptKind",
	"tagName",
	"text",
	"messageId",
	"errorMessage",
	"encodedText",
	"decodedText",
	"code",
	"language",
	"name",
	"value",
	"selfClosingTag",
	"tagContent",
	"urlDestination",
	"linkText",
	"parameterName",
	"packageName",
	"importPath",
	"identifier",
	"hasQuotes",
	"hasDot",
	"selector",
	"selectorKind",
]) {
	Object.defineProperty(LazyNode.prototype, name, {
		get() {
			return field(this, name);
		},
	});
}

/** A comment of parseTrees's result as parse() gives it, with a lazy tree. */
function result(trees, comment, text) {
	const tree = {
		nodes: trees.nodes,
		fields: trees.fields,
		strings: trees.strings,
		text,
		excerpt: undefined,
	};
	return {
		lines: comment.lines,
		messages: comment.messages,
		docComment: new LazyNode(tree, comment.root),
	};
}

/** Parses `text` as parse() does, into a lazy tree. */
function parseTree(text, settings) {
	const trees = wasm.parseTrees([text], settings);
	const [comment] = trees.comments;
	if (comment.error !== undefined) {
		throw new Error(comment.error);
	}
	return result(trees, comment, text);
}

/** Parses each of `texts` as parseTree does, in one call with one configuration. */
function parseMany(texts, settings) {
	const trees = wasm.parseTrees(texts, settings);
	return trees.comments.map((comment, at) =>
		comment.error === undefined ? result(trees, comment, texts[at]) : new Error(comment.error),
	);
}

module.exports = { ...wasm, parseTree, parseMany };
