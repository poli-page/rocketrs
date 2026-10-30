# poli-page-rocket

> Render Poli Page documents as Rocket responders.

[![CI](https://github.com/poli-page/rocketrs/actions/workflows/ci.yml/badge.svg)](https://github.com/poli-page/rocketrs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/poli-page-rocket.svg)](https://crates.io/crates/poli-page-rocket)
[![docs.rs](https://img.shields.io/docsrs/poli-page-rocket)](https://docs.rs/poli-page-rocket)
[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

## About

You attach a fairing, the SDK client lands in Rocket's managed state, and your routes return `Responder` types that carry the conventional PDF, preview, and redirect headers. The crate is a thin veneer over the official [`poli-page`] SDK — it does not reimplement transport, retries, or error classification.

**When to use this:**

- You write Rocket 0.5 routes and want to return PDFs without hand-rolling `Content-Disposition` or `Cache-Control`.
- You want SDK retry and error events bridged to `tracing` under a known target.
- You want the `?` operator in routes to produce JSON error responses with the wire status, through an error handler that is on by default.

**When not to:**

- You need a global Rocket catcher that swallows every SDK error type from every route — opt-in `Result<T, PoliPageError>` is the supported shape.
- You target Rocket 0.4 or earlier.

## Requirements

- Rust `1.93` or newer (MSRV)
- Rocket `0.5`
- [`poli-page`] `0.11.0` or newer within `0.11` (environment keys, no template versions)

## Install

```bash
cargo add poli-page-rocket poli-page@0.11.0 rocket
```

Set the API key before booting Rocket. Grab one from the [Poli Page dashboard](https://app.poli.page/settings/api-keys). A key is `pp_{environment}_{32 lowercase hex}` and belongs to one environment of your organization, which decides how its renders are treated ("Counted" means the render counts toward the organization's monthly PDF quota):

| Environment                           | Key            | Watermark | Counted |
| ------------------------------------- | -------------- | --------- | ------- |
| `sandbox` (every org)                 | `pp_sandbox_…` | yes       | no      |
| `live` (every org, deletable on Pro+) | `pp_live_…`    | no        | yes     |
| created on Pro+ (e.g. `staging`)      | `pp_staging_…` | no        | yes     |

```bash
# .env (loaded by your host application, not by this crate)
POLI_PAGE_API_KEY=pp_sandbox_xxx
```

The fairing only requires the key to be set and non-empty: the Poli Page API is the only authority on keys. A key that does not match `pp_{environment}_{32 lowercase hex}` logs one warning under the `poli_page_rocket` target (it names the expected format and shows no part of the key, not even its `pp_{environment}_` part), and Rocket ignites anyway, letting the API answer (`401 INVALID_API_KEY`, or `401 MISSING_API_KEY` for a value without `pp_`).

The crate reads from `std::env` only. The example app loads a `.env` file via [`dotenvy`](https://crates.io/crates/dotenvy); your application is free to do the same.

## Quick start

```rust
// src/main.rs
use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError, PoliPageFairing};
use rocket::{get, routes, State};
use serde_json::json;

#[get("/welcome.pdf")]
async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let bytes = client.render().pdf(poli_page::RenderInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        data: json!({ "name": "World" }),
        ..Default::default()
    }).await?;
    Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .attach(PoliPageFairing::from_env())
        .mount("/", routes![welcome])
}
```

## Configuration

`PoliPageFairing::from_env()` reads the variables below. To configure programmatically, pass a fully-built `poli_page::PoliPageBuilder` to `PoliPageFairing::new(builder)`, or wrap an existing client with `PoliPageFairing::with_client(client)`.

Every constructor also turns on the [error handler](#error-handler); chain `.error_handler(false)` to turn it off.

| Variable | Purpose | Default |
|---|---|---|
| `POLI_PAGE_API_KEY` | API key, `pp_{environment}_{32 lowercase hex}` (required and non-empty; e.g. `pp_sandbox_…`) | — |
| `POLI_PAGE_BASE_URL` | Override the API base URL | `https://api.poli.page` |
| `POLI_PAGE_TIMEOUT_SECS` | Per-attempt timeout, in seconds | `60` |
| `POLI_PAGE_MAX_RETRIES` | Retry budget for `5xx` and `429` (never `4xx`) | `2` |
| `POLI_PAGE_RETRY_DELAY_MS` | Initial retry delay, in milliseconds (exponential backoff) | `500` |

```rust
// src/main.rs
let builder = poli_page::PoliPage::builder()
    .api_key("pp_live_...")
    .timeout(std::time::Duration::from_secs(30));

rocket::build().attach(PoliPageFairing::new(builder))
```

## API at a glance

| Symbol | Purpose |
|---|---|
| `PoliPageFairing` | Rocket fairing; builds the SDK client at ignite and inserts it into managed state. |
| `PoliPageClient` | Newtype wrapping `poli_page::PoliPage`; resolved via `&State<PoliPageClient>`. |
| `PoliPage<'_>` | Optional request guard that yields `&poli_page::PoliPage`. |
| `PdfResponse` | `Responder` for PDF bytes or chunked streams, with RFC 5987 `Content-Disposition`. |
| `PreviewResponse` | `Responder` for the stored HTML of a rendered document (`documents.preview`), or any HTML `String`. |
| `DocumentRedirect` | `Responder` that issues a 302 (or 308 via `.permanent()`) to a presigned URL. |
| `PoliPageError` | Newtype around `poli_page::Error`; its `Responder` impl is the error handler. |

Full reference: [docs/api.md](docs/api.md) (forthcoming).

## Error handler

Routes return `Result<T, PoliPageError>`, and `?` converts the SDK's `poli_page::Error`. The `PoliPageError` responder is the crate's error handler, and it is **on by default** as soon as the fairing is attached. It answers:

- **Status:** the `status` of the SDK's `Error::to_payload()`: the API's status for an API error, `503` for `network_error`, `504` for `timeout`, or `500` when the payload has no status (for example `invalid_options` or `INTERNAL_ERROR` with no response).
- **Body:** exactly `to_payload()` as JSON, `{ "code", "message", "status", "requestId" }`. `status` and `requestId` are `null` when unknown.
- **Headers:** `Content-Type: application/json; charset=utf-8` and `Cache-Control: private, no-store`.

```json
{
  "code": "TEMPLATE_NOT_FOUND",
  "message": "Template 'does-not-exist' not found in project 'getting-started'.",
  "status": 404,
  "requestId": "req_123"
}
```

```rust
// src/routes.rs
use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError};
use rocket::{get, State};

#[get("/invoice.pdf")]
async fn invoice(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let bytes = client.render().pdf(input).await?; // ? converts poli_page::Error
    Ok(PdfResponse::bytes(bytes).filename("invoice.pdf"))
}
```

### Turning it off

```rust
rocket::build().attach(PoliPageFairing::from_env().error_handler(false))
```

With the handler off, the responder writes no body: it hands the same status to Rocket's catchers, so your own `#[catch(...)]` handlers (or Rocket's default HTML pages) answer. To build your own body instead, match on the `poli_page::Error` in the route.

## Example app

A self-contained Rocket 0.5 binary lives in [`example-app/`](example-app/). It covers the SDK demo steps through nine HTTP routes plus an interactive HTML dashboard at `GET /`, and ships a standalone `render_to_file` binary for SDK demo step 3.

```bash
cd example-app
POLI_PAGE_API_KEY=pp_sandbox_... cargo run --bin example-app
# open http://localhost:8000
```

## Going further

- [Streaming](docs/streaming.md) — forward `PdfResponse::stream` from `client.render().pdf_stream(...)` with `Transfer-Encoding: chunked` (forthcoming).
- [Observability](docs/observability.md) — bridge SDK `on_retry` and `on_error` events to `tracing` under the `poli_page_rocket` target (forthcoming).
- [API reference](docs/api.md) — every public symbol with full type signatures and header guarantees (forthcoming).

## Compatibility

| `poli-page-rocket` | Rocket | Rust (MSRV) | `poli-page` SDK |
|---|---|---|---|
| unreleased | `0.5` | `1.93` | `0.11.0` |
| `0.1.x` | `0.5` | `1.93` | `0.9.0` |

Rocket `0.5` is the only stable line at the time of writing. MSRV is bumped at most once per minor release.

## Contributing

Run the checks CI runs, in the crate and in `example-app/`:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo doc --no-deps
cd example-app && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo build --bins && cargo test
```

### Contract tests

`example-app/tests/contract.rs` runs the `impl` cases of the [Poli Page API contract](https://github.com/poli-page/api-contract) against the example app: for each case, a local mock server answers as the API would, the app's SDK points at it, and the test compares the app's answer and the SDK's requests with the case. CI runs them against the pinned contract version (the `poli-page/api-contract` action in `.github/workflows/ci.yml`). Locally:

```bash
cd example-app
POLI_CONTRACT=<path to contract.json> cargo test --test contract
```

Without `POLI_CONTRACT` the suite is skipped (and fails when `CI` is set). `example-app/tests/contract-skip.json` lists the cases allowed to fail, each with its reason.

## License

Dual-licensed under MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache-2.0 ([LICENSE-APACHE](LICENSE-APACHE)) at your option.

[`poli-page`]: https://crates.io/crates/poli-page
