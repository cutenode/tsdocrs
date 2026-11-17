import type { ParseResult, TSDocConfigFileJson } from "./pkg/tsdocrs.js";

export * from "./pkg/tsdocrs.js";

/**
 * Parses `text` as `parse` does, into a tree whose nodes are read from the parse's buffers as their
 * properties are, so reading part of the tree costs only that part. An excerpt always has its
 * `content`, and `toJSON()` gives a node and the nodes under it as plain objects. It throws where
 * `parse` throws.
 */
export function parseTree(text: string, settings?: TSDocConfigFileJson): ParseResult;

/**
 * Parses each of `texts` as `parseTree` does, in one call with one configuration. A comment
 * `@microsoft/tsdoc` throws on is an `Error` with its message, in its place.
 */
export function parseMany(texts: string[], settings?: TSDocConfigFileJson): (ParseResult | Error)[];
