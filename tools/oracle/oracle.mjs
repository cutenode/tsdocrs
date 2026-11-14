// Runs @microsoft/tsdoc, the parser tsdocrs ports, as a test oracle: for each comment it gives the
// node tree and messages tsdocrs's tests compare theirs with. `--serve` reads one JSON request a
// line on stdin ({ comment, config }) and writes one result a line; `--record FILE` fills in each
// entry of a corpus file. `config` is a tsdoc.json's keys, set up the way @microsoft/tsdoc-config
// does. Offsets are UTF-8 bytes, as Rust's strings index.
import { readFileSync, writeFileSync } from "node:fs";
import {
	DocErrorText,
	DocExcerpt,
	DocPlainText,
	TSDocConfiguration,
	TSDocParser,
} from "@microsoft/tsdoc";
import { TSDocConfigFile } from "@microsoft/tsdoc-config";

/** The tree under `node`, in the shape of TestHelpers.getDocNodeSnapshot, unescaped. */
function snapshot(node, context) {
	const item = { kind: node.kind };
	if (node instanceof DocExcerpt) {
		item.kind += ": " + node.excerptKind;
		item.excerpt = node.content.toString();
	}
	if (node instanceof DocPlainText && node.textExcerpt === undefined) {
		item.plainText = node.text;
	}
	if (node instanceof DocErrorText) {
		item.errorMessage = node.errorMessage;
		item.errorLocation = node.errorLocation.toString();
		if (node.errorLocation.startIndex > 0) {
			item.errorLocationPrecedingToken =
				context.tokens[node.errorLocation.startIndex - 1].toString();
		}
	}
	const children = node.getChildNodes();
	if (children.length > 0) {
		item.nodes = children.map((child) => snapshot(child, context));
	}
	return item;
}

/** What @microsoft/tsdoc makes of `request`, or what it throws. */
export function run({ comment, config }) {
	const configuration = new TSDocConfiguration();
	if (config) {
		// tsdoc-config reports some mistakes only once it sets up the parser, and throws on others.
		const file = TSDocConfigFile.loadFromObject({
			$schema: TSDocConfigFile.CURRENT_SCHEMA_URL,
			...config,
		});
		try {
			file.configureParser(configuration);
		} catch (error) {
			return { configError: error.message };
		}
		if (file.hasErrors) {
			return { configError: file.getErrorSummary() };
		}
	}
	// A range TSDoc ends inside a surrogate pair (a character it reports alone, like a stray one
	// before the comment) ends after the pair in Rust's, whose offsets can't split a character.
	const bytes = (pos) => {
		const inside =
			pos > 0 &&
			/[\uD800-\uDBFF]/.test(comment[pos - 1]) &&
			/[\uDC00-\uDFFF]/.test(comment[pos] ?? "");
		return Buffer.byteLength(comment.slice(0, inside ? pos + 1 : pos));
	};
	try {
		const context = new TSDocParser(configuration).parseString(comment);
		return {
			lines: context.lines.map((line) => line.toString()),
			messages: context.log.messages.map((m) => ({
				id: m.messageId,
				text: m.text,
				pos: bytes(m.textRange.pos),
				end: bytes(m.textRange.end),
			})),
			nodes: snapshot(context.docComment, context),
		};
	} catch (error) {
		return { throws: error.message };
	}
}

const [mode, file] = process.argv.slice(2);
if (mode === "--serve") {
	// One request a line. readline would also break lines at U+2028 and U+2029, which JSON leaves
	// as they are inside strings, so this splits at "\n" only.
	let buffered = "";
	process.stdin.setEncoding("utf8");
	for await (const chunk of process.stdin) {
		buffered += chunk;
		let end;
		while ((end = buffered.indexOf("\n")) !== -1) {
			const line = buffered.slice(0, end);
			buffered = buffered.slice(end + 1);
			process.stdout.write(JSON.stringify(run(JSON.parse(line))) + "\n");
		}
	}
} else if (mode === "--record") {
	const entries = JSON.parse(readFileSync(file, "utf8"));
	for (const entry of entries) {
		entry.expected = run(entry);
	}
	writeFileSync(file, JSON.stringify(entries, null, "\t") + "\n");
} else {
	process.stderr.write("usage: oracle.mjs --serve | --record FILE\n");
	process.exit(2);
}
