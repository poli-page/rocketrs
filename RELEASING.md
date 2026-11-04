# Releasing `poli-page-rocket`

Releases are tag-driven, as in the [`poli-page` SDK](https://github.com/poli-page/sdk-rust).

## Versions

`poli-page-rocket` shares its major.minor with the Poli Page API: crate `0.13.x` talks to API `0.13`, passes the [API contract](https://github.com/poli-page/api-contract) `v0.13.0`, and requires the `poli-page` SDK of the same minor from crates.io (`poli-page = "0.13"`, which Cargo reads as `^0.13`: `>=0.13.0, <0.14.0`). Patch numbers are free: a fix to the crate alone is a patch, whatever the API's or the SDK's patch. A change of the API contract, a new feature, and a raise of the minimum Rust (MSRV) or Rocket version are each a minor. The crate may skip minors when it has nothing to ship for one. The Release workflow refuses a tag whose major.minor differs from the contract version pinned in `.github/workflows/ci.yml` (and `release.yml`, which must pin the same commit) or from the `poli-page` requirement in `Cargo.toml` and `example-app/Cargo.toml`.

## How a release runs

Pushing a `vX.Y.Z` tag runs `.github/workflows/release.yml`:

1. **verify**: checks that the tag equals the `Cargo.toml` version and that `CHANGELOG.md` has a `## [X.Y.Z] - YYYY-MM-DD` section. `scripts/check-release-versions.sh` then checks the versions: the pinned API contract commit must carry a stable tag `vX.Y.Z` of the tag's major.minor (the `# vX.Y.Z` comment next to the pin is not trusted, and both workflows must pin the same commit), and `poli-page` must be required from crates.io as `"X.Y"` (or `"X.Y.Z"`) of the tag's major.minor. The error names both versions. The job then runs `cargo fmt`, `clippy -D warnings`, `cargo doc`, the crate's tests, the example app's format, clippy and tests with the API contract pinned in `ci.yml`, `cargo deny check advisories`, and `cargo publish --dry-run`.
2. **publish**: waits for an approval on the `crates-io-publish` environment, exchanges the job's OIDC token for a short-lived crates.io token (Trusted Publishing, no stored secret), runs `cargo publish`, and creates the GitHub release with the CHANGELOG section as notes. A version already on crates.io is not uploaded again; the release is still created.

A pushed tag is the only trigger; there is no manual-run path.

Crate: [`poli-page-rocket`](https://crates.io/crates/poli-page-rocket) on crates.io. The name was free when `0.13.0` was prepared (`https://crates.io/api/v1/crates/poli-page-rocket` answered 404).

What the crate ships: the `include` list in `Cargo.toml` (`src/`, `README.md`, `CHANGELOG.md`, both licences), plus the `Cargo.toml`, `Cargo.lock` and `.cargo_vcs_info.json` cargo adds. The example app, the tests and local tooling (CI, lint and formatter config) stay out. Check with:

```bash
cargo package --list
```

## First release (one time, 0.13.0)

crates.io has no "pending publisher": Trusted Publishing can be configured only on a crate that exists. So `0.13.0` is published by hand, then the workflow takes over.

The protected environment comes first. A workflow that names an environment that does not exist makes GitHub create it with no protection rules. Once the trusted publisher accepts `crates-io-publish`, an unprotected environment would let any tag push publish with no approval.

1. In the GitHub repo, Settings → Environments → New environment `crates-io-publish`:
   - Required reviewers: the people who approve releases.
   - Deployment branches and tags: "Selected branches and tags", with one tag rule `v*.*.*`.
   - No secret is needed.
2. Merge the release PR on `main` and wait for CI to be green. Note the merge commit's SHA.
3. On crates.io, log in with the GitHub account that will own the crate. Verify the account's email first under Account Settings: crates.io refuses a publish from an unverified account.
4. Under Account Settings → API Tokens, create a token:
   - Scopes: `publish-new`. Add `change-owners` only if you will add a team as an owner (the optional step 6).
   - Crates: restricted to `poli-page-rocket`.
   - A short expiry.
5. Publish from a fresh clone of the merge commit. Paste the block below into bash or zsh, with the merge commit's SHA in the first line. It clones into a new temporary directory, checks that the commit is `origin/main`'s head and that the tree is clean, runs the release check, the package list and a dry run, then reads the token without echoing it (so it stays out of shell history) and publishes. Any failed step stops the chain before the publish. The last line removes the token from the shell whatever happened.
   ```bash
   sha='<merge sha>'
   dir="$(mktemp -d)" && git clone https://github.com/poli-page/rocketrs.git "$dir/rocketrs" && cd "$dir/rocketrs" \
     && git switch --detach "$sha" \
     && test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" \
     && test -z "$(git status --porcelain)" \
     && ./scripts/check-release-versions.sh v0.13.0 \
     && cargo package --list --locked \
     && cargo publish --dry-run --locked \
     && printf 'crates.io token: ' && read -rs CARGO_REGISTRY_TOKEN && echo && export CARGO_REGISTRY_TOKEN \
     && cargo publish --locked \
     || echo 'Stopped before the publish, or the publish failed: see the output above. The commit must be the head of origin/main and the tree clean.'
   unset CARGO_REGISTRY_TOKEN
   ```
6. Optional: if the crate should not belong to one account, add the organisation's team as an owner (the token needs `change-owners`), in the same shell:
   ```bash
   printf 'crates.io token: ' && read -rs CARGO_REGISTRY_TOKEN && echo && export CARGO_REGISTRY_TOKEN \
     && cargo owner --add 'github:poli-page:<team>' poli-page-rocket
   unset CARGO_REGISTRY_TOKEN
   ```
7. On the crate's page, Settings → Trusted Publishing, add a GitHub publisher:
   - Repository owner `poli-page`, repository `rocketrs`.
   - Workflow `release.yml`.
   - Environment `crates-io-publish`.
8. Revoke the token from step 4.
9. Tag the published commit and push the tag, in the same shell as step 5 (from another clone, fetch first and put the SHA in place of `"$sha"`): `git tag v0.13.0 "$sha" && git push origin v0.13.0`. Approve the `publish` job. It skips the upload (0.13.0 is already on crates.io), checks the OIDC exchange, and creates the GitHub release.
10. Check that `https://docs.rs/poli-page-rocket/0.13.0` built (a few minutes after the publish).

## Each release

1. Bump `version` in `Cargo.toml`, and run `cargo check` here and in `example-app/` so both `Cargo.lock` files follow.
2. Update the version shown in `example-app/static/index.html` (two labels), and the compatibility table in `README.md` when the minor, the MSRV or the Rocket version changes.
   For a new minor `X.Y`: require `poli-page = "X.Y"` in `Cargo.toml` and `example-app/Cargo.toml` and run `cargo update -p poli-page` in both directories, pin the API contract's `vX.Y.0` commit in `.github/workflows/ci.yml` and `release.yml`, and check both with `./scripts/check-release-versions.sh X.Y.Z` (it needs `cargo`, `jq` and network access to list the contract's tags).
3. Move `[Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` in `CHANGELOG.md` and update the compare links. Keep the heading exactly that shape: the workflow checks it and uses the section as the release notes.
4. Commit `chore: release X.Y.Z` through a PR on `main` and wait for CI to be green.
5. Tag the merge commit and push the tag:
   ```bash
   git switch main && git pull --ff-only
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   The tag is only the trigger: `verify` refuses to publish when it differs from `Cargo.toml`.
6. Approve the `publish` job on the `crates-io-publish` environment.
7. Check `https://docs.rs/poli-page-rocket/X.Y.Z`.

A published version cannot be overwritten. A broken one is yanked (`cargo yank --version X.Y.Z poli-page-rocket`) and fixed in a new version.
