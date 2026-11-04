#!/usr/bin/env bash
# scripts/check-release-versions.sh: a release's major.minor must be the
# pinned API contract's and the required poli-page SDK's.
#
# Usage: scripts/check-release-versions.sh X.Y.Z   (or vX.Y.Z, or a prerelease)
#
# The crate shares its major.minor with the Poli Page API, and integration
# 0.N requires the poli-page SDK 0.N.x. This script:
#   - reads the contract commit pinned in .github/workflows/ci.yml (never its
#     `# vX.Y.Z` comment), checks that every workflow pins that same commit,
#     lists the contract repository's tags, and fails unless that commit
#     carries a stable tag vX.Y.Z with the release's major.minor;
#   - fails unless Cargo.toml and example-app/Cargo.toml require poli-page
#     from crates.io as "X.Y" or "X.Y.Z" (Cargo's default caret: ^X.Y) with
#     the release's major.minor, with no [patch], [replace] or `paths`
#     override of it in the manifests or in a .cargo/config(.toml);
#   - fails unless Cargo.lock and example-app/Cargo.lock resolve poli-page
#     from crates.io at a version of the release's major.minor.
# Patch numbers are free.
#
# Run by the Release workflow. Needs cargo and jq. API_CONTRACT_REMOTE
# overrides the contract repository (to check against a local one).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOWS="$REPO_ROOT/.github/workflows"
REMOTE="${API_CONTRACT_REMOTE:-https://github.com/poli-page/api-contract.git}"
CRATES_IO="registry+https://github.com/rust-lang/crates.io-index"

fail() { echo "check-release-versions: $1" >&2; exit 1; }

