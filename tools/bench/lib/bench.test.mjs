// The benchmark harness's statistics, report and helpers, without timing anything real.
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import os from "node:os";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { accuracy, accuracyMarkdown, outcomes, reason } from "./accuracy.mjs";
import { comments, load } from "./corpus.mjs";
import { samples } from "./criterion.mjs";
import { cargoLockVersion, environment, output } from "./environment.mjs";
import { run } from "./node.mjs";
import { BASELINE, build, markdown } from "./report.mjs";
import { percentile, summarize, throughput } from "./stats.mjs";
import { isPinned, wasmBindgenPin } from "./tools.mjs";

const root = fileURLToPath(new URL("../../..", import.meta.url));
const scratch = () => mkdtempSync(`${os.tmpdir()}/tsdocrs-bench-`);

test("percentiles interpolate between the nearest samples", () => {
	assert.equal(percentile([1, 2, 3, 4], 0.5), 2.5);
	assert.equal(percentile([1, 2, 3, 4], 0), 1);
	assert.equal(percentile([1, 2, 3, 4], 1), 4);
	assert.equal(percentile([7], 0.95), 7);
});

test("a summary leaves out the outliers past Tukey's fences", () => {
	const summary = summarize([10, 11, 9, 10, 12, 10, 1000]);
	assert.equal(summary.samples, 7);
	assert.equal(summary.outliers, 1);
	assert.equal(summary.max, 12);
	assert.equal(summary.mean, 62 / 6);
	assert.equal(summary.p50, 10);
	assert.ok(summary.p95 > summary.p50 && summary.p95 <= 12);
	assert.ok(summary.cv > 0 && summary.cv < 0.2);
	assert.equal(summarize([5]).sd, 0);
	assert.throws(() => summarize([]), /no samples/);
});

test("throughput is per second", () => {
	assert.deepEqual(throughput(5e8, 10, 2e6), { commentsPerSecond: 20, megabytesPerSecond: 4 });
});

test("comments are the /** */ ones, with LF line ends", () => {
	const source = "/** a */ x /* not */ /**/ y /**\r\n * b\r\n */ /** */";
	assert.deepEqual(comments(source), ["/** a */", "/**\n * b\n */", "/** */"]);
	assert.deepEqual(comments("none"), []);
	const dir = scratch();
	writeFileSync(`${dir}/c.json`, JSON.stringify(["/** é */", "/** */"]));
	assert.deepEqual(load(dir, "c"), { name: "c", comments: ["/** é */", "/** */"], bytes: 15 });
});

test("criterion's samples are each batch's time per iteration", () => {
	const dir = scratch();
	mkdirSync(`${dir}/small/parse/new`, { recursive: true });
	writeFileSync(
		`${dir}/small/parse/new/sample.json`,
		JSON.stringify({ iters: [1, 2, 4], times: [10, 30, 80] }),
	);
	assert.deepEqual(samples(dir, "small", "parse"), [10, 15, 20]);
});

test("the Node side times each implementation's parse and emit of each corpus", async () => {
	const seen = [];
	const implementations = [
		{ name: "a", parse: (c) => seen.push(`a parse ${c}`), emit: () => {} },
		{
			name: "b",
			parse: () => {},
			emit: () => {
				throw new Error("thrown errors are part of the time");
			},
		},
	];
	const corpora = [{ name: "tiny", comments: ["/** x */"] }];
	const runs = await run({ corpora, implementations, time: 5, warmupTime: 1 });
	assert.deepEqual(
		runs.map((r) => `${r.corpus} ${r.implementation} ${r.operation}`),
		["tiny a parse", "tiny a emit", "tiny b parse", "tiny b emit"],
	);
	assert.ok(runs.every((r) => r.samples.length >= 10 && r.samples.every((ns) => ns >= 0)));
	assert.ok(seen.includes("a parse /** x */"));
});

