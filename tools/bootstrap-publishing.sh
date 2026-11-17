#!/usr/bin/env bash
# Sets up what release.yml needs to publish: the `npm` and `crates-io` GitHub environments its
# publishing jobs run in, and each registry trusting release.yml from its environment, staged
# publishing only on npm. Both registries only trust a workflow with a package they already have, so
# an npm package that isn't there yet gets published first, built from the latest GitHub release's
# tag. Finished steps are skipped, so run it again after a failure.
#
# npm expires a new trust config that hasn't published within 2 days
# (https://docs.npmjs.com/trusted-publishers), so merge the next release PR within 2 days of running
# this, or run it again first with RETRUST=1 to replace the config.
#
# Needs `gh` logged in as a repository admin, `npm login` as an owner of the tsdocrs package or with
# the name still free, the wasm-bindgen CLI tools/build-npm.sh checks for, and asks for a crates.io
# token with the trusted-publishing scope (https://crates.io/settings/tokens/new).
set -euo pipefail
cd "$(dirname "$0")/.."
owner=cutenode repo=tsdocrs workflow=release.yml

npm whoami >/dev/null
if [[ -z ${CARGO_REGISTRY_TOKEN:-} ]]; then
	read -rsp "crates.io token: " CARGO_REGISTRY_TOKEN
	echo
fi

# An environment that's already there keeps whatever protection it has.
for environment in npm crates-io; do
	if gh api "repos/$owner/$repo/environments/$environment" >/dev/null 2>&1; then
		echo "GitHub environment $environment already exists"
		continue
	fi
	echo '{"deployment_branch_policy": {"protected_branches": false, "custom_branch_policies": true}}' |
		gh api -X PUT "repos/$owner/$repo/environments/$environment" --input - >/dev/null
	# Only release tags can deploy to it, so only they can publish.
	gh api "repos/$owner/$repo/environments/$environment/deployment-branch-policies" -f name='v*' -f type=tag >/dev/null
	echo "Created GitHub environment $environment for v* tags"
done

crates() {
	curl -sS --fail-with-body -A "tsdocrs bootstrap (https://github.com/$owner/$repo)" \
		-H @<(printf 'Authorization: %s\n' "$CARGO_REGISTRY_TOKEN") "$@"
}
config=$(jq -n --arg owner $owner --arg repo $repo --arg workflow $workflow \
	'{crate: "tsdocrs", repository_owner: $owner, repository_name: $repo, workflow_filename: $workflow, environment: "crates-io"}')
configs=https://crates.io/api/v1/trusted_publishing/github_configs
if crates "$configs?crate=tsdocrs" | jq -e --argjson config "$config" \
	'any(.github_configs[]; .repository_owner == $config.repository_owner and .repository_name == $config.repository_name and .workflow_filename == $config.workflow_filename and .environment == $config.environment)' >/dev/null; then
	echo "crates.io already trusts $workflow for tsdocrs"
else
	jq -n --argjson config "$config" '{github_config: $config}' |
		crates -X POST -H 'Content-Type: application/json' --data @- "$configs" >/dev/null
	echo "crates.io trusts $workflow for tsdocrs"
fi

if [[ -z $(npm view tsdocrs version 2>/dev/null) ]]; then
	# A worktree at the tag, so what's published is what the release's crate was built from.
	tag=$(gh release view -R $owner/$repo --json tagName -q .tagName)
	dir=$(mktemp -d)
	git fetch -q origin tag "$tag"
	git worktree add -q --detach "$dir" "$tag"
	"$dir/tools/build-npm.sh"
	(cd "$dir/npm/tsdocrs" && npm publish --access public)
	git worktree remove --force "$dir"
fi

# npm keeps one config per package, and an expired one still lists, so replace whatever's there
# unless it's the one uppt needs. Listing takes a second factor, which npm only asks for on a
# terminal, so it runs on one of its own from script(1) (macOS's arguments) and leaves what it
# printed in a file to read the configs back from.
transcript=$(mktemp)
script -q "$transcript" npm trust list tsdocrs --json
trusts=$(tr -d '\r' <"$transcript" | sed -n '/^{$/,/^}$/p' | jq -s .)
rm "$transcript"
if [[ -z ${RETRUST:-} ]] && jq -e --arg repo $owner/$repo --arg file $workflow \
	'length == 1 and (.[0] | .repository == $repo and .file == $file and .environment == "npm" and .permissions == ["createStagedPackage"])' \
	<<<"$trusts" >/dev/null; then
	echo "npm already trusts $workflow to stage tsdocrs"
	exit
fi
for id in $(jq -r '.[].id' <<<"$trusts"); do
	npm trust revoke tsdocrs --id "$id"
done
npm trust github tsdocrs --repo $owner/$repo --file $workflow --env npm --allow-stage-publish --yes
