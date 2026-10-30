# Releasing `poli-page-rocket`

Releases are tag-driven, as in the [`poli-page` SDK](https://github.com/poli-page/sdk-rust). Pushing a `vX.Y.Z` tag runs `.github/workflows/release.yml`:

1. **verify**: checks that the tag equals the `Cargo.toml` version and that `CHANGELOG.md` has a `## [X.Y.Z] - YYYY-MM-DD` section, then runs `cargo fmt`, `clippy -D warnings`, `cargo doc`, the crate's tests, the example app's format, clippy and tests with the API contract pinned in `ci.yml`, `cargo deny check advisories`, and `cargo publish --dry-run`.
2. **publish**: waits for an approval on the `crates-io-publish` environment, exchanges the job's OIDC token for a short-lived crates.io token (Trusted Publishing, no stored secret), runs `cargo publish`, and creates the GitHub release with the CHANGELOG section as notes. A version already on crates.io is not uploaded again; the release is still created.

A pushed tag is the only trigger; there is no manual-run path.

Crate: [`poli-page-rocket`](https://crates.io/crates/poli-page-rocket) on crates.io. The name was free when `0.11.0` was prepared (`https://crates.io/api/v1/crates/poli-page-rocket` answered 404).

What the crate ships: the `include` list in `Cargo.toml` (`src/`, `README.md`, `CHANGELOG.md`, both licences), plus the `Cargo.toml`, `Cargo.lock` and `.cargo_vcs_info.json` cargo adds. The example app, the tests and local tooling (CI, lint and formatter config) stay out. Check with:

```bash
cargo package --list
```

## First release (one time, 0.11.0)

crates.io has no "pending publisher": Trusted Publishing can be configured only on a crate that exists. So `0.11.0` is published by hand, then the workflow takes over.

The protected environment comes first. A workflow that names an environment that does not exist makes GitHub create it with no protection rules. Once the trusted publisher accepts `crates-io-publish`, an unprotected environment would let any tag push publish with no approval.

1. In the GitHub repo, Settings → Environments → New environment `crates-io-publish`:
   - Required reviewers: the people who approve releases.
   - Deployment branches and tags: "Selected branches and tags", with one tag rule `v*.*.*`.
   - No secret is needed.
2. Merge the release PR on `main` and wait for CI to be green. Note the merge commit's SHA.
3. On crates.io, log in with the GitHub account that will own the crate. Verify the account's email first under Account Settings: crates.io refuses a publish from an unverified account.
4. Under Account Settings → API Tokens, create a token:
   - Scopes: `publish-new` and `change-owners`.
   - Crates: restricted to `poli-page-rocket`.
   - A short expiry.
5. From a clean checkout of the merge commit, without leaving the token in shell history:
   ```bash
   git switch --detach <merge sha>
   cargo package --list
   read -rs CARGO_REGISTRY_TOKEN && export CARGO_REGISTRY_TOKEN
   cargo publish --locked
   ```
6. Optional: if the crate should not belong to one account, add the organisation's team as an owner, in the same shell: `cargo owner --add github:poli-page:<team> poli-page-rocket`. Then run `unset CARGO_REGISTRY_TOKEN`.
7. On the crate's page, Settings → Trusted Publishing, add a GitHub publisher:
   - Repository owner `poli-page`, repository `rocketrs`.
   - Workflow `release.yml`.
   - Environment `crates-io-publish`.
8. Revoke the token from step 4.
9. Tag the published commit and push the tag: `git tag v0.11.0 <merge sha> && git push origin v0.11.0`. Approve the `publish` job. It skips the upload (0.11.0 is already on crates.io), checks the OIDC exchange, and creates the GitHub release.
10. Check that `https://docs.rs/poli-page-rocket/0.11.0` built (a few minutes after the publish).

## Each release

1. Bump `version` in `Cargo.toml`, and run `cargo check` here and in `example-app/` so both `Cargo.lock` files follow.
2. Update the version shown in `example-app/static/index.html`.
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
