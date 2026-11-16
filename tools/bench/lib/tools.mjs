// The wasm-bindgen CLI the npm package needs, which `npm run bench` installs when it's missing.

/** The wasm-bindgen version npm/tsdocrs pins exactly, from its Cargo.toml's text. */
export function wasmBindgenPin(manifest) {
	const pin = manifest.match(/^wasm-bindgen = "=([^"]+)"$/m)?.[1];
	if (!pin) {
		throw new Error("npm/tsdocrs/Cargo.toml doesn't pin wasm-bindgen exactly");
	}
	return pin;
}

/** Whether `wasm-bindgen --version`'s output, if it ran, is version `pin`. */
export function isPinned(output, pin) {
	return output?.trim() === `wasm-bindgen ${pin}`;
}
