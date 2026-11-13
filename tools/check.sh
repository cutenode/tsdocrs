#!/usr/bin/env bash
# Everything CI checks: tools/lint.sh, tools/coverage.sh, the differential tests, then the npm
# package built (tools/build-npm.sh), tested and checked with publint. The differential tests and
# publint need `npm ci` first.
set -euo pipefail
cd "$(dirname "$0")/.."

tools/lint.sh
tools/coverage.sh
cargo nextest run --locked --test differential
tools/build-npm.sh
node --test npm/tsdocrs/*.test.mjs
npm run --silent publint -- npm/tsdocrs
