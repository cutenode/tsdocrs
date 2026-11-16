// What a benchmark ran on, so two runs can be compared: the versions of everything measured, and
// the machine.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import os from "node:os";

/**
 * The first line `command` prints, run in `cwd` with `env`, or what it failed with. Lines end in
 * CRLF on Windows.
 */
export function output(command, args, cwd, env = process.env) {
	try {
		return execFileSync(command, args, {
			cwd,
			env,
			encoding: "utf8",
			stdio: ["ignore", "pipe", "ignore"],
		}).split(/\r?\n/)[0];
	} catch (error) {
		return `unavailable (${error.code ?? error.status})`;
	}
}

/** The version a lockfile pins for `name`, from Cargo.lock's text. */
export function cargoLockVersion(lock, name) {
	return lock.match(new RegExp(`name = "${name}"\\r?\\nversion = "([^"]+)"`))?.[1] ?? "unknown";
}

/** The environment of a run in the checkout at `root`, on the machine `system` (Node's `os`) describes. */
export function environment(root, system = os) {
	const npmVersion = (name) =>
		JSON.parse(readFileSync(`${root}/node_modules/${name}/package.json`, "utf8")).version;
	const lock = readFileSync(`${root}/Cargo.lock`, "utf8");
	// Some platforms don't say what CPU they have.
	const cpus = system.cpus();
	return {
		commit: output("git", ["rev-parse", "--short", "HEAD"], root),
		tsdocrs: cargoLockVersion(lock, "tsdocrs"),
		"@microsoft/tsdoc": npmVersion("@microsoft/tsdoc"),
		rustc: output("rustc", ["--version"], root),
		node: process.version,
		criterion: cargoLockVersion(lock, "criterion"),
		tinybench: npmVersion("tinybench"),
		"wasm-bindgen": cargoLockVersion(
			readFileSync(`${root}/npm/tsdocrs/Cargo.lock`, "utf8"),
			"wasm-bindgen",
		),
		os: `${system.type()} ${system.release()} ${system.arch()}`,
		cpu: `${cpus[0]?.model ?? "unknown"}, ${cpus.length} cores`,
		memory: `${Math.round(system.totalmem() / 2 ** 30)} GiB`,
	};
}
