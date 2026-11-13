#!/usr/bin/env bash
# Vendors @microsoft/tsdoc's own Jest suite at COMMIT: runs every tsdoc/src/**/__tests__/*.test.ts
# with tools/upstream/recorder.cjs watching, and writes what each test did to
# tests/upstream/cases.json (with the snapshots' checked-in text), the counts to
# tests/upstream/manifest.json, and upstream's test files as they are to tests/upstream/suite/.
# Fails with Jest's output if upstream's suite fails at COMMIT.
# Usage: tools/upstream/vendor.sh [COMMIT]   (default: the commit in tests/upstream/manifest.json)
set -euo pipefail
cd "$(dirname "$0")/../.."

commit=${1:-$(node -p 'require("./tests/upstream/manifest.json").commit')}
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

node tools/upstream/write.mjs "$checkout" "$work/records" "$work/results.json" "$commit" "$date"

# From git's objects rather than the checkout, which upstream's .gitattributes gives CRLF .ts files.
rm -rf tests/upstream/suite
git -C "$checkout" ls-tree -r --name-only HEAD tsdoc/src | grep '/__tests__/' | while read -r file; do
	mkdir -p "tests/upstream/suite/$(dirname "$file")"
	git -C "$checkout" cat-file blob "HEAD:$file" >"tests/upstream/suite/$file"
done
