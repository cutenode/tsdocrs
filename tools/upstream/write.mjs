// Turns recorder.cjs's records of an upstream Jest run into tests/upstream/cases.json and
// manifest.json, after checking them against Jest's own counts.
// Usage: node tools/upstream/write.mjs CHECKOUT RECORDS RESULTS COMMIT DATE
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";

const [checkout, records, results, commit, date] = process.argv.slice(2);
const out = new URL("../../tests/upstream/", import.meta.url);
const require = createRequire(import.meta.url);
const jest = JSON.parse(fs.readFileSync(results, "utf8"));

const files = fs
	.readdirSync(records)
	.map((name) => JSON.parse(fs.readFileSync(path.join(records, name), "utf8")))
	.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));

let tests = 0;
let snapshots = 0;
for (const file of files) {
	tests += file.tests.length;
	// A .snap file is CommonJS: `exports[key] = value`, as Jest itself evaluates it.
	const snapPath = path.join(
		checkout,
		path.dirname(file.path),
		"__snapshots__",
		`${path.basename(file.path)}.snap`,
	);
	const snap = fs.existsSync(snapPath) ? require(snapPath) : {};
	for (const event of file.tests.flatMap((test) => test.events)) {
		if (event.type === "inlineSnapshot") snapshots++;
		if (event.type !== "snapshot") continue;
		snapshots++;
		if (!Object.hasOwn(snap, event.key))
			throw new Error(`${file.path}: no snapshot ${JSON.stringify(event.key)}`);
		event.expected = snap[event.key];
	}
}

// Every test and snapshot Jest ran should have a record.
const counts = [
	["test files", files.length, jest.numPassedTestSuites],
	["tests", tests, jest.numPassedTests],
	["snapshots", snapshots, jest.snapshot.matched],
];
for (const [what, recorded, ran] of counts) {
	if (recorded !== ran) throw new Error(`recorded ${recorded} ${what}, but Jest ran ${ran}`);
}

const json = (value) => `${JSON.stringify(value, null, "\t")}\n`;
fs.writeFileSync(new URL("cases.json", out), json({ files }));
fs.writeFileSync(
	new URL("manifest.json", out),
	json({ repository: "microsoft/tsdoc", commit, date, files: files.length, tests, snapshots }),
);
