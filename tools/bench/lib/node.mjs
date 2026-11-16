// The Node side of the benchmarks: each implementation's parse, and parse and emit, of each
// corpus, timed with tinybench.
import { Bench } from "tinybench";

/**
 * Times each of `implementations` ({ name, parse, emit }, each taking one comment) on each of
 * `corpora`, and gives each run's samples, one iteration (a whole corpus) each, in nanoseconds.
 * `time` and `warmupTime` are each run's budgets in milliseconds.
 */
export async function run({ corpora, implementations, time, warmupTime }) {
	const results = [];
	for (const corpus of corpora) {
		for (const implementation of implementations) {
			for (const operation of ["parse", "emit"]) {
				const each = implementation[operation];
				const bench = new Bench({
					time,
					warmupTime,
					retainSamples: true,
					iterations: 10,
					warmupIterations: 5,
				});
				bench.add(operation, () => {
					for (const comment of corpus.comments) {
						try {
							each(comment);
						} catch {
							// Both parsers throw on a few inputs, the same ones; that's still their time.
						}
					}
				});
				await bench.run();
				const [task] = bench.tasks;
				const samples = task.result.latency.samples.map((ms) => ms * 1e6);
				results.push({
					corpus: corpus.name,
					implementation: implementation.name,
					operation,
					samples,
				});
			}
		}
	}
	return results;
}
