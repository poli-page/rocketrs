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
- You want the `?` operator in routes to produce typed JSON error responses with the wire status.

**When not to:**

- You need a global Rocket catcher that swallows every SDK error type from every route — opt-in `Result<T, PoliPageError>` is the supported shape.
- You target Rocket 0.4 or earlier.

## Requirements

- Rust `1.75` or newer (MSRV)
- Rocket `0.5`
- [`poli-page`] `1.0.0-rc.1` (re-exported; you do not depend on it directly)

## Install

```bash
cargo add poli-page-rocket poli-page rocket
```

Set the API key before booting Rocket. Grab one from the [Poli Page dashboard](https://app.poli.page/settings/api-keys); it must start with `pp_test_` or `pp_live_`.

```bash
# .env (loaded by your host application, not by this crate)
POLI_PAGE_API_KEY=pp_test_xxx
```

The crate reads from `std::env` only. The example app loads a `.env` file via [`dotenvy`](https://crates.io/crates/dotenvy); your application is free to do the same.

## Quick start

```rust
// src/main.rs
use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError, PoliPageFairing};
use rocket::{get, routes, State};
use serde_json::json;

#[get("/welcome.pdf")]
async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let bytes = client.render().pdf(poli_page::ProjectModeInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        version: Some("1.0.0".into()),
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

| Variable | Purpose | Default |
|---|---|---|
| `POLI_PAGE_API_KEY` | API key (required, prefix `pp_test_` or `pp_live_`) | — |
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
| `PreviewResponse` | `Responder` for HTML preview output from `render.preview` or `documents.preview`. |
| `DocumentRedirect` | `Responder` that issues a 302 (or 308 via `.permanent()`) to a presigned URL. |
| `PoliPageError` | Newtype around `poli_page::Error` with the `Responder` impl. |

Full reference: [docs/api.md](docs/api.md) (forthcoming).

## Errors

Routes returning `Result<T, PoliPageError>` get a typed JSON body `{ code, message, requestId }` with `Content-Type: application/json; charset=utf-8` and `Cache-Control: private, no-store`. The SDK exposes more granular variants than the four-category taxonomy; this crate maps each one to the appropriate HTTP status.

### Variants

- **Auth** — `Error::Auth` (401), `Error::PermissionDenied` (403). Wire status passes through.
- **Rate limit** — `Error::RateLimited` (429). Wire status passes through.
- **Request rejected** — `Error::BadRequest` (400/422), `Error::NotFound` (404), `Error::Gone` (410), `Error::Api` (any other 4xx/5xx). Wire status passes through.
- **Network / transport** — `Error::Connection` and `Error::Download` map to 502; `Error::Timeout` maps to 504; `Error::Aborted` maps to 503; `Error::InvalidOptions` and `Error::Internal` map to 500.

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

## Example app

A self-contained Rocket 0.5 binary lives in [`example-app/`](example-app/). It covers all 10 SDK demo steps through nine HTTP routes plus an interactive HTML dashboard at `GET /`, and ships a standalone `render_to_file` binary for SDK demo step 3.

```bash
cd example-app
POLI_PAGE_API_KEY=pp_test_... cargo run --bin example-app
# open http://localhost:8000
```

## Going further

- [Streaming](docs/streaming.md) — forward `PdfResponse::stream` from `client.render().pdf_stream(...)` with `Transfer-Encoding: chunked` (forthcoming).
- [Observability](docs/observability.md) — bridge SDK `on_retry` and `on_error` events to `tracing` under the `poli_page_rocket` target (forthcoming).
- [API reference](docs/api.md) — every public symbol with full type signatures and header guarantees (forthcoming).

## Compatibility

| `poli-page-rocket` | Rocket | Rust (MSRV) | `poli-page` SDK |
|---|---|---|---|
| `0.1.x` | `0.5` | `1.75` | `1.0.0-rc.1` |

Rocket `0.5` is the only stable line at the time of writing. MSRV is bumped at most once per minor release.

## Contributing

See [CLAUDE.md](CLAUDE.md) (a `CONTRIBUTING.md` will replace it in a future release).

## License

Dual-licensed under MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache-2.0 ([LICENSE-APACHE](LICENSE-APACHE)) at your option.

[`poli-page`]: https://crates.io/crates/poli-page
