#!/usr/bin/env bash
# Rebuilds bench/corpus/ from its pinned sources: the doc comments of microsoft/tsdoc's own
# sources at a commit (small), TypeScript's ES library declarations (medium) and its DOM ones
# (large), from the npm registry's tarball at a version, checked against its integrity hash. The benchmarks
# read the committed files; this only says where they came from, and remakes them.
set -euo pipefail
cd "$(dirname "$0")/../.."

tsdoc_commit=87457daa029f68ad955422a6ae3f7ce2be59fc11
typescript=5.9.3
typescript_integrity=sha512-jl1vZzPDinLr9eUt3J/t7V6FgNEw9QjvBPdysz9KfQDD41fQrC2Y4vKQdiaUpFT4bXlb1RHhLpp8wtm6M5TgSw==

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

git -C "$work" init -q tsdoc
git -C "$work/tsdoc" fetch -q --depth 1 https://github.com/microsoft/tsdoc.git "$tsdoc_commit"
git -C "$work/tsdoc" checkout -q FETCH_HEAD

tarball=$work/typescript.tgz
curl -fsSL -o "$tarball" "https://registry.npmjs.org/typescript/-/typescript-$typescript.tgz"
actual=sha512-$(openssl dgst -sha512 -binary "$tarball" | openssl base64 -A)
if [[ $actual != "$typescript_integrity" ]]; then
  echo "typescript@$typescript's tarball doesn't match its integrity hash" >&2
  exit 1
fi
tar -xzf "$tarball" -C "$work"

mapfile -t small < <(find "$work/tsdoc/tsdoc/src" -name '*.ts' -not -path '*__tests__*' | sort)
mapfile -t medium < <(find "$work/package/lib" -name 'lib.es*.d.ts' | sort)
node tools/bench/extract.mjs bench/corpus/small.json "${small[@]}"
node tools/bench/extract.mjs bench/corpus/medium.json "${medium[@]}"
node tools/bench/extract.mjs bench/corpus/large.json "$work/package/lib/lib.dom.d.ts"
