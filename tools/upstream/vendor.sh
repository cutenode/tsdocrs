#!/usr/bin/env bash
# Vendors @microsoft/tsdoc's own Jest suite at COMMIT into OUT: runs every
# tsdoc/src/**/__tests__/*.test.ts with tools/upstream/recorder.cjs watching, and writes what each
# test did to OUT/cases.json (with the snapshots' checked-in text), the counts to
# OUT/manifest.json, and upstream's test files as they are to OUT/suite/. The harness reads another
# OUT than tests/upstream through UPSTREAM_DIR, which is how `npm run bench -- --upstream COMMIT`
# runs it on a suite that isn't checked in. Fails with Jest's output if upstream's suite fails at
# COMMIT.
# Usage: tools/upstream/vendor.sh [COMMIT [OUT]]
#   COMMIT defaults to the commit in tests/upstream/manifest.json, OUT to tests/upstream.
set -euo pipefail
cd "$(dirname "$0")/../.."

commit=${1:-$(node -p 'require("./tests/upstream/manifest.json").commit')}
out=${2:-tests/upstream}
if [[ ! $commit =~ ^[0-9a-f]{40}$ ]]; then
	echo "vendor.sh: COMMIT must be a full sha, got '$commit'" >&2
	exit 1
fi

# pwd -P because Jest reports real paths, and macOS's temp dir is behind a symlink.
work=$(mktemp -d "${TMPDIR:-/tmp}/tsdoc-upstream.XXXXXX")
trap 'rm -rf "$work"' EXIT
work=$(cd "$work" && pwd -P)
checkout=$work/tsdoc

git init --quiet "$checkout"
git -C "$checkout" fetch --quiet --depth 1 https://github.com/microsoft/tsdoc.git "$commit"
git -C "$checkout" checkout --quiet FETCH_HEAD
date=$(git -C "$checkout" log -1 --format=%cI)

# jest 29.7.0 is the last 29 release, so jest's own ^29 dependencies all resolve to it. Upstream
# runs 29.5.0, with the same snapshot format.
npm install --silent --no-save --no-package-lock --ignore-scripts --no-audit --no-fund --prefix "$work/deps" \
	jest@29.7.0 jest-environment-jsdom@29.7.0 ts-jest@29.4.15 typescript@5.8.3 >/dev/null

mkdir "$work/records"
if ! UPSTREAM_CHECKOUT=$checkout UPSTREAM_DEPS=$work/deps UPSTREAM_RECORDS=$work/records \
	node "$work/deps/node_modules/jest/bin/jest.js" --config tools/upstream/jest.config.cjs --ci --no-watchman \
	--cacheDirectory "$work/cache" --json --outputFile "$work/results.json" >"$work/jest.log" 2>&1; then
	cat "$work/jest.log" >&2
	exit 1
fi

mkdir -p "$out"
node tools/upstream/write.mjs "$checkout" "$work/records" "$work/results.json" "$commit" "$date" "$out"

# From git's objects rather than the checkout, which upstream's .gitattributes gives CRLF .ts files.
rm -rf "$out/suite"
git -C "$checkout" ls-tree -r --name-only HEAD tsdoc/src | grep '/__tests__/' | while read -r file; do
	mkdir -p "$out/suite/$(dirname "$file")"
	git -C "$checkout" cat-file blob "HEAD:$file" >"$out/suite/$file"
done
