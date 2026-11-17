// How tsdocrs does on TSDoc's own Jest suite: the upstream harness (tests/upstream/main.rs) run on
// a vendored suite, the pinned one in tests/upstream or a live one tools/upstream/vendor.sh wrote
// elsewhere, counted for the report.

const REPOSITORY = "https://github.com/microsoft/tsdoc";
const DAY = 24 * 60 * 60 * 1000;

/**
 * Each test's outcome from the harness's libtest JSON lines (`-- --format json`): passing names,
 * and failing names with their message. Throws when the run didn't finish, as when it didn't build.
 */
export function outcomes(jsonl) {
	const events = jsonl
		.split(/\r?\n/)
		.filter((line) => line.startsWith("{"))
		.map((line) => JSON.parse(line));
	const isFinished = events.some(
		(e) => e.type === "suite" && (e.event === "ok" || e.event === "failed"),
	);
	if (!isFinished) throw new Error("the upstream harness didn't finish a run");
	const passed = new Set();
	const failed = new Map();
	for (const e of events.filter((e) => e.type === "test")) {
		if (e.event === "ok") passed.add(e.name);
		// libtest-mimic writes a failure's message as `Error: "<message>"\n`.
		if (e.event === "failed") failed.set(e.name, e.stdout?.replace(/^Error: "|"\n$/g, "") ?? "");
	}
	return { passed, failed };
}

/** The first line of a failure's message, short enough for a list. */
export function reason(message) {
	const line =
		message
			.split("\n")
			.find((l) => l.trim() !== "")
			?.trim() ?? "no message";
	return line.length > 160 ? `${line.slice(0, 159)}…` : line;
}

/**
 * The accuracy section of the report: `suite` (cases.json) and `manifest` (manifest.json) as
 * vendored, the `pinned` manifest from tests/upstream, `allowlist` (allowlist.json) and the
 * harness's `outcomes`. A test passes when the harness passed it and the allowlist doesn't expect
 * it to fail. Its snapshots match when it passes.
 */
export function accuracy({ suite, manifest, pinned, allowlist, outcomes }) {
	let tests = 0;
	let snapshots = 0;
	let passed = 0;
	let matched = 0;
	for (const file of suite.files) {
		for (const test of file.tests) {
			const name = `${file.path}::${test.name}`;
			const count = test.events.filter(
				(e) => e.type === "snapshot" || e.type === "inlineSnapshot",
			).length;
			const isAllowed = allowlist.some((a) => a.file === file.path && a.name === test.name);
			tests++;
			snapshots += count;
			if (!outcomes.passed.has(name) || isAllowed) continue;
			passed++;
			matched += count;
		}
	}
	return {
		commit: manifest.commit,
		date: manifest.date,
		pinned: { commit: pinned.commit, date: pinned.date },
		tests: { passed, total: tests },
		snapshots: { matched, total: snapshots },
		allowlist: allowlist.length,
		failures: [...outcomes.failed].map(([name, message]) => ({ name, reason: reason(message) })),
	};
}

const plural = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
// Test names and messages hold backticks and HTML, which a code span keeps as they are.
const code = (text) => (text.includes("`") ? `\`\` ${text} \`\`` : `\`${text}\``);
const short = (commit) => commit.slice(0, 7);
const link = (commit) => `[${short(commit)}](${REPOSITORY}/commit/${commit})`;
const day = (date) => date.slice(0, 10);

/** Where the suite is from: its commit and date, and how far it is from the pinned one. */
function source({ commit, date, pinned }) {
	const at = `microsoft/tsdoc@${link(commit)}, ${day(date)}`;
	if (commit === pinned.commit) return `${at}, the commit tests/upstream is pinned to`;
	const days = Math.round((Date.parse(date) - Date.parse(pinned.date)) / DAY);
	const compare = `${REPOSITORY}/compare/${pinned.commit}...${commit}`;
	const by = days > 0 ? `, by ${plural(days, "day")}` : "";
	return `${at}, newer than the pinned ${link(pinned.commit)} of ${day(pinned.date)}${by} ([changes](${compare}))`;
}

/** `accuracy` as the report's Markdown lines: a table, then the failing tests, or the error. */
export function accuracyMarkdown(accuracy) {
	const lines = ["", "## Accuracy on TSDoc's own tests", ""];
	if (accuracy.error) {
		lines.push(
			`**The suite at microsoft/tsdoc@${short(accuracy.commit)} didn't run:** ${accuracy.error}`,
		);
		return lines;
	}
	const { tests, snapshots, allowlist, failures } = accuracy;
	lines.push(
		"| | |",
		"| --- | --- |",
		`| suite | ${source(accuracy)} |`,
		`| tests passed | ${tests.passed} of ${tests.total} |`,
		`| snapshots matched | ${snapshots.matched} of ${snapshots.total} |`,
		`| allowlist | ${plural(allowlist, "test")} expected to fail |`,
	);
	if (failures.length === 0) return lines;
	lines.push("", `**${failures.length} failing:**`, "");
	for (const { name, reason } of failures) lines.push(`- ${code(name)}: ${code(reason)}`);
	return lines;
}
