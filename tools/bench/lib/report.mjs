// The report: each run's statistics and throughput, the speedups over @microsoft/tsdoc, what it
// ran on, and how tsdocrs does on TSDoc's own tests, as JSON and as Markdown.
import { accuracyMarkdown } from "./accuracy.mjs";
import { summarize, throughput } from "./stats.mjs";

/** The implementation the others are compared with. */
export const BASELINE = "@microsoft/tsdoc";

const OPERATIONS = { parse: "parse", emit: "parse + emit" };

/**
 * The report of `runs` ({ corpus, implementation, operation, samples }) over `corpora`
 * ({ name, comments, bytes }), with `environment` (versions and the machine) and `accuracy`
 * (accuracy.mjs's) as given.
 */
export function build({ corpora, runs, environment, accuracy }) {
	const results = runs.map(({ corpus, implementation, operation, samples }) => {
		const { comments, bytes } = corpora.find((c) => c.name === corpus);
		const stats = summarize(samples);
		return {
			corpus,
			implementation,
			operation,
			...stats,
			...throughput(stats.mean, comments.length, bytes),
		};
	});
	for (const result of results) {
		const baseline = results.find(
			(other) =>
				other.implementation === BASELINE &&
				other.corpus === result.corpus &&
				other.operation === result.operation,
		);
		result.speedup = baseline ? baseline.mean / result.mean : null;
	}
	const sizes = corpora.map(({ name, comments, bytes }) => ({
		name,
		comments: comments.length,
		bytes,
	}));
	return { environment, corpora: sizes, results, accuracy };
}

const ms = (ns) => (ns / 1e6).toFixed(3);
const fixed = (n, digits) =>
	n.toLocaleString("en-US", { maximumFractionDigits: digits, minimumFractionDigits: digits });

/** `report` as Markdown: what it ran on, a table for each corpus, then the accuracy. */
export function markdown(report) {
	const { environment, corpora, results, accuracy } = report;
	const lines = ["# tsdocrs benchmarks", "", "| | |", "| --- | --- |"];
	for (const [key, value] of Object.entries(environment)) {
		lines.push(`| ${key} | ${value} |`);
	}
	for (const corpus of corpora) {
		lines.push(
			"",
			`## ${corpus.name}: ${corpus.comments} comments, ${fixed(corpus.bytes / 1e6, 2)} MB`,
			"",
			"| operation | implementation | mean ms | p50 ms | p95 ms | ± sd | samples (outliers) | comments/s | MB/s | speedup |",
			"| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
		);
		for (const r of results.filter((r) => r.corpus === corpus.name)) {
			const speedup = r.speedup === null ? "" : `${fixed(r.speedup, 2)}x`;
			lines.push(
				`| ${OPERATIONS[r.operation]} | ${r.implementation} | ${ms(r.mean)} | ${ms(r.p50)} | ${ms(r.p95)} | ${fixed(r.cv * 100, 1)}% | ${r.samples} (${r.outliers}) | ${fixed(r.commentsPerSecond, 0)} | ${fixed(r.megabytesPerSecond, 2)} | ${speedup} |`,
			);
		}
	}
	lines.push(
		"",
		`Each sample is one pass over the whole corpus. Outliers past Tukey's fences (1.5 times the interquartile range) are left out of the statistics; ± sd is the standard deviation as a share of the mean. The speedup is ${BASELINE}'s mean time over the implementation's.`,
		...accuracyMarkdown(accuracy),
	);
	return lines.join("\n") + "\n";
}