test("the report compares each implementation with @microsoft/tsdoc", () => {
	const corpora = [{ name: "c", comments: ["/** a */", "/** b */"], bytes: 2e6 }];
	const runs = [
		{ corpus: "c", implementation: BASELINE, operation: "parse", samples: [4e9, 4e9] },
		{ corpus: "c", implementation: "fast", operation: "parse", samples: [1e9, 1e9] },
		{ corpus: "c", implementation: "fast", operation: "emit", samples: [2e9] },
	];
	const report = build({
		corpora,
		runs,
		environment: { node: "v24" },
		accuracy: { commit: "f".repeat(40), error: "it broke" },
	});
	assert.deepEqual(report.corpora, [{ name: "c", comments: 2, bytes: 2e6 }]);
	const [baseline, fast, emit] = report.results;
	assert.equal(baseline.speedup, 1);
	assert.equal(fast.speedup, 4);
	assert.equal(emit.speedup, null);
	assert.equal(fast.commentsPerSecond, 2);
	assert.equal(fast.megabytesPerSecond, 2);
	const text = markdown(report);
	assert.match(text, /\| node \| v24 \|/);
	assert.match(text, /## c: 2 comments, 2\.00 MB/);
	assert.match(
		text,
		/\| parse \| fast \| 1000\.000 \| 1000\.000 \| 1000\.000 \| 0\.0% \| 2 \(0\) \| 2 \| 2\.00 \| 4\.00x \|/,
	);
	assert.match(text, /\| parse \+ emit \| fast \| 2000\.000 .* \|  \|$/m);
	assert.equal(report.accuracy.error, "it broke");
	assert.match(
		text,
		/## Accuracy on TSDoc's own tests\n\n\*\*The suite at microsoft\/tsdoc@fffffff didn't run:\*\* it broke\n$/,
	);
});

// libtest-mimic's JSON lines, as the upstream harness prints them with `-- --format json`.
const event = (fields) => JSON.stringify({ type: "test", ...fields });
const finished = '{ "type": "suite", "event": "failed", "passed": 2, "failed": 2 }';

test("the harness's outcomes are its passing tests and its failures' messages", () => {
	const jsonl = [
		'{ "type": "suite", "event": "started", "test_count": 4 }',
		event({ event: "started", name: "a.ts::one" }),
		event({ event: "ok", name: "a.ts::one" }),
		event({ event: "failed", name: "a.ts::two", stdout: 'Error: "key: \\"x\\" differs\nmore"\n' }),
		event({ event: "failed", name: "a.ts::three" }),
		finished,
		"",
	].join("\r\n");
	const { passed, failed } = outcomes(jsonl);
	assert.deepEqual([...passed], ["a.ts::one"]);
	assert.deepEqual(
		[...failed],
		[
			["a.ts::two", 'key: \\"x\\" differs\nmore'],
			["a.ts::three", ""],
		],
	);
	// A run that didn't finish, like a build that failed, has no result to count.
	assert.throws(() => outcomes(event({ event: "ok", name: "a" })), /didn't finish/);
	assert.throws(() => outcomes(""), /didn't finish/);
});

test("a failure's reason is its message's first line, cut short", () => {
	assert.equal(reason("\n  first  \nsecond"), "first");
	assert.equal(reason(""), "no message");
	assert.equal(reason("x".repeat(161)), `${"x".repeat(159)}…`);
	assert.equal(reason("x".repeat(160)), "x".repeat(160));
});

test("accuracy counts the tests and snapshots that pass, leaving out the allowlisted", () => {
	const snapshot = { type: "snapshot", key: "k", helper: null, expected: "" };
	const suite = {
		files: [
			{
				path: "a.ts",
				tests: [
					{ name: "one", events: [{ type: "parse" }, snapshot, { type: "inlineSnapshot" }] },
					{ name: "two", events: [snapshot] },
					{ name: "allowed", events: [snapshot, { type: "assertion" }] },
				],
			},
			{ path: "b.ts", tests: [{ name: "one", events: [snapshot] }] },
		],
	};
	const live = "a".repeat(40);
	const old = "b".repeat(40);
	const result = accuracy({
		suite,
		manifest: { commit: live, date: "2026-11-10T12:00:00Z" },
		pinned: { commit: old, date: "2026-09-29T11:29:12-07:00" },
		allowlist: [{ file: "a.ts", name: "allowed", reason: "why" }],
		outcomes: {
			passed: new Set(["a.ts::one", "a.ts::allowed", "b.ts::one"]),
			failed: new Map([["a.ts::two", "key: differs\n- a\n+ b"]]),
		},
	});
	assert.deepEqual(result, {
		commit: live,
		date: "2026-11-10T12:00:00Z",
		pinned: { commit: old, date: "2026-09-29T11:29:12-07:00" },
		tests: { passed: 2, total: 4 },
		snapshots: { matched: 3, total: 5 },
		allowlist: 1,
		failures: [{ name: "a.ts::two", reason: "key: differs" }],
	});
	const text = accuracyMarkdown(result).join("\n");
	assert.match(
		text,
		/\| suite \| microsoft\/tsdoc@\[aaaaaaa\]\(https:\/\/github\.com\/microsoft\/tsdoc\/commit\/a{40}\), 2026-11-10, newer than the pinned \[bbbbbbb\]\([^)]*\) of 2026-09-29, by 42 days \(/,
	);
	assert.match(
		text,
		/\(\[changes\]\(https:\/\/github\.com\/microsoft\/tsdoc\/compare\/b{40}\.\.\.a{40}\)\) \|/,
	);
	assert.match(text, /\| tests passed \| 2 of 4 \|/);
	assert.match(text, /\| snapshots matched \| 3 of 5 \|/);
	assert.match(text, /\| allowlist \| 1 test expected to fail \|/);
	assert.match(text, /\*\*1 failing:\*\*\n\n- `a\.ts::two`: `key: differs`$/);
});

test("the accuracy of the pinned suite says so, and a clean run lists no failures", () => {
	const commit = "c".repeat(40);
	const lines = accuracyMarkdown({
		commit,
		date: "2026-09-29T11:29:12-07:00",
		pinned: { commit, date: "2026-09-29T11:29:12-07:00" },
		tests: { passed: 1, total: 1 },
		snapshots: { matched: 0, total: 0 },
		allowlist: 0,
		failures: [],
	});
	assert.match(lines.join("\n"), /2026-09-29, the commit tests\/upstream is pinned to \|/);
	assert.match(lines.join("\n"), /\| allowlist \| 0 tests expected to fail \|$/);
	assert.match(
		accuracyMarkdown({
			commit,
			date: "2026-09-29T12:13:46-07:00",
			pinned: { commit: "d".repeat(40), date: "2026-09-29T11:29:12-07:00" },
			tests: { passed: 0, total: 1 },
			snapshots: { matched: 0, total: 0 },
			allowlist: 2,
			failures: [{ name: "a `b` c", reason: "<b>" }],
		}).join("\n"),
		/, newer than the pinned \[ddddddd\]\([^)]*\) of 2026-09-29 \(\[changes\][^]*\| 2 tests expected to fail \|[^]*- `` a `b` c ``: `<b>`$/,
	);
});

test("the environment names the versions and the machine", () => {
	assert.equal(output("node", ["--version"], root), process.version);
	assert.match(output("a-command-that-isnt-there", [], root), /^unavailable/);
	assert.match(output("node", ["-e", "process.exit(3)"], root), /^unavailable \(3\)/);
	const lock = '[[package]]\nname = "criterion"\nversion = "0.8.2"\n';
	assert.equal(cargoLockVersion(lock, "criterion"), "0.8.2");
	assert.equal(cargoLockVersion(lock, "nope"), "unknown");
	// A command's output and a file checked out on Windows can end their lines in CRLF.
	assert.equal(cargoLockVersion(lock.replaceAll("\n", "\r\n"), "criterion"), "0.8.2");
	assert.equal(output("node", ["-e", "process.stdout.write('a\\r\\nb')"], root), "a");
	const env = environment(root);
	assert.equal(env.node, process.version);
	assert.match(env["@microsoft/tsdoc"], /^\d+\.\d+\.\d+$/);
	assert.match(env.tsdocrs, /^\d+\.\d+\.\d+$/);
	assert.match(env["wasm-bindgen"], /^\d+\.\d+\.\d+$/);
	assert.match(env.cpu, /cores$/);
	const unknown = { ...os, cpus: () => [], totalmem: () => 2 ** 31 };
	assert.equal(environment(root, unknown).cpu, "unknown, 0 cores");
	assert.equal(environment(root, unknown).memory, "2 GiB");
});

test("the wasm-bindgen CLI has to be the version npm/tsdocrs pins", () => {
	assert.equal(wasmBindgenPin('a = "1"\nwasm-bindgen = "=0.2.130"\n'), "0.2.130");
	assert.equal(wasmBindgenPin('a = "1"\r\nwasm-bindgen = "=0.2.130"\r\n'), "0.2.130");
	assert.throws(() => wasmBindgenPin('wasm-bindgen = "0.2"\n'), /pin wasm-bindgen exactly/);
	assert.ok(isPinned("wasm-bindgen 0.2.130\n", "0.2.130"));
	assert.ok(!isPinned("wasm-bindgen 0.2.129", "0.2.130"));
	assert.ok(!isPinned(undefined, "0.2.130"));
});
