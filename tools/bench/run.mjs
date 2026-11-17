// `npm run bench`: tsdocrs against @microsoft/tsdoc on bench/corpus/, written to
// target/bench/report.md and report.json. The Rust side is criterion (benches/parse.rs); the Node
// side, tsdocrs's npm package and @microsoft/tsdoc in the same process, is tinybench. `--quick`
// makes every run short, to check the harness rather than to measure. The report also says how
// tsdocrs does on TSDoc's own tests (tests/upstream): the suite pinned there, or with `--live`,
// microsoft/tsdoc's default branch as it is now.
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { delimiter } from "node:path";
import { fileURLToPath } from "node:url";
import { TSDocParser } from "@microsoft/tsdoc";
import { accuracy, outcomes } from "./lib/accuracy.mjs";
import { load } from "./lib/corpus.mjs";
import { samples } from "./lib/criterion.mjs";
import { environment, output } from "./lib/environment.mjs";
import { run } from "./lib/node.mjs";
import { BASELINE, build, markdown } from "./lib/report.mjs";
import { isPinned, wasmBindgenPin } from "./lib/tools.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const quick = process.argv.includes("--quick");
const live = process.argv.includes("--live");
const exec = (command, args, env = process.env) =>
	execFileSync(command, args, { cwd: root, stdio: "inherit", env });

// The live suite goes to target/upstream, which isn't checked in: re-vendoring tests/upstream is
// upstream.yml's job, as a pull request. If vendor.sh fails, as when TSDoc's own suite fails at
// that commit, the report says so and the benchmarks still run. It runs first, so a harness that
// doesn't build stops the run before the benchmarks take their time.
const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const pinned = readJson(`${root}tests/upstream/manifest.json`);
function measureAccuracy() {
	let dir = `${root}tests/upstream`;
	if (live) {
		const tsdoc = "https://github.com/microsoft/tsdoc.git";
		const head = execFileSync("git", ["ls-remote", tsdoc, "HEAD"], { encoding: "utf8" });
		const commit = head.split("\t")[0];
		if (commit !== pinned.commit) {
			dir = `${root}target/upstream`;
			try {
				exec("tools/upstream/vendor.sh", [commit, dir]);
			} catch {
				return { commit, error: "tools/upstream/vendor.sh failed, with its output in the log." };
			}
		}
	}
	// libtest's JSON on stdout. It exits 101 when a test fails, which outcomes() reads; a run that
	// didn't finish, as when it didn't build, throws there.
	const harness = spawnSync(
		"cargo",
		["test", "--locked", "--test", "upstream", "--", "--format", "json"],
		{
			cwd: root,
			env: { ...process.env, UPSTREAM_DIR: dir },
			encoding: "utf8",
			stdio: ["ignore", "pipe", "inherit"],
			maxBuffer: 2 ** 26,
		},
	);
	return accuracy({
		suite: readJson(`${dir}/cases.json`),
		manifest: readJson(`${dir}/manifest.json`),
		pinned,
		allowlist: readJson(`${root}tests/upstream/allowlist.json`),
		outcomes: outcomes(harness.stdout),
	});
}

const upstream = measureAccuracy();

// The npm package, built with the wasm-bindgen CLI it pins, installed under target/ if need be.
const tools = `${root}target/bench-tools`;
const env = { ...process.env, PATH: `${tools}/bin${delimiter}${process.env.PATH}` };
const pin = wasmBindgenPin(readFileSync(`${root}npm/tsdocrs/Cargo.toml`, "utf8"));
if (!isPinned(output("wasm-bindgen", ["--version"], root, env), pin)) {
	exec("cargo", ["install", "--locked", "--root", tools, "wasm-bindgen-cli", "--version", pin]);
}
exec("tools/build-npm.sh", [], env);
exec("cargo", ["bench", "--locked", "--bench", "parse", "--", ...(quick ? ["--quick"] : [])]);

const corpora = ["small", "medium", "large"].map((name) => load(`${root}bench/corpus`, name));
const require = createRequire(import.meta.url);
const tsdocrs = require("../../npm/tsdocrs/pkg/tsdocrs.js");
const parser = new TSDocParser();
const implementations = [
	{
		name: BASELINE,
		parse: (comment) => parser.parseString(comment),
		emit: (comment) => parser.parseString(comment).docComment.emitAsTsdoc(),
	},
	{
		name: "tsdocrs (npm)",
		parse: (comment) => tsdocrs.parse(comment),
		emit: (comment) => tsdocrs.emit(comment),
	},
];
const runs = await run({
	corpora,
	implementations,
	time: quick ? 100 : 5000,
	warmupTime: quick ? 20 : 1000,
});
for (const corpus of corpora) {
	for (const operation of ["parse", "emit"]) {
		const native = samples(`${root}target/criterion`, corpus.name, operation);
		runs.push({
			corpus: corpus.name,
			implementation: "tsdocrs (Rust)",
			operation,
			samples: native,
		});
	}
}

const report = build({
	corpora,
	runs,
	environment: environment(root),
	accuracy: upstream,
});
mkdirSync(`${root}target/bench`, { recursive: true });
writeFileSync(`${root}target/bench/report.json`, JSON.stringify(report, null, "\t") + "\n");
writeFileSync(`${root}target/bench/report.md`, markdown(report));
process.stdout.write(`\n${markdown(report)}\nWritten to target/bench/report.md and report.json.\n`);
