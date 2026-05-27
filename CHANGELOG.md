# Changelog

All notable changes to `poli-page-rocket` are documented here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-05-27

### Added
- `PoliPageFairing` with three constructors — `from_env`, `new(builder)`, `with_client` — that builds the SDK client at ignite-time and inserts it into Rocket's managed state. Ignite fails cleanly when `POLI_PAGE_API_KEY` is missing or malformed.
- Env-var contract: `POLI_PAGE_API_KEY` (required, `pp_test_` / `pp_live_` prefix) plus optional `POLI_PAGE_BASE_URL`, `POLI_PAGE_TIMEOUT_SECS`, `POLI_PAGE_MAX_RETRIES`, `POLI_PAGE_RETRY_DELAY_MS`.
- `PoliPageClient` newtype (`Clone + Send + Sync + 'static`) for managed-state extraction via `&State<PoliPageClient>`, plus optional `PoliPage<'_>` request guard for one-character-less ergonomics.
- Response types — `PdfResponse` (bytes + stream), `PreviewResponse`, `DocumentRedirect` — with RFC 5987 filename encoding for non-ASCII names. Default headers: `Cache-Control: private, no-store` and `X-Content-Type-Options: nosniff`. `Cache-Control` is overridable via `.cache_control(...)`.
- `PoliPageError` newtype wrapper around `poli_page::Error` with `Responder` and `From` impls — routes return `Result<T, PoliPageError>` and `?` runs the conversion. Status mapping: 4xx pass-through, 5xx pass-through, `Connection` / `Download` → 502, `Timeout` → 504, `Aborted` → 503, `InvalidOptions` / `Internal` → 500. Body is JSON `{ code, message, requestId }`.
- Default `on_retry` / `on_error` hooks bridge SDK events to `tracing` under the `poli_page_rocket` target. Overridable via a custom `PoliPageBuilder` passed to `PoliPageFairing::new(...)`.
- SDK re-exports at the crate root (`Error`, `RetryEvent`, `ProjectModeInput`, `InlineModeInput`, `RenderInput`, `DocumentDescriptor`, `DocumentPreviewResult`, `PreviewResult`, `ThumbnailFormat`, `ThumbnailOptions`, `PoliPage as SdkPoliPage`) so user code typically only imports from `poli_page_rocket::*`.
- Example Rocket 0.5 app at `example-app/` covering all 10 SDK demo steps, with an interactive HTML dashboard at `GET /` and a standalone `render_to_file` binary for SDK demo step 3.

[Unreleased]: https://github.com/poli-page/rocketrs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/poli-page/rocketrs/releases/tag/v0.1.0
