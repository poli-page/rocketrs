# Changelog

All notable changes to `poli-page-rocket` are documented here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- The live API CI job passes the optional `POLI_PAGE_BASE_URL` secret to the integration test (unset: the SDK default), next to the `pp_sandbox_` key in `POLI_PAGE_API_KEY`.

## [0.11.0] - 2026-10-30

First release. The version matches the `poli-page` SDK 0.11.0 it requires. Entries in Changed and Removed describe changes to the pre-release code on `main`; no `0.1.0` was ever published.

Breaking: requires the `poli-page` SDK 0.11.0, the first release with the environments contract (environment keys `pp_{environment}_{32 hex}`, no template versions, no render preview).

### Added
- Error handler, on by default: the `PoliPageError` responder answers the SDK error's `to_payload()` as JSON, `{ code, message, status, requestId }` (`status` and `requestId` are `null` when unknown), with that status: the API status, `503` for `network_error`, `504` for `timeout`, or `500` when the payload has none. `PoliPageFairing::error_handler(false)` turns it off: the responder then writes no body and hands the status to Rocket's catchers.
- `PoliPageFairing` with three constructors — `from_env`, `new(builder)`, `with_client` — that builds the SDK client at ignite-time and inserts it into Rocket's managed state. Ignite fails cleanly when `POLI_PAGE_API_KEY` is missing or empty.
- Env-var contract: `POLI_PAGE_API_KEY` (required) plus optional `POLI_PAGE_BASE_URL`, `POLI_PAGE_TIMEOUT_SECS`, `POLI_PAGE_MAX_RETRIES`, `POLI_PAGE_RETRY_DELAY_MS`.
- `PoliPageClient` newtype (`Clone + Send + Sync + 'static`) for managed-state extraction via `&State<PoliPageClient>`, plus optional `PoliPage<'_>` request guard for one-character-less ergonomics.
- Response types — `PdfResponse` (bytes + stream), `PreviewResponse`, `DocumentRedirect` — with RFC 5987 filename encoding for non-ASCII names. Default headers: `Cache-Control: private, no-store` and `X-Content-Type-Options: nosniff`. `Cache-Control` is overridable via `.cache_control(...)`.
- `PoliPageError` newtype wrapper around `poli_page::Error` with `Responder` and `From` impls — routes return `Result<T, PoliPageError>` and `?` runs the conversion. Its body and status are the error handler's (see the first Added entry and Changed).
- Default `on_retry` / `on_error` hooks bridge SDK events to `tracing` under the `poli_page_rocket` target. Overridable via a custom `PoliPageBuilder` passed to `PoliPageFairing::new(...)`.
- SDK re-exports at the crate root (`Error`, `RetryEvent`, `RenderInput`, `DocumentDescriptor`, `DocumentPreviewResult`, `ThumbnailFormat`, `ThumbnailOptions`, `PoliPage as SdkPoliPage`) so user code typically only imports from `poli_page_rocket::*`.
- Example Rocket 0.5 app at `example-app/` covering all 10 SDK demo steps, with an interactive HTML dashboard at `GET /` and a standalone `render_to_file` binary for SDK demo step 3.
- Contract tests: `example-app/tests/contract.rs` runs the `impl` cases of the Poli Page API contract against the example app with the SDK pointed at a local mock server. CI runs them on every toolchain leg against the pinned contract (v1.3.0), and also checks, builds and tests `example-app/`. Dependabot proposes each new contract version the next day.
- The example app is also a library target (`app(fairing)`, `rocket()`), so tests mount the same routes.

### Changed
- CI: the live API job runs on `main` (and on demand) against the production API with the sandbox key from the `POLI_PAGE_API_KEY` secret, and skips with a notice when the key is absent or not a `pp_sandbox_` key.
- Depends on `poli-page` `0.11.0` from crates.io (was `0.9.0`).
- MSRV raised from 1.75 to 1.93, the `rust-version` of the `poli-page` SDK this crate depends on; 1.75 could never build it. CI's MSRV job now really builds with 1.93 instead of the `rust-toolchain.toml` stable.
- Error answers: the body is the SDK's `to_payload()`, which adds `status` and carries the bare message (no `Display` prefix). `Error::Connection` answers `503` (was `502`), `Error::Aborted` `500` (was `503`), and `Error::Download` the storage status, or `500` without one (was `502`).
- `PoliPageFairing::from_env()` accepts any non-empty `POLI_PAGE_API_KEY` (`pp_sandbox_…`, `pp_live_…`, `pp_staging_…`, …) instead of failing ignition unless the key starts with `pp_test_` or `pp_live_`. A missing or empty key still fails ignition. A key that does not match `pp_{environment}_{32 lowercase hex}` logs one `tracing` warning under the `poli_page_rocket` target, naming the expected format and showing no part of the key (not even its `pp_{environment}_` part), and ignites anyway: the API decides whether the key is valid.
- Builds against the `poli-page` SDK whose render input is `RenderInput` with no `version`; the crate root re-exports `RenderInput`.
- Example app: renders send no `version`, and `GET /errors/not-found` (template `does-not-exist`, answering `404` `TEMPLATE_NOT_FOUND` through the error handler) replaces `GET /errors/bad-version`.

### Removed
- `impl From<poli_page::PreviewResult> for PreviewResponse` and the `InlineModeInput`, `PreviewResult` and `ProjectModeInput` re-exports: the SDK no longer has render preview or inline templates. `PreviewResponse` still builds from `DocumentPreviewResult` and from a `String`.
- Example app: the `/render/preview` route and its dashboard button.

### Fixed
- `PdfResponse` / `headers::content_disposition`: control characters (CR/LF, TAB, DEL, C1) are now stripped from the `Content-Disposition` filename instead of surviving as `_` in the ASCII fallback and as `%0D%0A`-style escapes in `filename*`; `\` and `"` stay escaped as quoted-pairs.

[Unreleased]: https://github.com/poli-page/rocketrs/compare/v0.11.0...HEAD
[0.11.0]: https://github.com/poli-page/rocketrs/releases/tag/v0.11.0