[ "$#" -eq 1 ] || fail "usage: $(basename "$0") X.Y.Z"
version="${1#v}"
if ! [[ "$version" =~ ^([0-9]+)\.([0-9]+)\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
    fail "release version '$1' is not X.Y.Z"
fi
release_minor="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"

# poli-page: ^X.Y or ^X.Y.Z of the release's major.minor, from crates.io, in
# the crate and in the example app. Cargo writes a bare "0.13" as "^0.13".
for manifest in Cargo.toml example-app/Cargo.toml; do
    [ -f "$REPO_ROOT/$manifest" ] || fail "$manifest not found"
    metadata="$(cargo metadata --format-version 1 --no-deps --offline \
        --manifest-path "$REPO_ROOT/$manifest")" || fail "cargo metadata failed on $manifest"
    deps="$(jq -r '
        .workspace_members as $members
        | .packages[] | select(.id as $id | $members | index($id))
        | .dependencies[] | select(.name == "poli-page")
        | "\(.req)\t\(.source // "")"' <<<"$metadata")"
    [ -n "$deps" ] || fail "$manifest does not require poli-page"
    while IFS=$'\t' read -r req source; do
        if [ "$source" != "$CRATES_IO" ]; then
            fail "$manifest takes poli-page from '${source:-a path}', not from crates.io"
        fi
        if ! [[ "$req" =~ ^\^([0-9]+)\.([0-9]+)(\.[0-9]+)?$ ]]; then
            fail "release v$version needs poli-page \"$release_minor\", but $manifest requires '$req'"
        fi
        sdk_minor="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"
        if [ "$sdk_minor" != "$release_minor" ]; then
            fail "release v$version and the required poli-page $req ($manifest) differ in major.minor: require poli-page \"$release_minor\", or release v$sdk_minor.x"
        fi
    done <<<"$deps"
done

# No [patch] or [replace] of poli-page (nor a `paths` override), in either
# manifest or in a Cargo config file of the repository: the tests would run
# against another SDK than the one the published crate gets. Whitespace and
# quotes are dropped first, so `"poli-page:0.13.0" = …` reads poli-page:0.13.0=….
for file in Cargo.toml example-app/Cargo.toml \
    .cargo/config .cargo/config.toml example-app/.cargo/config example-app/.cargo/config.toml; do
    [ -f "$REPO_ROOT/$file" ] || continue
    hit="$(tr -d " \t\"'" <"$REPO_ROOT/$file" | awk '
        /^#/ { next }
        /^\[/ {
            section = $0
            if (section ~ /^\[(patch|replace)(\]|\.)/ && section ~ /poli-page/) { print NR ": " $0; exit }
            next
        }
        section ~ /^\[(patch|replace)(\]|\.)/ && ($0 ~ /^([^=]*\.)?poli-page[=.:]/ || $0 ~ /package=poli-page[,}]/) { print NR ": " $0; exit }
        section == "" && $0 ~ /^(patch|replace)\./ && $0 ~ /poli-page/ { print NR ": " $0; exit }
        section == "" && $0 ~ /^paths=/ { print NR ": " $0; exit }
    ')"
    [ -z "$hit" ] || fail "$file overrides poli-page (line $hit): remove the [patch], [replace] or paths entry before a release"
done

# What the committed lock files resolve: poli-page from crates.io, at a
# version of the release's major.minor, and no unused patch of it.
for lock in Cargo.lock example-app/Cargo.lock; do
    [ -f "$REPO_ROOT/$lock" ] || fail "$lock not found"
    entries="$(awk '
        function flush() { if (name == "poli-page") print kind "\t" ver "\t" src }
        /^\[/ { flush(); kind = $0; name = ver = src = ""; next }
        $1 == "name" { name = $3; gsub(/"/, "", name) }
        $1 == "version" { ver = $3; gsub(/"/, "", ver) }
        $1 == "source" { src = $3; gsub(/"/, "", src) }
        END { flush() }
    ' "$REPO_ROOT/$lock")"
    [ -n "$entries" ] || fail "$lock resolves no poli-page"
    while IFS=$'\t' read -r kind ver src; do
        if [ "$kind" != "[[package]]" ]; then
            fail "$lock records a $kind entry for poli-page $ver: remove the patch and update the lock"
        fi
        if [ "$src" != "$CRATES_IO" ]; then
            fail "$lock resolves poli-page $ver from '${src:-a path}', not from crates.io"
        fi
        if ! [[ "$ver" =~ ^([0-9]+)\.([0-9]+)\. ]] || [ "${BASH_REMATCH[1]}.${BASH_REMATCH[2]}" != "$release_minor" ]; then
            fail "release v$version needs poli-page $release_minor.x, but $lock resolves poli-page $ver: run cargo update -p poli-page"
        fi
    done <<<"$entries"
done
echo "poli-page ^$release_minor is required and locked from crates.io, matching release v$version"

# A live `uses:` line only, so a commented-out pin does not count.
pin_pattern='s/^[[:space:]]*(-[[:space:]]*)?uses:[[:space:]]*poli-page\/api-contract@([0-9a-f]{40}).*/\2/p'
contract_sha="$(sed -nE "$pin_pattern" "$WORKFLOWS/ci.yml" | head -n 1)"
[ -n "$contract_sha" ] || fail "no poli-page/api-contract pin found in .github/workflows/ci.yml"

# The release runs the contract tests with its own pin: all pins must agree.
other_pins="$(sed -nE "$pin_pattern" "$WORKFLOWS"/*.yml | sort -u | grep -v "^$contract_sha\$" || true)"
if [ -n "$other_pins" ]; then
    fail "the workflows pin different API contract commits ($contract_sha in ci.yml, also $(tr '\n' ' ' <<<"$other_pins" | sed 's/ $//')): pin one everywhere"
fi

tags="$(git ls-remote --tags "$REMOTE")" || fail "cannot list the tags of $REMOTE"

# An annotated tag is listed twice: its own object, then `^{}` with the commit
# it points at. A lightweight tag is listed once, with its commit.
stable_tags="$(
    awk -v sha="$contract_sha" '$1 == sha { ref = $2; sub(/^refs\/tags\//, "", ref); sub(/\^\{\}$/, "", ref); print ref }' <<<"$tags" \
        | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -u -V || true
)"
short_sha="${contract_sha:0:8}"
if [ -z "$stable_tags" ]; then
    fail "release v$version needs the API contract v$release_minor.x, but the pinned contract commit $short_sha has no stable vX.Y.Z tag: pin a released contract"
fi

while IFS= read -r tag; do
    if [[ "$tag" =~ ^v([0-9]+)\.([0-9]+)\. ]] && [ "${BASH_REMATCH[1]}.${BASH_REMATCH[2]}" = "$release_minor" ]; then
        echo "API contract $tag ($short_sha) matches release v$version"
        exit 0
    fi
done <<<"$stable_tags"

pinned="$(tr '\n' ' ' <<<"$stable_tags" | sed 's/ $//')"
# The newest of the commit's tags, which may carry two (an unchanged contract).
[[ "$(tail -n 1 <<<"$stable_tags")" =~ ^v([0-9]+)\.([0-9]+)\. ]]
fail "release v$version and the pinned API contract $pinned ($short_sha) differ in major.minor: pin the contract v$release_minor.x, or release v${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.x"
