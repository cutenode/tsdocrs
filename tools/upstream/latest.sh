#!/usr/bin/env bash
# Prints the sha of microsoft/tsdoc's newest commit touching tsdoc/src, for tools/upstream/vendor.sh.
set -euo pipefail

gh api 'repos/microsoft/tsdoc/commits?path=tsdoc/src&per_page=1' --jq '.[0].sha'
