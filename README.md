# poli-page-rocket

[![CI](https://github.com/poli-page/rocketrs/actions/workflows/ci.yml/badge.svg)](https://github.com/poli-page/rocketrs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/poli-page-rocket.svg)](https://crates.io/crates/poli-page-rocket)
[![docs.rs](https://img.shields.io/docsrs/poli-page-rocket)](https://docs.rs/poli-page-rocket)
[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Rocket.rs fairing and responders for the [Poli Page] PDF rendering API. A thin idiomatic veneer over the official [`poli-page`] SDK — managed-state DI, `Responder` impls with correct headers, opt-in typed JSON error mapping.

## Install

```bash
cargo add poli-page-rocket poli-page rocket
```

MSRV: Rust `1.75`. Tracks Rocket `0.5`.

## Quick start

```rust
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

## The three primitives

### `PoliPageFairing`

Builds the SDK client at ignite-time and inserts it into Rocket's managed state. Three constructors:

```rust
// Read everything from environment variables.
PoliPageFairing::from_env();

// Pass a fully-configured builder (custom reqwest client, hooks, etc.).
let builder = poli_page::PoliPage::builder()
    .api_key("pp_live_...")
    .timeout(std::time::Duration::from_secs(30));
PoliPageFairing::new(builder);

// Wrap an already-built client (e.g. one shared with non-Rocket code).
PoliPageFairing::with_client(client);
```

Ignite fails with a logged `tracing::error!` when the API key is missing or malformed.

### `PoliPageClient` and the `PoliPage<'_>` guard

The canonical Rocket pattern — pull the client out of managed state:

```rust
#[get("/welcome.pdf")]
async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let bytes = client.render().pdf(input).await?;
    Ok(PdfResponse::bytes(bytes))
}
```

Or the optional sugar — one character less typing:

```rust
use poli_page_rocket::PoliPage;

#[get("/welcome.pdf")]
async fn welcome(client: PoliPage<'_>) -> Result<PdfResponse, PoliPageError> {
    let bytes = client.0.render.pdf(input).await?;
    Ok(PdfResponse::bytes(bytes))
}
```

`PoliPageClient` is `Clone + Send + Sync + 'static` — the SDK's `PoliPage` is `Arc`-internal, so cloning is an atomic refcount bump.

### Response types

```rust
// PDF, fully buffered. Sets Content-Type, RFC 5987 Content-Disposition,
// Cache-Control: private, no-store, X-Content-Type-Options: nosniff.
PdfResponse::bytes(bytes).filename("invoice.pdf").inline();

// PDF, streamed chunk by chunk (no Content-Length, transfer-encoding: chunked).
PdfResponse::stream(client.render().pdf_stream(input).await?);

// HTML preview from render.preview or documents.preview.
PreviewResponse::from(client.render().preview(input).await?);

// 302 (default) or 308 (.permanent()) redirect to a presigned URL.
DocumentRedirect::to(&descriptor.presigned_pdf_url);
DocumentRedirect::from(&descriptor); // shorthand
```

## Environment variables

| Var | Purpose | Default |
|---|---|---|
| `POLI_PAGE_API_KEY` | API key (required, must start with `pp_test_` or `pp_live_`) | — |
| `POLI_PAGE_BASE_URL` | Override base URL | SDK default (`https://api.poli.page`) |
| `POLI_PAGE_TIMEOUT_SECS` | Per-attempt timeout | SDK default (60s) |
| `POLI_PAGE_MAX_RETRIES` | Retry budget (5xx + 429 only — 4xx never retried) | SDK default (2) |
| `POLI_PAGE_RETRY_DELAY_MS` | Initial retry delay (exponential backoff) | SDK default (500ms) |

The library reads from `std::env` only — loading `.env` files is the host application's responsibility. The example app uses [`dotenvy`](https://crates.io/crates/dotenvy).

## Error handling

Routes returning `Result<T, PoliPageError>` get a typed JSON error response for free:

```json
{
  "code": "INVALID_VERSION_FORMAT",
  "message": "bad request (400): Version selector must be 'draft' or an exact semver.",
  "requestId": "req_abc123"
}
```

Plus `Content-Type: application/json; charset=utf-8` and `Cache-Control: private, no-store`. Status mapping:

| Error variant | HTTP status |
|---|---|
| `BadRequest` (400 / 422), `Auth` (401), `PermissionDenied` (403), `NotFound` (404), `Gone` (410), `RateLimited` (429), `Api` | wire status (pass-through) |
| `Connection`, `Download` | 502 Bad Gateway |
| `Timeout` | 504 Gateway Timeout |
| `Aborted` | 503 Service Unavailable |
| `InvalidOptions`, `Internal` | 500 Internal Server Error |

`PoliPageError` is a newtype wrapper around `poli_page::Error` — Rust's orphan rule blocks `impl Responder for poli_page::Error` directly. `From<poli_page::Error>` is implemented so the `?` operator does the conversion at route boundaries. Other error types bubble to Rocket's default 500 catcher; no global catch-all is registered.

## Streaming

For large PDFs, stream the SDK's chunked response straight through:

```rust
#[get("/large-report.pdf")]
async fn report(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let stream = client.render().pdf_stream(input).await?;
    Ok(PdfResponse::stream(stream).filename("report.pdf"))
}
```

Rocket emits `Transfer-Encoding: chunked` and forwards bytes to the client as the SDK yields them.

## Observability

Default `on_retry` / `on_error` hooks emit structured `tracing` events under the target `poli_page_rocket`:

```text
WARN poli_page_rocket: poli_page retry attempt=2 delay_ms=500 code="INTERNAL_ERROR" status=503 ...
ERROR poli_page_rocket: poli_page terminal error code="timeout" message="request timed out after 60s" ...
```

Override the bridge by passing a custom builder to `PoliPageFairing::new(...)` with your own `.on_retry(...)` / `.on_error(...)` closures.

## Example app

The `example-app/` directory ships a self-contained Rocket binary covering all 10 SDK demo steps with an interactive HTML dashboard at `GET /`:

```bash
cd example-app
POLI_PAGE_API_KEY=pp_test_... cargo run --bin example-app
open http://localhost:8000
```

`cargo run --bin render_to_file` runs SDK demo step 3 (write a PDF to `/tmp`) as a standalone binary.

## Contributing

See [`CLAUDE.md`](./CLAUDE.md). TL;DR: TDD is mandatory, no `.unwrap()` in `src/`, conventional commit messages.

## License

MIT OR Apache-2.0 (at your option). See [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE).

[Poli Page]: https://poli.page
[`poli-page`]: https://crates.io/crates/poli-page
