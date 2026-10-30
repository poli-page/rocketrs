# Releasing `poli-page-rocket`

Releases are tag-driven, as in the [`poli-page` SDK](https://github.com/poli-page/sdk-rust). Pushing a `vX.Y.Z` tag runs `.github/workflows/release.yml`:

1. **verify**: checks that the tag equals the `Cargo.toml` version and that `CHANGELOG.md` has a `## [X.Y.Z] - YYYY-MM-DD` section, then runs `cargo fmt`, `clippy -D warnings`, `cargo doc`, the crate's tests, the example app's format, clippy and tests with the API contract pinned in `ci.yml`, and `cargo publish --dry-run`.
2. **publish**: waits for an approval on the `crates-io-publish` environment, exchanges the job's OIDC token for a short-lived crates.io token (Trusted Publishing, no stored secret), runs `cargo publish`, and creates the GitHub release with the CHANGELOG section as notes. A version already on crates.io is not uploaded again; the release is still created.

`workflow_dispatch` with a `version` input is a fallback that creates and pushes the tag for you after the same gate and approval.

Crate: [`poli-page-rocket`](https://crates.io/crates/poli-page-rocket) on crates.io. The name was free when `0.11.0` was prepared (`https://crates.io/api/v1/crates/poli-page-rocket` answered 404).

What the crate ships: the `include` list in `Cargo.toml` (`src/`, `README.md`, `CHANGELOG.md`, both licences), plus the `Cargo.toml`, `Cargo.lock` and `.cargo_vcs_info.json` cargo adds. The example app, the tests and local tooling (CI, lint and formatter config) stay out. Check with:

```bash
cargo package --list
```

## First release (one time, 0.11.0)

crates.io has no "pending publisher": Trusted Publishing can be configured only on a crate that exists. So `0.11.0` is published by hand, then the workflow takes over.

1. Merge the release PR on `main` and wait for CI to be green.
2. On crates.io (logged in with the GitHub account that will own the crate), create an API token under Account Settings → API Tokens with the `publish-new` scope, restricted to `poli-page-rocket`, with a short expiry.
3. From a clean checkout of the merged `main`:
   ```bash
   cargo package --list
   cargo publish --locked --token <token>
   ```
4. On the crate's page, Settings → Trusted Publishing → add a GitHub publisher: repository owner `poli-page`, repository `rocketrs`, workflow `release.yml`, environment `crates-io-publish`.
5. Add the `poli-page` organisation (or its owners team) as a crate owner if the crate should not belong to one account: `cargo owner --add github:poli-page:<team> poli-page-rocket`.
6. Revoke the token from step 2.
7. In the GitHub repo, Settings → Environments → New environment `crates-io-publish`, with required reviewers and deployment tags limited to `v*.*.*`. No secret is needed.
8. Push the tag: `git tag v0.11.0 && git push origin v0.11.0`. Approve the `publish` job. It skips the upload (0.11.0 is already on crates.io), checks the OIDC exchange, and creates the GitHub release.
9. Check that `https://docs.rs/poli-page-rocket/0.11.0` built (a few minutes after the publish).

## Each release

1. Bump `version` in `Cargo.toml`, and run `cargo check` here and in `example-app/` so both `Cargo.lock` files follow.
2. Update the version shown in `example-app/static/index.html`.
3. Move `[Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` in `CHANGELOG.md` and update the compare links. Keep the heading exactly that shape: the workflow checks it and uses the section as the release notes.
4. Commit `chore: release X.Y.Z` through a PR on `main` and wait for CI to be green.
5. Tag the merged commit and push the tag:
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   The tag is only the trigger: `verify` refuses to publish when it differs from `Cargo.toml`.
6. Approve the `publish` job on the `crates-io-publish` environment.
7. Check `https://docs.rs/poli-page-rocket/X.Y.Z`.

A published version cannot be overwritten. A broken one is yanked (`cargo yank --version X.Y.Z poli-page-rocket`) and fixed in a new version.
