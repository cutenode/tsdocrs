// Writes the `/** */` comments of the given source files, in order, to a corpus file: a JSON array
// of strings. tools/bench/corpus.sh runs it.
import { readFileSync, writeFileSync } from "node:fs";
import { comments } from "./lib/corpus.mjs";

const [output, ...files] = process.argv.slice(2);
const found = files.flatMap((file) => comments(readFileSync(file, "utf8")));
writeFileSync(output, JSON.stringify(found, null, "\t") + "\n");
process.stdout.write(`${output}: ${found.length} comments from ${files.length} files\n`);
