#!/usr/bin/env bash
# Bumps Cargo.toml, npm/tsdocrs/Cargo.toml's tsdocrs dependency, and the Cargo.lock files that record
# tsdocrs to npm/tsdocrs/package.json's version on each open release PR (release/v* in this
# repository) and pushes the commit. uppt/pr only bumps package.json, so release.yml runs this after
# it. A branch already at that version is left alone, and uppt keeps the extra commit when it
# updates the PR.
#
# Needs GH_TOKEN, allowed to push and to read pull requests.
set -euo pipefail
cd "$(dirname "$0")/.."

gh auth setup-git
branches=$(gh pr list --state open --json headRefName,isCrossRepository \
	-q '.[] | select(.isCrossRepository | not) | .headRefName | select(startswith("release/v"))')
for branch in $branches; do
	# A worktree per branch, so this script isn't swapped for the branch's copy while it runs.
	dir=$(mktemp -d)
	git fetch -q --depth 1 origin "$branch"
	git worktree add -q --detach "$dir" FETCH_HEAD
	(
		cd "$dir"
		version=$(jq -r .version npm/tsdocrs/package.json)
		# [package]'s version is the first in the file.
		perl -0777 -pi -e "s/^version = \".*?\"/version = \"$version\"/m" Cargo.toml
		perl -pi -e "s/^(tsdocrs = \{ version = )\"[^\"]*\"/\$1\"$version\"/" npm/tsdocrs/Cargo.toml
		if git diff --quiet; then
			echo "$branch: already at $version"
			exit
		fi
		# npm/tsdocrs and fuzz/ are workspaces of their own, each with a Cargo.lock that takes the
		# crate by path.
		for manifest in Cargo.toml npm/tsdocrs/Cargo.toml fuzz/Cargo.toml; do
			cargo update -p tsdocrs --manifest-path "$manifest"
		done
		if [[ $(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version') != "$version" ]]; then
			echo "::error::couldn't set Cargo.toml's version to $version on $branch"
			exit 1
		fi
		git -c user.name='github-actions[bot]' -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
			commit -qam "chore(release): bump the crate to v$version"
		git push -q origin "HEAD:refs/heads/$branch"
		echo "$branch: bumped the crate to $version"
	)
	git worktree remove --force "$dir"
done
