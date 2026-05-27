# `poli-page-rocket` — implementation specification

> Source of truth for what we build, in what shape, and explicitly what we don't. Mirrors `symfony-bundle/docs/spec/bundle-specification.md`'s section structure so reviewers can cross-reference across integrations. Read `INTEGRATIONS_PLAN.md` first; this is the per-repo expansion of the Rocket.rs slot (a gap-fill — the Rust SDK didn't exist when the integrations plan was drafted).

**Roadmap slot**: gap-fill, post-`sdk-rust`.
**Target**: ship v0.1.0 as a working `crates.io` crate, not a recipe in the SDK examples.
**Stance**: thin idiomatic Rocket veneer over the `poli-page` SDK. Anything the SDK already does — HTTP, retries, error classification, idempotency, presigned-URL fetching, streaming — does NOT get reimplemented here.

---

## 1. What this crate is, and what it isn't

### Is
- A Rocket-native wrapper around `poli-page` that gives Rocket 0.5+ users:
  - A **`PoliPageFairing`** that builds the SDK client at ignite-time and attaches it to Rocket's managed state.
  - A `PoliPageClient` newtype in managed state, extractable in routes via the standard `&State<PoliPageClient>` parameter or the sugar `PoliPage<'_>` request guard.
  - Three `Responder<'r, 'static>` types — `PdfResponse`, `PreviewResponse`, `DocumentRedirect` — with correct headers including RFC 5987 filename encoding.
  - A `Responder` impl for the SDK's `poli_page::Error` so routes return `Result<PdfResponse, poli_page::Error>` and `?` does all the work.
  - Bridging of SDK retry / error hooks into structured `tracing` events under the `poli_page_rocket` target.
  - A small example app at `example-app/` with the same interactive demo-UI pattern shipped in symfony-bundle, nextjs, and nestjs.

### Isn't
- A reimplementation of SDK behaviour. Tests do not cover transport, retries, 4xx mapping, idempotency, or stream chunking — `poli-page`'s test suite (`tests/`) owns those.
- A blocking-feature crate. The SDK has a `blocking` feature for sync callers, but Rocket 0.5 is async-only; we don't surface a blocking wrapper. Users wanting sync access can call into the SDK's `blocking` module directly.
- A `rocket_contrib`-style swiss-army knife. No template helper, no database integration, no fairings beyond `PoliPageFairing`, no Swagger generation.
- A global catcher. Rocket's `#[catch]` system is per-status-code, not per-error-type, so we cannot auto-map `poli_page::Error` the way NestJS's exception filter does. Instead, the SDK's error type implements `Responder` — users opt in by returning `Result<PdfResponse, poli_page::Error>`. See §10 and `CLAUDE.md` §10.3.
- A custom CLI. Rocket's `cargo run` is the launcher; the example app's `cargo run --bin example-app` IS the smoke test.

---

## 2. Required reading (concrete file paths)

Before touching code, read in this order:

1. `/Users/mickael/Projects/INTEGRATIONS_PLAN.md` — cross-repo plan, scope verdicts, cross-cutting DX patterns (§"Cross-cutting DX patterns" is the most relevant section). Note: the original plan did not include Rocket because `sdk-rust` didn't exist; this spec closes that gap with the same DX rules.
2. `CLAUDE.md` (this repo, §10 "Known gotchas") — five battle-tested constraints that flow into the design below.
3. `/Users/mickael/Projects/symfony-bundle/docs/spec/bundle-specification.md` — reference structure; this doc mirrors its section numbering where the concept transfers.
4. `/Users/mickael/Projects/nextjs/docs/spec/nextjs-implementation.md` §18 ("Resolved decisions") — the nine cross-cutting decisions carried into this spec.
5. `/Users/mickael/Projects/nestjs/CLAUDE.md` and its spec — the closest conceptual match for "DI via framework state + opt-in error mapping".
6. `/Users/mickael/Projects/sdk-rust/Cargo.toml` and `src/lib.rs` — the SDK crate this wraps. Verify the actual exported types (`PoliPage`, `PoliPageBuilder`, `Error`, `RetryEvent`, `Render`, `Documents`, `ProjectModeInput`, `InlineModeInput`, `RenderInput`, `DocumentDescriptor`, `PreviewResult`, `DocumentPreviewResult`, `Thumbnail`, `ThumbnailOptions`, `render_to_file`).
7. `/Users/mickael/Projects/sdk-rust/examples/demo.rs` — the 10-step canonical demo; `example-app/` mirrors this 1:1 (§14).
8. Rocket 0.5 guide: <https://rocket.rs/guide/v0.5/> — Fairings, State, Responders, and Local Client chapters.
9. `sentry-rocket` source on GitHub — primary industry reference for "third-party SDK + Rocket fairing" shape.

---

## 3. Version targets

| Field | Value |
|---|---|
| Crate name (publish) | `poli-page-rocket` |
| Library name | `poli_page_rocket` |
| Initial version | `0.1.0` |
| Rust edition | `2021` |
| MSRV | `1.75` (Rocket 0.5's minimum) |
| Rocket | `^0.5` (only stable line as of 2026-05) |
| `poli-page` SDK | `^1.0.0-rc.1` (pinned until SDK stabilises) |
| Tokio | inherited from Rocket; no direct dep beyond `rt` / `macros` for tests |
| License | `MIT OR Apache-2.0` (matches the SDK; required for crates.io ecosystem fit) |
| Default features | none beyond the SDK's `rustls-tls` (which the SDK enables by default) |

CI matrix: Rust `stable` × `beta` × `1.75` (MSRV) on `ubuntu-latest`. See §15.

---

## 4. Architecture style

Three primitives:

1. **`PoliPageFairing`** — a `rocket::fairing::Fairing` that on ignite reads config, builds the SDK client via `PoliPage::builder().<...>.build()?`, wraps it in `PoliPageClient`, and attaches it to managed state via `rocket.manage(client)`. Failure aborts ignite with a `tracing::error!`.
2. **`PoliPageClient`** — a `Clone + Send + Sync + 'static` newtype wrapping `poli_page::PoliPage`. Cheap-to-clone (the SDK's `PoliPage` is `Arc`-internal), so `&State<PoliPageClient>` is sufficient — routes can `client.0.clone()` if they need an owned handle for spawned tasks.
3. **Three response types + one error `Responder`**:
   - `PdfResponse { body: Bytes | PdfByteStream, filename: Option<String>, inline: bool, cache_control: Option<String> }`
   - `PreviewResponse { html: String, cache_control: Option<String> }`
   - `DocumentRedirect { url: String, permanent: bool }`
   - `impl<'r> Responder<'r, 'static> for poli_page::Error` — produces a typed JSON `Response` (status mapped per spec §10).

That is the entire public surface. No DI macros, no proc-macro derives, no `attach()` builder beyond `rocket::build().attach(PoliPageFairing::from_env())`.

The crate is **tree-friendly**: each public item lives in its own module so future feature gates (e.g. a `metrics` feature) can be added without churn.

---

## 5. File layout

```
rocketrs/
├── src/
│   ├── lib.rs                              # re-exports the public API
│   ├── fairing.rs                          # PoliPageFairing
│   ├── state.rs                            # PoliPageClient newtype + PoliPage<'_> request guard
│   ├── responses/
│   │   ├── mod.rs                          # re-exports
│   │   ├── pdf.rs                          # PdfResponse + Responder impl
│   │   ├── preview.rs                      # PreviewResponse
│   │   └── redirect.rs                     # DocumentRedirect
│   ├── headers.rs                          # internal: RFC 5987 filename encoding (pure fns)
│   ├── errors.rs                           # Responder impl for poli_page::Error
│   └── tracing_bridge.rs                   # internal: builds the on_retry / on_error closures
├── tests/
│   ├── unit_headers.rs                     # RFC 5987 cases (ASCII + non-ASCII)
│   ├── unit_pdf_response.rs                # PdfResponse Responder shape
│   ├── unit_preview_response.rs            # PreviewResponse Responder shape
│   ├── unit_redirect_response.rs           # DocumentRedirect Responder shape
│   ├── unit_error_responder.rs             # poli_page::Error → status map
│   ├── unit_tracing_bridge.rs              # on_retry / on_error emit the right events
│   ├── fairing_state.rs                    # boot Rocket, assert PoliPageClient is in state
│   ├── fairing_invalid_config.rs           # missing key fails ignite cleanly
│   └── integration_render.rs               # #[ignore] real-API smoke test
├── example-app/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                         # rocket::build().attach(...).launch()
│   │   ├── bin/
│   │   │   └── render_to_file.rs           # SDK demo step 3 (standalone binary)
│   │   ├── routes/
│   │   │   ├── mod.rs
│   │   │   ├── demo.rs                     # GET /
│   │   │   ├── render.rs                   # /render/pdf, /render/stream, /render/preview
│   │   │   ├── documents.rs                # POST /documents, GET|DELETE /documents/<id>, …
│   │   │   └── errors.rs                   # GET /errors/bad-version
│   ├── static/
│   │   └── index.html                      # the interactive demo dashboard (port of demo.html)
│   └── README.md                           # `cargo run --bin example-app` → http://localhost:8000
├── docs/
│   ├── spec/rocket-crate-specification.md       # this file
│   └── plan/2026-05-27-implementation.md        # step-by-step plan
├── Cargo.toml
├── README.md
├── CHANGELOG.md
├── CLAUDE.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── .github/workflows/ci.yml
├── rust-toolchain.toml                     # `stable`, mirrors sdk-rust
├── rustfmt.toml                            # match sdk-rust
├── clippy.toml                             # match sdk-rust
└── .gitignore
```

**File count**: 7 source modules (`lib`, `fairing`, `state`, `responses/{pdf, preview, redirect}`, `headers`, `errors`, `tracing_bridge`), 9 test files (1:1 with src + fairing tests + integration), plus example app. Adding files beyond this list requires editing §17 first.

---

## 6. Fairing options + env-var contract

(Replaces symfony §6 "Configuration tree".)

### 6.1 `PoliPageFairing` constructors

Three ergonomic entry points, each returning the same fairing:

```rust
pub struct PoliPageFairing { /* private */ }

impl PoliPageFairing {
    /// Read all options from environment variables (see §6.3).
    /// Fails ignite if `POLI_PAGE_API_KEY` is missing or malformed.
    pub fn from_env() -> Self { /* defers validation to ignite */ }

    /// Build a fairing from an already-configured `PoliPageBuilder`.
    /// Use this when the host application owns the builder (custom
    /// `reqwest::Client`, hooks, etc.).
    pub fn new(builder: poli_page::PoliPageBuilder) -> Self { /* ... */ }

    /// Wrap an already-built client (e.g. one shared with non-Rocket
    /// code paths). The fairing simply inserts it into state.
    pub fn with_client(client: poli_page::PoliPage) -> Self { /* ... */ }
}
```

### 6.2 Internal config struct

`PoliPageFairing` holds an `enum Source { Env, Builder(Box<PoliPageBuilder>), Built(PoliPage) }`. On ignite (`Fairing::on_ignite`), the variant is resolved into a `PoliPage` and the result inserted into managed state via `Rocket::manage`.

The `Box` on `Builder` keeps the fairing struct small (the builder is `~200` bytes; the box trims the enum to one word + tag). Optimisation note, not user-facing.

### 6.3 Environment variable contract

`PoliPageFairing::from_env()` reads:

| Var | SDK builder method | Default if unset |
|---|---|---|
| `POLI_PAGE_API_KEY` | `.api_key(...)` | **required**; missing → ignite failure |
| `POLI_PAGE_BASE_URL` | `.base_url(...)` | SDK default (`https://api.poli.page`) |
| `POLI_PAGE_TIMEOUT_SECS` | `.timeout(Duration::from_secs(_))` | SDK default (60s) |
| `POLI_PAGE_MAX_RETRIES` | `.max_retries(_)` | SDK default (2) |
| `POLI_PAGE_RETRY_DELAY_MS` | `.retry_delay(Duration::from_millis(_))` | SDK default (500ms) |

The fairing reads from `std::env::var(...)` directly; the example app uses `dotenvy::from_path_override("../.env")` in its `main.rs` to populate `std::env` from the workspace root `.env` before `rocket::build()` runs (see §13.3).

### 6.4 Validation

- `POLI_PAGE_API_KEY` must match `/^pp_(test|live)_/`. Same regex as every sibling integration. Error on ignite: `"POLI_PAGE_API_KEY must start with pp_test_ or pp_live_. Get one at https://app.poli.page/settings/api-keys."`
- `POLI_PAGE_TIMEOUT_SECS` parses as `u64`; out-of-range or non-numeric → ignite failure with the offending value echoed.
- Same for `POLI_PAGE_MAX_RETRIES` (`u32`) and `POLI_PAGE_RETRY_DELAY_MS` (`u64`).

### 6.5 No `.env` loading inside `src/`

The fairing reads `std::env` only. Loading `.env` files is the host application's responsibility (with `dotenvy` or any other mechanism). This keeps `src/` free of filesystem assumptions and matches what `axum`-flavored SDK integrations do.

---

## 7. State extraction & request guard

(Replaces symfony §7 "DI services & wiring".)

### 7.1 The `PoliPageClient` newtype

```rust
#[derive(Clone)]
pub struct PoliPageClient(pub poli_page::PoliPage);

impl PoliPageClient {
    pub fn client(&self) -> &poli_page::PoliPage { &self.0 }
    pub fn render(&self) -> &poli_page::Render { &self.0.render }
    pub fn documents(&self) -> &poli_page::Documents { &self.0.documents }
}
```

`Clone + Send + Sync + 'static` for managed-state eligibility (Rocket's `State` requires `Send + Sync`; `Arc<ClientInner>` inside `PoliPage` already gives us cheap clones). The `pub` tuple field is intentional — users wanting raw access (`client.0`) shouldn't have to fight us.

### 7.2 Standard state extraction

The canonical Rocket pattern is sufficient:

```rust
use rocket::State;
use poli_page_rocket::PoliPageClient;

#[get("/welcome.pdf")]
async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, poli_page::Error> {
    let bytes = client.render().pdf(input()).await?;
    Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
}
```

### 7.3 Optional sugar: `PoliPage<'r>` request guard

For users who'd rather not type `&State<PoliPageClient>`:

```rust
pub struct PoliPage<'r>(&'r poli_page::PoliPage);

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for PoliPage<'r> {
    type Error = std::convert::Infallible;
    async fn from_request(req: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        match req.rocket().state::<PoliPageClient>() {
            Some(client) => rocket::request::Outcome::Success(PoliPage(&client.0)),
            None => rocket::request::Outcome::Error((rocket::http::Status::InternalServerError, std::convert::Infallible::from(()))),
        }
    }
}
```

(Note: the `Error = Infallible` shape is the canonical Rocket idiom for "this guard only fails if the fairing isn't attached, which is a programmer error".) Documented as optional; the `&State<PoliPageClient>` form is the primary path.

---

## 8. `Responder` impls

(Replaces symfony §8 "`PoliPageResponseFactory`".)

### 8.1 `PdfResponse`

```rust
pub struct PdfResponse {
    body: PdfBody,
    filename: Option<String>,
    inline: bool,
    cache_control: Option<String>,
}

enum PdfBody {
    Bytes(bytes::Bytes),
    Stream(poli_page::client::PdfByteStream),
}

impl PdfResponse {
    pub fn bytes(body: bytes::Bytes) -> Self { /* ... */ }
    pub fn stream(stream: poli_page::client::PdfByteStream) -> Self { /* ... */ }
    pub fn filename(mut self, name: impl Into<String>) -> Self { /* ... */ }
    pub fn inline(mut self) -> Self { self.inline = true; self }
    pub fn cache_control(mut self, value: impl Into<String>) -> Self { /* ... */ }
}
```

Headers set by the `Responder<'r, 'static>` impl:
- `Content-Type: application/pdf`
- `Content-Length` set when the body is `Bytes` (omitted on streams; chunked transfer wins)
- `Content-Disposition` per RFC 5987 (see §11)
- `Cache-Control: private, no-store` by default; overridable via `.cache_control(...)`
- `X-Content-Type-Options: nosniff`

### 8.2 `PreviewResponse`

```rust
pub struct PreviewResponse {
    html: String,
    cache_control: Option<String>,
}

impl PreviewResponse {
    pub fn new(html: impl Into<String>) -> Self { /* ... */ }
    pub fn cache_control(mut self, value: impl Into<String>) -> Self { /* ... */ }
}
```

Headers:
- `Content-Type: text/html; charset=utf-8`
- `Cache-Control: private, no-store` (overridable)
- `X-Content-Type-Options: nosniff`

`From<poli_page::PreviewResult>` and `From<poli_page::DocumentPreviewResult>` implementations let routes return `PreviewResponse::from(client.render().preview(input).await?)` directly.

### 8.3 `DocumentRedirect`

```rust
pub struct DocumentRedirect {
    url: String,
    permanent: bool,
}

impl DocumentRedirect {
    pub fn to(url: impl Into<String>) -> Self { /* permanent: false */ }
    pub fn permanent(mut self) -> Self { self.permanent = true; self }
}
```

Headers:
- Status `302` (default) or `308` (when `.permanent()` is set)
- `Location: <url>`
- `Cache-Control: private, no-store`

`From<&poli_page::DocumentDescriptor> for DocumentRedirect` builds one from `descriptor.presigned_pdf_url`.

---

## 9. No CLI

(Replaces symfony §9 "`bin/console poli-page:render`".)

Rocket has no per-app CLI surface user code attaches to. The example app's `cargo run --bin example-app` IS the smoke test. The SDK's free function `poli_page::render_to_file(&client, input, path)` becomes a standalone binary at `example-app/src/bin/render_to_file.rs`, run via `cargo run --bin render_to_file`. This matches the next.js and nest.js stance documented in their respective `CLAUDE.md` §10.

The crate does NOT ship a `poli-page` subcommand or a `cargo` extension; doing so would add maintenance surface for one workflow that's already covered by the SDK's own `cargo run --example demo`.

---

## 10. Error `Responder` + tracing bridge

(Replaces symfony §10 "EventDispatcher integration".)

### 10.1 `impl Responder<'r, 'static> for poli_page::Error`

The SDK's `Error` enum maps to HTTP responses as follows:

| `poli_page::Error` variant | HTTP status | Notes |
|---|---|---|
| `BadRequest { status, .. }` | `status` (400 or 422) | body has the wire `code`. |
| `Auth` | `401` | |
| `PermissionDenied` | `403` | |
| `NotFound` | `404` | |
| `Gone` | `410` | |
| `RateLimited` | `429` | |
| `Api { status, .. }` | `status` (pass-through) | catch-all for 4xx/5xx outside the specific variants. |
| `Connection` | `502` | no upstream response; mapped to Bad Gateway. |
| `Timeout` | `504` | mapped to Gateway Timeout — semantic match per RFC 9110. |
| `Aborted` | `503` | service unavailable (request was cancelled). |
| `InvalidOptions` | `500` | programmer error; should surface during ignite, but defensive. |
| `Download` | `502` | second-hop presigned-URL failure. |
| `Internal` | `500` | |

Response body (JSON):

```json
{
  "code": "INVALID_VERSION_FORMAT",
  "message": "Version selector must be 'draft' or an exact semver.",
  "requestId": "req_abc123"
}
```

- `code` comes from `err.code()` (the SDK's `Error::code()` method — returns a fixed string for reserved variants and the wire `code` for API variants).
- `message` comes from the `Display` impl (`err.to_string()`).
- `requestId` comes from `err.request_id()` — `null` for reserved variants (no upstream response).

Response headers: `Content-Type: application/json; charset=utf-8`, `Cache-Control: private, no-store`.

### 10.2 Opt-in, not global

The `Responder` impl is opt-in: routes return `Result<PdfResponse, poli_page::Error>` and `?` does the conversion. There is no global Rocket `#[catch]` because catchers are per-status-code, not per-error-type — registering a catch-all `#[catch(default)]` would swallow every error type from every route in the application. See `CLAUDE.md` §10.3.

### 10.3 Tracing bridge (the SDK hooks)

The SDK exposes two `Fn` constructor hooks: `on_retry(impl Fn(&RetryEvent) + Send + Sync + 'static)` and `on_error(impl Fn(&Error) + Send + Sync + 'static)`. `PoliPageFairing::from_env()` and `PoliPageFairing::new(...)` install bridges that emit structured `tracing` events:

```rust
// on_retry
tracing::warn!(
    target: "poli_page_rocket",
    attempt = event.attempt,
    delay_ms = event.delay.as_millis() as u64,
    code = event.reason.code(),
    status = event.reason.status(),
    request_id = event.reason.request_id(),
    "poli_page retry"
);

// on_error
tracing::error!(
    target: "poli_page_rocket",
    code = err.code(),
    status = err.status(),
    request_id = err.request_id(),
    message = %err,
    "poli_page terminal error"
);
```

Users wanting a different bridge (Sentry, OpenTelemetry, custom logger) construct the fairing via `PoliPageFairing::new(builder)` with their own `.on_retry(...)` / `.on_error(...)` on the builder. The default bridge is therefore overridable, not mandatory.

### 10.4 No user-configurable hook field on the fairing

Unlike the Symfony bundle's `on_retry` / `on_error` config keys, we don't expose those as fairing options. Reason: in Rust, a closure capturing user data is most naturally constructed by the user at the call site, not at the framework's config layer. The `PoliPageFairing::new(builder)` path is the documented seam.

---

## 11. Header utilities (`src/headers.rs`)

Internal, not exported.

```rust
pub(crate) fn is_ascii_safe(s: &str) -> bool;
pub(crate) fn rfc5987_encode(s: &str) -> String;
pub(crate) fn content_disposition(filename: &str, inline: bool) -> String;
```

Behaviour:
- If `filename` is ASCII-safe (printable 0x20..=0x7E, no quotes/backslashes that need escaping): `attachment; filename="<escaped>"`.
- Otherwise: `attachment; filename="<ascii-fallback>"; filename*=UTF-8''<rfc5987>`.
- `inline: true` swaps `attachment` for `inline`.

ASCII fallback: replace non-ASCII with `_`. RFC 5987 encoding: percent-encode bytes outside the `attr-char` set (per RFC 8187, the modern reference for `filename*`).

Same algorithm as `PoliPageResponseFactory` in the symfony-bundle, ported character-for-character. The bundle's unit tests for this are the canonical reference; port the cases verbatim to `tests/unit_headers.rs`.

Pure Rust, no `unsafe`, no `unwrap`. Public surface: zero items — these are crate-private helpers consumed by `responses/pdf.rs`.

---

## 12. Unpublished-SDK workaround (dev only)

`poli-page` is at `1.0.0-rc.1` on crates.io as of 2026-05; it's installable directly. The "dev override" pattern is only relevant when we want to test against unreleased SDK changes locally.

### 12.1 Solution: `[patch.crates-io]` in `Cargo.toml`

Top-level `Cargo.toml` carries a clean dependency declaration:

```toml
[dependencies]
poli-page = "1.0.0-rc.1"
rocket    = "0.5"
tracing   = "0.1"
bytes     = "1.7"
```

A second block, **kept as long as the SDK is pre-stable**, points Cargo at the local sibling checkout:

```toml
[patch.crates-io]
poli-page = { path = "../sdk-rust" }
```

`[patch.crates-io]` is the Cargo-idiomatic way to override a dep without changing its version requirement. When the SDK publishes `1.0.0` stable and we're ready to track it:

1. Bump `poli-page = "1.0.0"` in `[dependencies]`.
2. Remove the `[patch.crates-io]` block.
3. `cargo update -p poli-page`.
4. Tag.

**The crate's source code (everything in `src/`, `tests/`) does not change.** The only changes are the manifest's version constraint and the deletion of the patch block.

### 12.2 Alternative: `.cargo/config.toml` `paths` directive

For developers who'd rather not commit the patch block:

```toml
# .cargo/config.toml
paths = ["../sdk-rust"]
```

This is honoured for `cargo build`/`cargo test` in this repo (and any workspace member) but not by downstream consumers. Equivalent behaviour, zero churn on the published manifest.

We document both options in the README; the in-repo default is the `[patch.crates-io]` block because it's discoverable from the manifest itself.

### 12.3 CI handling

`.github/workflows/ci.yml` checks out `sdk-rust` as a sibling directory before `cargo test`:

```yaml
- uses: actions/checkout@v4
  with:
    repository: poli-page/sdk-rust
    path: sdk-rust
- uses: actions/checkout@v4
  with:
    path: rocketrs
```

Both repos end up siblings under `$GITHUB_WORKSPACE`, exactly as on the maintainer's machine. The `[patch.crates-io]` block resolves `../sdk-rust` from inside `rocketrs/` correctly. After the SDK publishes stable, the checkout step is also removed alongside the patch block.

### 12.4 example-app workaround

`example-app/Cargo.toml` declares the crate via a path dependency (it's not a published artifact, so a clean manifest doesn't matter):

```toml
[dependencies]
poli-page-rocket = { path = ".." }
poli-page        = "1.0.0-rc.1"   # resolved via the workspace's [patch.crates-io]
rocket           = "0.5"
dotenvy          = "0.15"
tokio            = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Stays as-is forever — example-app installs from local sources, not crates.io.

---

## 13. Testing strategy

### 13.1 Layers

**Unit tests** (~90% of the suite, no network, millisecond-fast):

| Test file | What it covers |
|---|---|
| `unit_headers.rs` | `is_ascii_safe`, `rfc5987_encode`, `content_disposition` — ASCII, non-ASCII, embedded quotes, `inline: true` swap. Port cases verbatim from symfony-bundle's `PoliPageResponseFactoryTest`. |
| `unit_pdf_response.rs` | `PdfResponse` `Responder` produces the right `Content-Type` / `Content-Disposition` / `Cache-Control` / `X-Content-Type-Options`. Cover `Bytes` body and `PdfByteStream` body (mocked stream that yields one chunk and ends). |
| `unit_preview_response.rs` | `PreviewResponse` headers + body. `From<PreviewResult>` and `From<DocumentPreviewResult>` conversions. |
| `unit_redirect_response.rs` | 302 default, 308 on `.permanent()`, `Location` header, `Cache-Control`. |
| `unit_error_responder.rs` | Every `poli_page::Error` variant → expected status code + body shape. |
| `unit_tracing_bridge.rs` | Use `tracing_subscriber::fmt::with_test_writer()` to capture events; invoke the bridge closure with a fake `RetryEvent` / `Error` and assert the fields. |

**Fairing tests** (boot a Rocket instance, ~few-hundred-ms each):

| Test file | What it covers |
|---|---|
| `fairing_state.rs` | Set env vars, attach the fairing, dispatch a request to a route that takes `&State<PoliPageClient>`, assert success. |
| `fairing_invalid_config.rs` | Missing `POLI_PAGE_API_KEY` → ignite fails. Bad prefix → ignite fails. Out-of-range timeout → ignite fails. |

**Integration test** (single test, gated):

`integration_render.rs`:
- Marked `#[ignore]` so default `cargo test` doesn't run it.
- Top-of-test runtime check: `if std::env::var("POLI_PAGE_API_KEY").is_err() { return; }` (PR contributors without a key get a clean no-op when they do run `cargo test -- --ignored`).
- Boots Rocket with `PoliPageFairing::from_env()`, dispatches `GET /welcome.pdf`, asserts the response body's first 5 bytes are `%PDF-` and `Content-Type` is `application/pdf`.
- One test, idempotent, ~3 seconds when it runs.

### 13.2 What we explicitly do NOT test

- HTTP transport behaviour (reqwest / hyper / TLS edge cases).
- Retry policy (exponential backoff, max attempts, `Retry-After` parsing, jitter, never-retry-4xx).
- 4xx / 5xx → `Error` mapping inside the SDK.
- Idempotency-Key auto-generation.
- Stream chunking / `PdfByteStream` correctness.
- API contract drift — the SDK's contract tests own that.

If a bug in any of these appears, fix it in `sdk-rust`. **Do not reach for `wiremock` here.**

### 13.3 Tooling

- **Test runner**: Rust's built-in `#[test]` harness. No external framework.
- **Async tests**: `#[rocket::async_test]` for fairing/integration tests; `#[test]` for pure-function unit tests.
- **Local client**: `rocket::local::asynchronous::Client` exclusively (see `CLAUDE.md` §10.1).
- **Tracing assertions**: `tracing-subscriber` `with_test_writer` (per `sdk-rust`'s pattern in its own tests).
- **Lints**: `cargo clippy --all-targets -- -D warnings` runs in CI. The crate root sets `#![warn(clippy::pedantic, clippy::cargo)]`; specific allows live in `lib.rs` with one-line comments.

---

## 14. `example-app/` structure

A self-contained Rocket 0.5 binary that demonstrates every public method of the SDK through Rocket idioms. **Mirrors the SDK's `examples/demo.rs`** in feature coverage so a reader can put the two side-by-side and verify the crate adds shape, not behaviour.

### 14.1 Routes

| SDK demo step | example-app route | Method called |
|---|---|---|
| 1. `render.pdf` | `GET /render/pdf` | `client.render().pdf(input).await?` → `PdfResponse::bytes(...)` |
| 2. `render.pdf_stream` | `GET /render/stream` | `client.render().pdf_stream(input).await?` → `PdfResponse::stream(...)` |
| 3. `render_to_file` | `cargo run --bin render_to_file` | Standalone binary in `src/bin/render_to_file.rs`; not a route. |
| 4. `render.preview` | `GET /render/preview` | `client.render().preview(input).await?` → `PreviewResponse::from(...)` |
| 5. `render.document` | `POST /documents` | Returns descriptor as JSON via `rocket::serde::json::Json(descriptor)`. |
| 6. `documents.get(id)` | `GET /documents/<id>` | `DocumentRedirect::to(descriptor.presigned_pdf_url)` |
| 7. `documents.thumbnails(id)` | `GET /documents/<id>/thumbnails` | Returns thumbnails as JSON. |
| 8. `documents.preview(id)` | `GET /documents/<id>/preview` | `PreviewResponse::from(client.documents().preview(id).await?)` |
| 9. `documents.delete(id)` | `DELETE /documents/<id>` | Returns 204. |
| 10. Error handling | `GET /errors/bad-version` | Triggers `INVALID_VERSION_FORMAT`; the route returns `Result<_, poli_page::Error>` and the crate's `Responder` impl maps it. |

### 14.2 Interactive demo UI at `GET /`

`example-app/static/index.html` is the **same** interactive dashboard shipped by symfony-bundle (`templates/demo.html`). One button per SDK feature, inline `<iframe>` PDF previews, `<iframe srcdoc>` HTML previews, JSON pretty-print for the JSON-returning endpoints, document-lifecycle state machine in client-side JS.

Aesthetic copied verbatim: white surface, brand indigo `#4f5d99`, Manrope display sans + IBM Plex Sans body + JetBrains Mono code. The HTML file is `include_str!`'d into `main.rs` (or served via `FileServer::from(relative!("static"))` — preferred for hot-edit during development; `include_str!` for the published binary).

The dashboard's `fetch()` calls hit the routes above unchanged — the same JSON / PDF endpoints as the symfony / next / nest demos.

### 14.3 Env loading

`example-app/src/main.rs`'s `#[rocket::main]` async fn calls `dotenvy::from_path_override("../.env").ok()` **before** `rocket::build()`. The `from_path_override` variant means real shell env vars still win (12-factor), but the workspace root `.env` populates anything missing.

```rust
#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ = dotenvy::from_path_override("../.env");
    let _ = dotenvy::from_path_override(".env"); // per-app fallback, mostly unused
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("poli_page_rocket=info".parse().unwrap())
    ).init();
    rocket::build()
        .attach(poli_page_rocket::PoliPageFairing::from_env())
        .mount("/", routes![
            routes::demo::index,
            routes::render::pdf,
            routes::render::stream,
            routes::render::preview,
            routes::documents::create,
            routes::documents::get,
            routes::documents::delete,
            routes::documents::thumbnails,
            routes::documents::preview,
            routes::errors::bad_version,
        ])
        .launch()
        .await
        .map(|_| ())
}
```

(The `dotenvy::from_path_override` call is the only place in the codebase that touches the filesystem for config; the library crate itself reads only `std::env`.)

### 14.4 What example-app proves

- The fairing actually wires the client into managed state in a real Rocket app (not just `local::Client`).
- The PDF actually streams to a browser with the right headers (open in Chrome, see the PDF render).
- Every SDK surface is reachable through `&State<PoliPageClient>` without manual `PoliPage::new(...)` calls in route handlers.
- A reader who knows the SDK can read the route modules and immediately see the wrapping pattern.

---

## 15. CI

`.github/workflows/ci.yml`:

```yaml
name: CI
on:
  push:
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        rust: ['stable', 'beta', '1.75']
    steps:
      - uses: actions/checkout@v4
        with:
          path: rocketrs
      - uses: actions/checkout@v4
        with:
          repository: poli-page/sdk-rust
          path: sdk-rust
          ref: main
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rocketrs
      - name: Format
        working-directory: rocketrs
        run: cargo fmt --check
        if: hashFiles('rocketrs/Cargo.toml') != ''
      - name: Clippy
        working-directory: rocketrs
        run: cargo clippy --all-targets -- -D warnings
        if: hashFiles('rocketrs/Cargo.toml') != ''
      - name: Test
        working-directory: rocketrs
        run: cargo test
        if: hashFiles('rocketrs/Cargo.toml') != ''
      - name: Doc
        working-directory: rocketrs
        run: cargo doc --no-deps
        if: hashFiles('rocketrs/Cargo.toml') != ''

  integration:
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
        with:
          path: rocketrs
      - uses: actions/checkout@v4
        with:
          repository: poli-page/sdk-rust
          path: sdk-rust
          ref: main
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
      - name: Integration test against develop API
        working-directory: rocketrs
        env:
          POLI_PAGE_API_KEY: ${{ secrets.POLI_PAGE_DEVELOP_API_KEY }}
        run: cargo test -- --ignored
```

**Auto-skip behaviour** (inherited from the SDK CI convention): each step short-circuits if `Cargo.toml` is missing. A freshly scaffolded repo is green from day one.

When the SDK publishes stable, remove the "checkout sdk-rust" step alongside the `[patch.crates-io]` removal in §12.

---

## 16. README content (post-spec, for v0.1.0)

The README ships with:

1. Title + badges (CI, crates.io version, docs.rs, MIT/Apache-2.0).
2. One-paragraph pitch ("Rocket fairing + responders for the Poli Page PDF rendering API").
3. `cargo add poli-page-rocket` + a 12-line quick start (build, attach fairing, route returns `Result<PdfResponse, poli_page::Error>`).
4. The three primitives, one short example each: the fairing, `&State<PoliPageClient>`, `PdfResponse` / `PreviewResponse` / `DocumentRedirect`.
5. Env-var reference (`POLI_PAGE_API_KEY` + the four overrides).
6. Error handling — what gets mapped (every `poli_page::Error` variant), what doesn't (any other error type bubbles to Rocket's default 500 catcher).
7. Streaming example (`render.pdf_stream` → `PdfResponse::stream`).
8. Pointer to `example-app/` (interactive demo dashboard at `GET /`).
9. Contributing → `CLAUDE.md`.
10. License → `MIT OR Apache-2.0`.

Aim: under 250 lines. The SDK's README is the deep-dive surface; this README is the "how does this look in Rocket specifically" surface.

---

## 17. Out of scope (v0.1.0)

Calling these out explicitly so they don't sneak in mid-implementation. Each has a real use case but adds maintenance surface beyond v0.1.0's scope.

| Feature | Why deferred |
|---|---|
| **Blocking-feature surface** | The SDK has a `blocking` module; Rocket 0.5 is async-only, so wrapping it adds zero value. Users who want blocking access call the SDK directly. |
| **Custom Tera/Handlebars template helper** | Niche; Rocket's own template support is enough for the demo HTML. Add as a separate `poli-page-rocket-templates` crate later if asked. |
| **Per-route fairing config overrides** (e.g. different API key per mount point) | Multi-client support; v0.2 add-on like the symfony-bundle's `clients.live` / `clients.test`. v0.1 single-client is purely additive. |
| **OpenAPI schema export via `rocket-okapi`** | Real value but only for users already on `okapi`. Better as a separate `poli-page-rocket-okapi` crate. |
| **A Rocket catcher** for `poli_page::Error` | Catchers are per-status-code, not per-error-type; the `Responder` impl is the correct seam. See `CLAUDE.md` §10.3. |
| **Sentry-style auto-instrumentation** (request spans, breadcrumbs) | The `tracing` bridge in §10.3 already emits structured events; downstream layers (Sentry, OTel) can subscribe via their `tracing-subscriber` integrations. No bespoke instrumentation needed. |
| **`#[derive(PoliPageInput)]` proc-macro** | Cute but YAGNI. The SDK's `ProjectModeInput` / `InlineModeInput` are plain structs; users construct them directly. |
| **A `cargo poli-page` subcommand** | Adds a separate binary crate to maintain; the SDK's `cargo run --example demo` covers the smoke-test workflow. |

**Discipline rule**: when implementing, if a "small addition" feels tempting, check this list first. If it's here, defer. If it's not here, ask before adding.

---

## 18. Resolved decisions

Captured from the spec-review conversation so future agents don't reopen them:

| Decision | Choice | Why |
|---|---|---|
| Verdict (crate vs recipe) | **Full crates.io crate** | Rocket has clear conventions (fairing, managed state, `Responder`) that the SDK alone can't satisfy from a README. Matches the bar set by `sentry-rocket`. |
| Crate naming | **`poli-page-rocket`** | Hyphenated `<service>-<framework>` is the convention in the Rocket ecosystem (`rocket_db_pools` is the outlier; new third-party crates follow `sentry-rocket`, `rocket_cors`, `figment`). Library name `poli_page_rocket` (Rust requires snake_case). |
| State injection | **Managed state via fairing** | Standard Rocket pattern; matches `rocket_db_pools` and `sentry-rocket`. |
| Request guard | **Sugar `PoliPage<'_>`, primary `&State<PoliPageClient>`** | The state form is the canonical Rocket pattern and what the docs lead with; the guard is convenience. |
| Error mapping | **`Responder` impl, opt-in** | Catchers are per-status-code, not per-error-type. `Responder` + `?` is the idiomatic Rust seam. Matches NextJS's "only `PoliPageError` gets mapped" rule. |
| Tracing bridge | **`tracing` crate, default-on, override via custom builder** | `tracing` is the de-facto standard in the Tokio ecosystem; the SDK itself uses it. Custom hooks via `PoliPageFairing::new(builder)` keep the user in control. |
| Hook surface in fairing | **No `on_retry` / `on_error` fields on the fairing** | Closures capture user data — most natural to construct at the call site, not at the framework config layer. The `new(builder)` constructor is the seam. |
| Async runtime | **Tokio (inherited from Rocket)** | Rocket 0.5 is Tokio-only; no choice to make. |
| Test framework | **Rust built-in `#[test]` + `#[rocket::async_test]`** | No external runner. Aligns with `sdk-rust`. |
| Test client | **`rocket::local::asynchronous::Client`** | `blocking::Client` spawns its own runtime per call and breaks under parallel test execution. See `CLAUDE.md` §10.1. |
| Edition / MSRV | **2021 / 1.75** | Rocket 0.5's baseline. Bump only when Rocket bumps. |
| Default features | **None beyond SDK defaults** | The SDK's default `rustls-tls` is inherited. We don't add features in v0.1.0. |
| License | **MIT OR Apache-2.0** | Matches the SDK and the broader Rust ecosystem (Rocket itself, Tokio, reqwest). |
| Env-var loading | **Caller's responsibility (`dotenvy` in example-app)** | Library crates don't touch the filesystem for config. |
| Single root `.env` | **Yes; `dotenvy::from_path_override("../.env")`** | Cross-cutting DX rule from `INTEGRATIONS_PLAN.md` §2. |
| CLI | **None** | Rocket has no per-app CLI surface. Same stance as next.js and nest.js. |
| Interactive demo at `/` | **Yes, ported from symfony-bundle's demo.html** | Cross-cutting DX rule from `INTEGRATIONS_PLAN.md` §1. |
| Integration test | **One, env-gated, `#[ignore]` + runtime check** | Cross-cutting DX rule from `INTEGRATIONS_PLAN.md` §5. |

---

## 19. Definition of done (v0.1.0)

- All §5 files exist; all §6-§11 items are typed, documented, and tested.
- `cargo test` passes offline (no integration test).
- `cargo test -- --ignored` passes with `POLI_PAGE_API_KEY` set in env (one real-API test); without the key, the ignored test is a clean no-op.
- `cargo clippy --all-targets -- -D warnings` is green.
- `cargo doc --no-deps` builds without warnings.
- Example app runs from `cargo run --bin example-app` after `dotenvy` reads `../.env`; the dashboard at `http://localhost:8000/` exercises all 10 SDK demo steps.
- README + CHANGELOG match the v0.1.0 row.
- CLAUDE.md is the integration-flavored one (already in place — the repo had no inherited SDK template).
- CI matrix green: 3 cells (Rust stable / beta / 1.75 MSRV).

This document is the source of truth. If a PR's design conflicts with it, the spec gets updated FIRST in the same PR, with reasoning in the description.
