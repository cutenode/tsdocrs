// Jest config for tools/upstream/vendor.sh: upstream's own (tsdoc/config/jest.config.json and the
// tsdoc-build-rig and heft profiles it extends), with ts-jest in place of heft's tsc build.
const path = require("node:path");

const { UPSTREAM_CHECKOUT: checkout, UPSTREAM_DEPS: deps } = process.env;
const dependency = (name) => require.resolve(name, { paths: [deps] });

module.exports = {
	rootDir: path.join(checkout, "tsdoc"),
	testMatch: ["<rootDir>/src/**/__tests__/*.test.ts"],
	moduleFileExtensions: ["ts", "js", "json"],
	testEnvironment: dependency("jest-environment-jsdom"),
	testEnvironmentOptions: {
		url: "http://localhost/",
		customExportConditions: ["require", "node", "umd"],
	},
	snapshotFormat: { escapeString: true, printBasicPrototype: true },
	clearMocks: true,
	setupFilesAfterEnv: [path.join(__dirname, "recorder.cjs")],
	transform: {
		"\\.ts$": [
			dependency("ts-jest"),
			// Explicit, or ts-jest would find upstream's tsconfig.json, which extends an uninstalled rig.
			{ tsconfig: path.join(__dirname, "tsconfig.json") },
		],
	},
};
