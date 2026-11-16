// criterion's samples, from what `cargo bench` leaves in target/criterion/.
import { readFileSync } from "node:fs";

/**
 * Each sample's time per iteration, in nanoseconds, for criterion's benchmark `name` in group
 * `group`: criterion times each sample as a batch of iterations, and records both.
 */
export function samples(directory, group, name) {
	const { iters, times } = JSON.parse(
		readFileSync(`${directory}/${group}/${name}/new/sample.json`, "utf8"),
	);
	return times.map((time, at) => time / iters[at]);
}
