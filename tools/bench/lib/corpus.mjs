// The benchmark corpora: doc comments, and what reading them in bench/corpus/ gives.
import { readFileSync } from "node:fs";

/** The `/** *\/` comments in `source`, in order, with CRLF line ends made LF. */
export function comments(source) {
	return (source.replace(/\r\n/g, "\n").match(/\/\*\*(?!\/)[\s\S]*?\*\//g) ?? []).filter(
		(c) => c !== "/**/",
	);
}

/** The corpus named `name` in `directory`: its comments, and their total size in UTF-8 bytes. */
export function load(directory, name) {
	const items = JSON.parse(readFileSync(`${directory}/${name}.json`, "utf8"));
	const bytes = items.reduce((total, comment) => total + Buffer.byteLength(comment), 0);
	return { name, comments: items, bytes };
}
