// The statistics the report gives for a set of timings, the same for criterion's and tinybench's.

/** The `p`th percentile (0 to 1) of `sorted`, interpolating between the two nearest values. */
export function percentile(sorted, p) {
	const at = (sorted.length - 1) * p;
	const below = Math.floor(at);
	const above = Math.min(below + 1, sorted.length - 1);
	return sorted[below] + (sorted[above] - sorted[below]) * (at - below);
}

/**
 * A summary of `samples`, each one iteration's time in nanoseconds: Tukey's fences (1.5 times the
 * interquartile range past the quartiles) leave out the outliers, which a scheduler or a garbage
 * collection makes, and the rest give the mean, the median, the 95th percentile and the spread.
 */
export function summarize(samples) {
	if (samples.length === 0) {
		throw new Error("no samples to summarize");
	}
	const sorted = [...samples].sort((a, b) => a - b);
	const [q1, q3] = [percentile(sorted, 0.25), percentile(sorted, 0.75)];
	const fence = 1.5 * (q3 - q1);
	const kept = sorted.filter((sample) => sample >= q1 - fence && sample <= q3 + fence);
	const mean = kept.reduce((total, sample) => total + sample, 0) / kept.length;
	const variance =
		kept.reduce((total, sample) => total + (sample - mean) ** 2, 0) / Math.max(kept.length - 1, 1);
	const sd = Math.sqrt(variance);
	return {
		samples: samples.length,
		outliers: samples.length - kept.length,
		mean,
		p50: percentile(kept, 0.5),
		p95: percentile(kept, 0.95),
		min: kept[0],
		max: kept[kept.length - 1],
		sd,
		cv: sd / mean,
	};
}

/** How much of a corpus of `comments` comments and `bytes` bytes one pass of `meanNs` gets through, per second. */
export function throughput(meanNs, comments, bytes) {
	const seconds = meanNs / 1e9;
	return { commentsPerSecond: comments / seconds, megabytesPerSecond: bytes / 1e6 / seconds };
}
