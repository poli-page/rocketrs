# `poli-page-rocket` v0.1.0 Implementation Plan

> Step-by-step plan to ship v0.1.0. Each task is a single PR, reviewable in <30 min, RED→GREEN→refactor. The implementation spec at `docs/spec/rocket-crate-specification.md` is the design contract; this is the execution order.

**Prerequisite**: read `docs/spec/rocket-crate-specification.md` (entire), `CLAUDE.md`, and `INTEGRATIONS_PLAN.md` §"Cross-cutting DX patterns" before starting Task 1.

**Today**: 2026-05-27.

---

## Pre-flight: clean the inherited scaffold

**Goal**: start from a known clean slate. The `poli-page/rocketrs` repo currently has no inherited SDK-template (it was a fresh git init), so this step is mostly a verification.

- [ ] **Step 0.1: Verify what's actually in the repo**
  ```bash
  cd /Users/mickael/Projects/rocketrs
  git status
  ls -la
  ```
  Expected: `.git/`, `.gitignore`, `CLAUDE.md`, `docs/spec/`, `docs/plan/`. Nothing else.

- [ ] **Step 0.2: Confirm the SDK is reachable**
  ```bash
  ls /Users/mickael/Projects/sdk-rust/Cargo.toml
  ls /Users/mickael/Projects/sdk-rust/src/lib.rs
  ```
  Both must exist. If `target/` is missing, run `cd /Users/mickael/Projects/sdk-rust && cargo build` first to make sure the SDK builds on the current toolchain.

- [ ] **Step 0.3: Pin the Rust toolchain**
  ```bash
  cat > rust-toolchain.toml <<'EOF'
  [toolchain]
  channel = "stable"
  components = ["rustfmt", "clippy"]
  EOF
  ```
  Matches `sdk-rust`'s pin.

- [ ] **Step 0.4: Echo a baseline `.gitignore`**
  ```bash
  cat >> .gitignore <<'EOF'
  /target
  Cargo.lock          # libraries don't commit lockfiles
  /example-app/target
  EOF
  ```
  Cargo's recommendation: commit `Cargo.lock` for binaries, not for libraries. Our published artifact is a library; the example-app under `example-app/` is a binary and gets its own lockfile, also gitignored (it's a workspace member, so the workspace's root `Cargo.lock` would suffice — but we're not making this a workspace, see Task 1).

---

## Task 1: Bootstrap `Cargo.toml`, tooling configs, and CI workflow

**Files:**
- Create: `Cargo.toml`
- Create: `rustfmt.toml`
- Create: `clippy.toml`
- Create: `.github/workflows/ci.yml`
- Create: `LICENSE-MIT`
- Create: `LICENSE-APACHE`
- Create: `src/lib.rs` (skeleton)
- Create: `tests/smoke.rs` (one passing test to prove the pipeline)

**Goal**: every `cargo <command>` works on a freshly cloned repo. `cargo test` passes (one no-op test). CI matrix is green.

- [ ] **Step 1.1: `Cargo.toml`**

```toml
[package]
name          = "poli-page-rocket"
version       = "0.1.0"
edition       = "2021"
rust-version  = "1.75"
authors       = ["Poli Page"]
description   = "Rocket.rs fairing and responders for the Poli Page PDF rendering API"
homepage      = "https://poli.page"
repository    = "https://github.com/poli-page/rocketrs"
documentation = "https://docs.rs/poli-page-rocket"
readme        = "README.md"
license       = "MIT OR Apache-2.0"
keywords      = ["rocket", "pdf", "html", "template", "poli-page"]
categories    = ["api-bindings", "web-programming"]
exclude       = ["/.github", "/example-app"]

[lib]
name = "poli_page_rocket"
path = "src/lib.rs"

[dependencies]
poli-page = "1.0.0-rc.1"
rocket    = { version = "0.5", default-features = false }
tracing   = "0.1"
bytes     = "1.7"
http      = "1.1"

[dev-dependencies]
tokio              = { version = "1.40", features = ["macros", "rt-multi-thread", "time"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Local-dev override while the SDK is pre-stable. Remove on the same PR
# that bumps the requirement to a published stable release.
[patch.crates-io]
poli-page = { path = "../sdk-rust" }

[lints.clippy]
pedantic    = { level = "warn", priority = -1 }
cargo       = { level = "warn", priority = -1 }
# Hand-off lints — see CLAUDE.md §10.2.
unwrap_used = "deny"
expect_used = "deny"
# Quiet a few pedantic lints the SDK pattern intentionally accepts:
module_name_repetitions = "allow"
missing_errors_doc      = "allow"

[package.metadata.docs.rs]
rustdoc-args = ["--cfg", "docsrs"]
targets      = ["x86_64-unknown-linux-gnu"]
```

- [ ] **Step 1.2: `rustfmt.toml`** — match `sdk-rust`'s file
  ```toml
  edition    = "2021"
  max_width  = 100
  ```

- [ ] **Step 1.3: `clippy.toml`** — match `sdk-rust`
  ```toml
  msrv = "1.75"
  ```

- [ ] **Step 1.4: `src/lib.rs` (skeleton)**
  ```rust
  #![forbid(unsafe_code)]
  #![warn(missing_docs)]
  #![cfg_attr(docsrs, feature(doc_cfg))]

  //! Rocket.rs fairing and responders for the [Poli Page] PDF rendering API.
  //!
  //! Wraps the official [`poli_page`] SDK so a Rocket application can
  //! `cargo add poli-page-rocket`, attach the fairing, and inject the SDK
  //! client into routes via [`rocket::State`].
  //!
  //! [Poli Page]: https://poli.page

  // Public modules appear in subsequent tasks. Skeleton-only for Task 1.
  ```

- [ ] **Step 1.5: `tests/smoke.rs`** — prove the pipeline
  ```rust
  #[test]
  fn pipeline_runs() {
      assert_eq!(2 + 2, 4);
  }
  ```

- [ ] **Step 1.6: LICENSE files**
  - Copy `LICENSE-MIT` and `LICENSE-APACHE` from `/Users/mickael/Projects/sdk-rust/` verbatim.

- [ ] **Step 1.7: `.github/workflows/ci.yml`**

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
```

- [ ] **Step 1.8: Install and verify**

```bash
cd /Users/mickael/Projects/rocketrs
cargo fmt --check       # → no diff
cargo clippy --all-targets -- -D warnings   # → no warnings
cargo test              # → 1 passing
cargo doc --no-deps     # → no warnings
```

**Acceptance**: every step green. Commit as `chore: bootstrap Cargo.toml, lint, ci, smoke test`.

---

## Task 2: Header utilities — RFC 5987 filename encoding

**Files:**
- Create: `src/headers.rs`
- Create: `tests/unit_headers.rs`
- Modify: `src/lib.rs` (add `pub(crate) mod headers;`)

**Goal**: filename-encoding logic used by `PdfResponse`. Direct port of the symfony-bundle's `PoliPageResponseFactory` equivalent. Reference: `/Users/mickael/Projects/symfony-bundle/src/Http/PoliPageResponseFactory.php` and its `tests/Unit/Http/PoliPageResponseFactoryTest.php`.

- [ ] **Step 2.1: RED — `tests/unit_headers.rs`**

```rust
//! Header-encoding behaviour (RFC 5987 / RFC 8187 `filename*` parameter).

use poli_page_rocket::headers::{content_disposition, is_ascii_safe, rfc5987_encode};

#[test]
fn is_ascii_safe_true_for_plain_ascii() {
    assert!(is_ascii_safe("invoice-123.pdf"));
}

#[test]
fn is_ascii_safe_false_for_non_ascii() {
    assert!(!is_ascii_safe("facture-éléphant.pdf"));
}

#[test]
fn is_ascii_safe_false_for_control_chars() {
    assert!(!is_ascii_safe("filename\u{0007}.pdf"));
}

#[test]
fn rfc5987_encode_percent_encodes_utf8_bytes() {
    assert_eq!(rfc5987_encode("café.pdf"), "caf%C3%A9.pdf");
}

#[test]
fn rfc5987_encode_leaves_attr_chars_alone() {
    assert_eq!(rfc5987_encode("plain.pdf"), "plain.pdf");
}

#[test]
fn content_disposition_attachment_for_ascii() {
    assert_eq!(
        content_disposition("invoice.pdf", false),
        r#"attachment; filename="invoice.pdf""#,
    );
}

#[test]
fn content_disposition_inline_when_inline_true() {
    assert_eq!(
        content_disposition("invoice.pdf", true),
        r#"inline; filename="invoice.pdf""#,
    );
}

#[test]
fn content_disposition_emits_both_fallback_and_filename_star_for_non_ascii() {
    assert_eq!(
        content_disposition("café.pdf", false),
        r#"attachment; filename="caf_.pdf"; filename*=UTF-8''caf%C3%A9.pdf"#,
    );
}

#[test]
fn content_disposition_escapes_embedded_quotes() {
    assert!(content_disposition(r#"say "hi".pdf"#, false)
        .contains(r#"filename="say \"hi\".pdf""#));
}
```

To make these tests compile against `pub use` from the crate root we expose `headers` as `pub` in this task and demote it to `pub(crate)` in Task 7 (when the public surface is finalised). Workable interim choice; tests import via `use poli_page_rocket::headers::...`.

Run → fails (`headers` module doesn't exist).

- [ ] **Step 2.2: GREEN — `src/headers.rs`**

```rust
//! RFC 5987 / RFC 8187 `Content-Disposition` `filename*` encoding.
//!
//! Ported character-for-character from the symfony-bundle's
//! `PoliPageResponseFactory::makeDisposition` (which itself uses
//! `Symfony\Component\HttpFoundation\HeaderUtils::makeDisposition`).
//! The bundle's tests are the canonical reference; cases live in
//! `tests/unit_headers.rs`.

/// Returns `true` when every byte of `s` is a printable ASCII character
/// (`0x20..=0x7E`). Control characters and any byte ≥ `0x7F` count as
/// non-ASCII for the purposes of `filename` encoding.
#[must_use]
pub fn is_ascii_safe(s: &str) -> bool {
    s.bytes().all(|b| (0x20..=0x7E).contains(&b))
}

/// Percent-encode a string for use in the `filename*=UTF-8''<value>`
/// parameter (RFC 5987 / 8187). Encodes any byte outside the `attr-char`
/// set; the conservative implementation here percent-encodes anything that
/// isn't an unreserved URL character (letters, digits, `-`, `_`, `.`, `~`).
#[must_use]
pub fn rfc5987_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{b:02X}"));
        }
    }
    out
}

/// Build a `Content-Disposition` header value. If `filename` is ASCII-safe
/// the result is `attachment; filename="<escaped>"`; otherwise both an
/// ASCII fallback and an RFC 5987 `filename*` are emitted.
#[must_use]
pub fn content_disposition(filename: &str, inline: bool) -> String {
    let disposition = if inline { "inline" } else { "attachment" };
    if is_ascii_safe(filename) {
        return format!(r#"{disposition}; filename="{}""#, escape_quotes(filename));
    }
    let ascii_fallback: String = filename
        .chars()
        .map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '_' })
        .collect();
    let encoded = rfc5987_encode(filename);
    format!(
        r#"{disposition}; filename="{}"; filename*=UTF-8''{}"#,
        escape_quotes(&ascii_fallback),
        encoded,
    )
}

fn escape_quotes(s: &str) -> String {
    s.replace('\\', r"\\").replace('"', r#"\""#)
}
```

- [ ] **Step 2.3: Wire into `src/lib.rs`**

```rust
// Add to src/lib.rs (Task 1's skeleton):
pub mod headers;
```

Run `cargo test` → all nine assertions pass.

**Acceptance**: green. Commit as `feat: RFC 5987 filename encoding (port from symfony-bundle)`.

---

## Task 3: `PdfResponse` + `Responder` impl

**Files:**
- Create: `src/responses/mod.rs`
- Create: `src/responses/pdf.rs`
- Create: `tests/unit_pdf_response.rs`
- Modify: `src/lib.rs` (add `pub mod responses;`)

**Goal**: a `Responder<'r, 'static>` type that wraps either `bytes::Bytes` or a `PdfByteStream` and emits the documented headers.

- [ ] **Step 3.1: RED — `tests/unit_pdf_response.rs`**

```rust
//! `PdfResponse` produces the documented headers and body.

use bytes::Bytes;
use poli_page_rocket::responses::PdfResponse;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use rocket::{get, routes};

const PDF_STUB: &[u8] = b"%PDF-1.4\nstub\n";

#[get("/bytes")]
fn bytes_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB)).filename("invoice.pdf")
}

#[get("/bytes-inline")]
fn bytes_inline_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB))
        .filename("invoice.pdf")
        .inline()
}

#[get("/bytes-non-ascii")]
fn bytes_non_ascii_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB)).filename("café.pdf")
}

#[get("/bytes-no-filename")]
fn bytes_no_filename_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB))
}

#[get("/bytes-cache-override")]
fn bytes_cache_override_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB))
        .filename("x.pdf")
        .cache_control("public, max-age=60")
}

async fn client() -> Client {
    let rocket = rocket::build().mount(
        "/",
        routes![
            bytes_route,
            bytes_inline_route,
            bytes_non_ascii_route,
            bytes_no_filename_route,
            bytes_cache_override_route,
        ],
    );
    Client::tracked(rocket).await.unwrap()
}

#[rocket::async_test]
async fn pdf_response_sets_application_pdf_and_attachment() {
    let c = client().await;
    let r = c.get("/bytes").dispatch().await;
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(r.content_type(), Some(ContentType::PDF));
    assert_eq!(
        r.headers().get_one("content-disposition"),
        Some(r#"attachment; filename="invoice.pdf""#),
    );
    assert_eq!(
        r.headers().get_one("cache-control"),
        Some("private, no-store"),
    );
    assert_eq!(r.headers().get_one("x-content-type-options"), Some("nosniff"));
    assert_eq!(r.into_bytes().await.unwrap(), PDF_STUB);
}

#[rocket::async_test]
async fn pdf_response_uses_inline_when_requested() {
    let c = client().await;
    let r = c.get("/bytes-inline").dispatch().await;
    assert_eq!(
        r.headers().get_one("content-disposition"),
        Some(r#"inline; filename="invoice.pdf""#),
    );
}

#[rocket::async_test]
async fn pdf_response_rfc5987_encodes_non_ascii_filename() {
    let c = client().await;
    let r = c.get("/bytes-non-ascii").dispatch().await;
    assert_eq!(
        r.headers().get_one("content-disposition"),
        Some(r#"attachment; filename="caf_.pdf"; filename*=UTF-8''caf%C3%A9.pdf"#),
    );
}

#[rocket::async_test]
async fn pdf_response_omits_filename_when_none_given() {
    let c = client().await;
    let r = c.get("/bytes-no-filename").dispatch().await;
    assert_eq!(r.headers().get_one("content-disposition"), Some("attachment"));
}

#[rocket::async_test]
async fn pdf_response_honors_cache_control_override() {
    let c = client().await;
    let r = c.get("/bytes-cache-override").dispatch().await;
    assert_eq!(r.headers().get_one("cache-control"), Some("public, max-age=60"));
}
```

- [ ] **Step 3.2: GREEN — `src/responses/mod.rs`**

```rust
//! Rocket `Responder` types for Poli Page outputs.

pub mod pdf;
pub mod preview;
pub mod redirect;

pub use pdf::PdfResponse;
pub use preview::PreviewResponse;
pub use redirect::DocumentRedirect;
```

`preview` and `redirect` are added in later tasks; create empty `pub mod` files now so this compiles:

```rust
// src/responses/preview.rs — placeholder
// src/responses/redirect.rs — placeholder
```

- [ ] **Step 3.3: GREEN — `src/responses/pdf.rs`**

```rust
//! PDF response — `Responder<'r, 'static>` over `Bytes` or `PdfByteStream`.

use std::io::Cursor;

use bytes::Bytes;
use rocket::http::{ContentType, Header};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use crate::headers::content_disposition;

/// PDF body — either pre-buffered `Bytes` or a streaming view from the SDK.
pub enum PdfBody {
    /// Fully-buffered bytes. `Content-Length` is set on the response.
    Bytes(Bytes),
    /// Chunked stream from `client.render.pdf_stream(...)`.
    Stream(poli_page::client::PdfByteStream),
}

/// `Responder` returning an `application/pdf` body with the conventional
/// headers (`Content-Disposition` per RFC 5987, `Cache-Control: private,
/// no-store`, `X-Content-Type-Options: nosniff`).
#[must_use]
pub struct PdfResponse {
    body: PdfBody,
    filename: Option<String>,
    inline: bool,
    cache_control: Option<String>,
}

impl PdfResponse {
    /// Build a `PdfResponse` from buffered bytes.
    pub fn bytes(body: Bytes) -> Self {
        Self {
            body: PdfBody::Bytes(body),
            filename: None,
            inline: false,
            cache_control: None,
        }
    }

    /// Build a `PdfResponse` from a streaming view.
    pub fn stream(stream: poli_page::client::PdfByteStream) -> Self {
        Self {
            body: PdfBody::Stream(stream),
            filename: None,
            inline: false,
            cache_control: None,
        }
    }

    /// Suggest a filename for the browser's Save dialog.
    pub fn filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }

    /// Switch from `attachment` to `inline` disposition (browser preview).
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    /// Override the default `Cache-Control: private, no-store`.
    pub fn cache_control(mut self, value: impl Into<String>) -> Self {
        self.cache_control = Some(value.into());
        self
    }
}

impl<'r> Responder<'r, 'static> for PdfResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        let mut builder = Response::build();
        builder
            .header(ContentType::PDF)
            .header(Header::new(
                "Cache-Control",
                self.cache_control.unwrap_or_else(|| "private, no-store".into()),
            ))
            .header(Header::new("X-Content-Type-Options", "nosniff"));

        let disposition = match self.filename.as_deref() {
            Some(name) => content_disposition(name, self.inline),
            None => if self.inline { "inline".into() } else { "attachment".into() },
        };
        builder.header(Header::new("Content-Disposition", disposition));

        match self.body {
            PdfBody::Bytes(b) => {
                let len = b.len();
                builder.sized_body(len, Cursor::new(b.to_vec()));
            }
            PdfBody::Stream(stream) => {
                use futures_core::Stream as _;
                use std::pin::Pin;
                use std::task::{Context, Poll};

                // Wrap PdfByteStream as a tokio AsyncRead. The SDK's stream
                // yields `Result<Bytes, poli_page::Error>`; map errors to
                // `io::Error` so Rocket's streamed body can consume it.
                struct ReadAdapter {
                    inner: poli_page::client::PdfByteStream,
                    pending: Option<Bytes>,
                }
                impl tokio::io::AsyncRead for ReadAdapter {
                    fn poll_read(
                        mut self: Pin<&mut Self>,
                        cx: &mut Context<'_>,
                        buf: &mut tokio::io::ReadBuf<'_>,
                    ) -> Poll<std::io::Result<()>> {
                        loop {
                            if let Some(b) = self.pending.as_mut() {
                                let take = buf.remaining().min(b.len());
                                buf.put_slice(&b[..take]);
                                let _ = b.split_to(take);
                                if b.is_empty() { self.pending = None; }
                                return Poll::Ready(Ok(()));
                            }
                            // Safety: pin-project the inner stream by hand —
                            // PdfByteStream is `Unpin`-safe per its docs
                            // (`Pin<Box<dyn Stream + Send>>` internally).
                            let inner = unsafe { Pin::new_unchecked(&mut self.inner) };
                            match inner.poll_next(cx) {
                                Poll::Pending => return Poll::Pending,
                                Poll::Ready(None) => return Poll::Ready(Ok(())),
                                Poll::Ready(Some(Ok(chunk))) => {
                                    self.pending = Some(chunk);
                                }
                                Poll::Ready(Some(Err(e))) => {
                                    return Poll::Ready(Err(std::io::Error::new(
                                        std::io::ErrorKind::Other, e.to_string(),
                                    )));
                                }
                            }
                        }
                    }
                }
                let adapter = ReadAdapter { inner: stream, pending: None };
                builder.streamed_body(adapter);
            }
        }

        builder.ok()
    }
}
```

> NOTE: this matches the SDK's actual exported types verified in `/Users/mickael/Projects/sdk-rust/src/client.rs`: `PdfByteStream` (public, `pub struct`), `bytes::Bytes` (re-exported from the `bytes` crate), `poli_page::Error`. The `client` module path comes from `pub mod client;` in `sdk-rust/src/lib.rs` line 31.

The `unsafe { Pin::new_unchecked }` is a known wart. Acceptable trade-off because (a) `PdfByteStream`'s internal `Pin<Box<dyn Stream + Send>>` is structurally `Unpin`-safe (the box owns the future), and (b) we'd need a `pin-project` macro dep otherwise. We document this with a `// Why:` comment matching the codebase rule in `CLAUDE.md` §5. Re-evaluate at v0.2 once the SDK's stream type may have stabilised an `Unpin` impl publicly.

- [ ] **Step 3.4: typecheck + lint + tests**

```bash
cargo clippy --all-targets -- -D warnings
cargo test --test unit_pdf_response
```

**Acceptance**: 5 tests green. Commit as `feat: PdfResponse responder (bytes + stream)`.

---

## Task 4: `PreviewResponse` + `DocumentRedirect`

**Files:**
- Create: `src/responses/preview.rs` (overwrite the placeholder)
- Create: `src/responses/redirect.rs` (overwrite the placeholder)
- Create: `tests/unit_preview_response.rs`
- Create: `tests/unit_redirect_response.rs`

**Goal**: round out the responders. `PreviewResponse` ships HTML with `text/html; charset=utf-8`; `DocumentRedirect` issues a 302/308 with the presigned URL.

- [ ] **Step 4.1: RED — `tests/unit_preview_response.rs`**

```rust
use poli_page_rocket::responses::PreviewResponse;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes};

#[get("/preview")]
fn preview_route() -> PreviewResponse {
    PreviewResponse::new("<h1>Hi</h1>")
}

#[get("/preview-cache-override")]
fn preview_cache_override_route() -> PreviewResponse {
    PreviewResponse::new("<h1>Hi</h1>").cache_control("public, max-age=300")
}

async fn client() -> Client {
    let r = rocket::build().mount("/", routes![preview_route, preview_cache_override_route]);
    Client::tracked(r).await.unwrap()
}

#[rocket::async_test]
async fn preview_sets_text_html_and_no_store() {
    let c = client().await;
    let r = c.get("/preview").dispatch().await;
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(
        r.headers().get_one("content-type"),
        Some("text/html; charset=utf-8"),
    );
    assert_eq!(r.headers().get_one("cache-control"), Some("private, no-store"));
    assert_eq!(r.headers().get_one("x-content-type-options"), Some("nosniff"));
    assert_eq!(r.into_string().await.unwrap(), "<h1>Hi</h1>");
}

#[rocket::async_test]
async fn preview_honors_cache_control_override() {
    let c = client().await;
    let r = c.get("/preview-cache-override").dispatch().await;
    assert_eq!(r.headers().get_one("cache-control"), Some("public, max-age=300"));
}
```

- [ ] **Step 4.2: GREEN — `src/responses/preview.rs`**

```rust
//! HTML preview response.

use rocket::http::Header;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

/// `Responder` returning `text/html; charset=utf-8`.
#[must_use]
pub struct PreviewResponse {
    html: String,
    cache_control: Option<String>,
}

impl PreviewResponse {
    /// Build a `PreviewResponse` from an owned HTML string.
    pub fn new(html: impl Into<String>) -> Self {
        Self { html: html.into(), cache_control: None }
    }

    /// Override the default `Cache-Control: private, no-store`.
    pub fn cache_control(mut self, value: impl Into<String>) -> Self {
        self.cache_control = Some(value.into());
        self
    }
}

impl From<poli_page::PreviewResult> for PreviewResponse {
    fn from(r: poli_page::PreviewResult) -> Self { Self::new(r.html) }
}

impl From<poli_page::DocumentPreviewResult> for PreviewResponse {
    fn from(r: poli_page::DocumentPreviewResult) -> Self { Self::new(r.html) }
}

impl<'r> Responder<'r, 'static> for PreviewResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(Header::new("Content-Type", "text/html; charset=utf-8"))
            .header(Header::new(
                "Cache-Control",
                self.cache_control.unwrap_or_else(|| "private, no-store".into()),
            ))
            .header(Header::new("X-Content-Type-Options", "nosniff"))
            .sized_body(self.html.len(), std::io::Cursor::new(self.html))
            .ok()
    }
}
```

- [ ] **Step 4.3: RED — `tests/unit_redirect_response.rs`**

```rust
use poli_page_rocket::responses::DocumentRedirect;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes};

#[get("/redirect")]
fn redirect_route() -> DocumentRedirect {
    DocumentRedirect::to("https://example.com/x.pdf")
}

#[get("/redirect-permanent")]
fn redirect_permanent_route() -> DocumentRedirect {
    DocumentRedirect::to("https://example.com/x.pdf").permanent()
}

async fn client() -> Client {
    let r = rocket::build().mount("/", routes![redirect_route, redirect_permanent_route]);
    Client::tracked(r).await.unwrap()
}

#[rocket::async_test]
async fn redirect_default_is_302() {
    let c = client().await;
    let r = c.get("/redirect").dispatch().await;
    assert_eq!(r.status(), Status::Found); // 302
    assert_eq!(r.headers().get_one("location"), Some("https://example.com/x.pdf"));
    assert_eq!(r.headers().get_one("cache-control"), Some("private, no-store"));
}

#[rocket::async_test]
async fn redirect_permanent_is_308() {
    let c = client().await;
    let r = c.get("/redirect-permanent").dispatch().await;
    assert_eq!(r.status(), Status::PermanentRedirect); // 308
}
```

- [ ] **Step 4.4: GREEN — `src/responses/redirect.rs`**

```rust
//! 302/308 redirect to a presigned PDF URL.

use rocket::http::{Header, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

/// `Responder` issuing a 302 (default) or 308 (when `.permanent()` is set)
/// redirect to a presigned URL.
#[must_use]
pub struct DocumentRedirect {
    url: String,
    permanent: bool,
}

impl DocumentRedirect {
    /// 302 redirect to `url`.
    pub fn to(url: impl Into<String>) -> Self {
        Self { url: url.into(), permanent: false }
    }

    /// Upgrade to a 308 permanent redirect.
    pub fn permanent(mut self) -> Self {
        self.permanent = true;
        self
    }
}

impl From<&poli_page::DocumentDescriptor> for DocumentRedirect {
    fn from(d: &poli_page::DocumentDescriptor) -> Self { Self::to(&d.presigned_pdf_url) }
}

impl<'r> Responder<'r, 'static> for DocumentRedirect {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .status(if self.permanent { Status::PermanentRedirect } else { Status::Found })
            .header(Header::new("Location", self.url))
            .header(Header::new("Cache-Control", "private, no-store"))
            .ok()
    }
}
```

**Acceptance**: 4 assertions green per file. Commit as `feat: PreviewResponse and DocumentRedirect responders`.

---

## Task 5: `Responder` impl for `poli_page::Error`

**Files:**
- Create: `src/errors.rs`
- Create: `tests/unit_error_responder.rs`
- Modify: `src/lib.rs` (add `pub mod errors;`)

**Goal**: routes return `Result<PdfResponse, poli_page::Error>` and `?` produces the typed JSON `Response`. The mapping is documented in spec §10.1.

- [ ] **Step 5.1: RED — `tests/unit_error_responder.rs`**

```rust
//! Verify the status map and body shape of `Responder for poli_page::Error`.

use poli_page::Error;
use poli_page_rocket::PoliPageRocket; // marker re-export; see Task 8.
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes};
use std::time::Duration;

#[get("/<variant>")]
async fn err_route(variant: &str) -> Result<&'static str, Error> {
    match variant {
        "bad-request" => Err(Error::BadRequest {
            status: 400,
            code: "INVALID_VERSION_FORMAT".into(),
            message: "bad".into(),
            request_id: Some("req_1".into()),
        }),
        "auth" => Err(Error::Auth {
            status: 401,
            code: "INVALID_API_KEY".into(),
            message: "x".into(),
            request_id: None,
        }),
        "perm" => Err(Error::PermissionDenied {
            status: 403,
            code: "FORBIDDEN".into(),
            message: "x".into(),
            request_id: None,
        }),
        "not-found" => Err(Error::NotFound {
            status: 404,
            code: "NOT_FOUND".into(),
            message: "x".into(),
            request_id: None,
        }),
        "gone" => Err(Error::Gone {
            status: 410,
            code: "GONE".into(),
            message: "x".into(),
            request_id: None,
        }),
        "rate" => Err(Error::RateLimited {
            status: 429,
            code: "QUOTA_EXCEEDED".into(),
            message: "x".into(),
            request_id: None,
        }),
        "api500" => Err(Error::Api {
            status: 503,
            code: "INTERNAL_ERROR".into(),
            message: "x".into(),
            request_id: None,
        }),
        "conn" => Err(Error::Connection {
            message: "dns".into(),
            source: Box::<dyn std::error::Error + Send + Sync>::from("inner"),
        }),
        "timeout" => Err(Error::Timeout { timeout: Duration::from_secs(60) }),
        "aborted" => Err(Error::Aborted),
        "invalid-options" => Err(Error::InvalidOptions { message: "x".into() }),
        "download" => Err(Error::Download {
            message: "s3".into(),
            status: Some(403),
            source: None,
        }),
        "internal" => Err(Error::Internal { message: "x".into(), status: None }),
        _ => Ok("ok"),
    }
}

async fn client() -> Client {
    let r = rocket::build().mount("/", routes![err_route]);
    Client::tracked(r).await.unwrap()
}

async fn body(r: rocket::local::asynchronous::LocalResponse<'_>) -> serde_json::Value {
    let s = r.into_string().await.unwrap();
    serde_json::from_str(&s).unwrap()
}

#[rocket::async_test]
async fn bad_request_maps_to_status_400_with_code_and_request_id() {
    let c = client().await;
    let r = c.get("/bad-request").dispatch().await;
    assert_eq!(r.status(), Status::BadRequest);
    assert_eq!(r.content_type().unwrap().to_string(), "application/json; charset=utf-8");
    assert_eq!(r.headers().get_one("cache-control"), Some("private, no-store"));
    let j = body(r).await;
    assert_eq!(j["code"], "INVALID_VERSION_FORMAT");
    assert_eq!(j["message"], "bad request (400): bad");
    assert_eq!(j["requestId"], "req_1");
}

#[rocket::async_test]
async fn auth_maps_to_401() {
    let c = client().await;
    assert_eq!(c.get("/auth").dispatch().await.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn permission_denied_maps_to_403() {
    let c = client().await;
    assert_eq!(c.get("/perm").dispatch().await.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn not_found_maps_to_404() {
    let c = client().await;
    assert_eq!(c.get("/not-found").dispatch().await.status(), Status::NotFound);
}

#[rocket::async_test]
async fn gone_maps_to_410() {
    let c = client().await;
    assert_eq!(c.get("/gone").dispatch().await.status(), Status::Gone);
}

#[rocket::async_test]
async fn rate_limited_maps_to_429() {
    let c = client().await;
    assert_eq!(c.get("/rate").dispatch().await.status(), Status::TooManyRequests);
}

#[rocket::async_test]
async fn api_503_passes_through() {
    let c = client().await;
    assert_eq!(c.get("/api500").dispatch().await.status(), Status::ServiceUnavailable);
}

#[rocket::async_test]
async fn connection_error_maps_to_502() {
    let c = client().await;
    assert_eq!(c.get("/conn").dispatch().await.status(), Status::BadGateway);
}

#[rocket::async_test]
async fn timeout_maps_to_504() {
    let c = client().await;
    assert_eq!(c.get("/timeout").dispatch().await.status(), Status::GatewayTimeout);
}

#[rocket::async_test]
async fn aborted_maps_to_503() {
    let c = client().await;
    assert_eq!(c.get("/aborted").dispatch().await.status(), Status::ServiceUnavailable);
}

#[rocket::async_test]
async fn invalid_options_maps_to_500() {
    let c = client().await;
    assert_eq!(
        c.get("/invalid-options").dispatch().await.status(),
        Status::InternalServerError,
    );
}

#[rocket::async_test]
async fn download_maps_to_502() {
    let c = client().await;
    assert_eq!(c.get("/download").dispatch().await.status(), Status::BadGateway);
}

#[rocket::async_test]
async fn internal_maps_to_500() {
    let c = client().await;
    assert_eq!(c.get("/internal").dispatch().await.status(), Status::InternalServerError);
}

#[rocket::async_test]
async fn null_request_id_for_reserved_variants() {
    let c = client().await;
    let r = c.get("/conn").dispatch().await;
    let j = body(r).await;
    assert!(j["requestId"].is_null());
}
```

(`PoliPageRocket` is a marker re-export so the test file links to our crate; see Task 8 for the final `lib.rs` shape. Until then, replace the `use poli_page_rocket::PoliPageRocket;` line with the local import — the test compiles without it once `src/errors.rs` exists.)

- [ ] **Step 5.2: GREEN — `src/errors.rs`**

```rust
//! `Responder` impl for the SDK's `poli_page::Error`.

use poli_page::Error;
use rocket::http::{Header, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use std::io::Cursor;

impl<'r> Responder<'r, 'static> for ErrorShim {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        self.0.respond_to(req)
    }
}

/// Wrapper so we can implement `Responder` for the SDK's foreign type
/// without an orphan-rule violation. Routes returning
/// `Result<_, poli_page::Error>` use the blanket impl below; callers who
/// need to materialise the response manually can wrap with
/// `ErrorShim(err)`.
pub struct ErrorShim(pub Error);

impl<'r> Responder<'r, 'static> for &Error {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        respond(self)
    }
}

impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        respond(&self)
    }
}

fn respond(err: &Error) -> response::Result<'static> {
    let status = status_for(err);
    let body = serde_json::json!({
        "code": err.code(),
        "message": err.to_string(),
        "requestId": err.request_id(),
    });
    let body_bytes = serde_json::to_vec(&body).map_err(|_| Status::InternalServerError)?;
    Response::build()
        .status(status)
        .header(Header::new("Content-Type", "application/json; charset=utf-8"))
        .header(Header::new("Cache-Control", "private, no-store"))
        .sized_body(body_bytes.len(), Cursor::new(body_bytes))
        .ok()
}

fn status_for(err: &Error) -> Status {
    match err {
        Error::BadRequest { status, .. }
        | Error::Auth { status, .. }
        | Error::PermissionDenied { status, .. }
        | Error::NotFound { status, .. }
        | Error::Gone { status, .. }
        | Error::RateLimited { status, .. }
        | Error::Api { status, .. } => Status::from_code(*status).unwrap_or(Status::InternalServerError),
        Error::Connection { .. } => Status::BadGateway,
        Error::Timeout { .. } => Status::GatewayTimeout,
        Error::Aborted => Status::ServiceUnavailable,
        Error::InvalidOptions { .. } => Status::InternalServerError,
        Error::Download { .. } => Status::BadGateway,
        Error::Internal { .. } => Status::InternalServerError,
    }
}
```

Note: `Error` is `#[non_exhaustive]`. The `match` here is exhaustive against the current variants; if the SDK adds a new variant, the compiler will flag this as a build failure (`#[non_exhaustive]` requires a wildcard arm in downstream crates). Add a `_ => Status::InternalServerError` arm at the bottom in a follow-up PR if/when the SDK adds a variant; for v0.1 the match is exhaustive against `1.0.0-rc.1`. (We accept the build break as an explicit signal that the spec needs to be reviewed when the SDK changes.)

Add `serde_json` to `[dependencies]` in `Cargo.toml`:

```toml
serde_json = "1.0"
```

- [ ] **Step 5.3: Wire in `src/lib.rs`**

```rust
pub mod errors;
```

Run `cargo test --test unit_error_responder` → all 14 cases green.

**Acceptance**: 14 assertions green. Commit as `feat: Responder impl for poli_page::Error (typed JSON map)`.

---

## Task 6: `PoliPageClient` state newtype + request guard sugar

**Files:**
- Create: `src/state.rs`
- Create: `tests/fairing_state.rs` (uses what state.rs exports; the fairing arrives in Task 7)
- Modify: `src/lib.rs` (add `pub mod state;`)

**Goal**: the newtype that lives in Rocket's managed state and the optional `PoliPage<'_>` request guard.

- [ ] **Step 6.1: RED — write a partial test that uses `PoliPageClient`**

The full fairing-driven test arrives in Task 7. For Task 6 the unit-level test simply asserts that `PoliPageClient` is `Clone + Send + Sync + 'static` (the bounds Rocket's `State` requires).

```rust
// tests/state_bounds.rs
use poli_page_rocket::PoliPageClient;

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

#[test]
fn poli_page_client_is_send_sync_static() {
    assert_send_sync_static::<PoliPageClient>();
}

#[test]
fn poli_page_client_is_clone() {
    fn assert_clone<T: Clone>() {}
    assert_clone::<PoliPageClient>();
}
```

- [ ] **Step 6.2: GREEN — `src/state.rs`**

```rust
//! Managed-state newtype + optional `PoliPage<'r>` request guard.

use rocket::request::{FromRequest, Outcome, Request};
use rocket::State;

/// Newtype wrapping the SDK client for Rocket's managed state.
///
/// `Clone` is cheap — the SDK's `PoliPage` is `Arc`-internal, so cloning is
/// an atomic refcount bump.
#[derive(Clone)]
#[must_use]
pub struct PoliPageClient(pub poli_page::PoliPage);

impl PoliPageClient {
    /// Borrow the underlying SDK client.
    pub fn client(&self) -> &poli_page::PoliPage { &self.0 }
    /// Borrow the `render` namespace.
    pub fn render(&self) -> &poli_page::Render { &self.0.render }
    /// Borrow the `documents` namespace.
    pub fn documents(&self) -> &poli_page::Documents { &self.0.documents }
}

/// Optional sugar: a request guard that resolves to `&poli_page::PoliPage`.
///
/// Routes can either take `&State<PoliPageClient>` (standard Rocket) or
/// `PoliPage<'_>` (one-character less typing). Both work; `&State` is the
/// canonical path documented by Rocket.
pub struct PoliPage<'r>(pub &'r poli_page::PoliPage);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for PoliPage<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.rocket().state::<PoliPageClient>() {
            Some(c) => Outcome::Success(PoliPage(&c.0)),
            None => {
                tracing::error!(
                    target: "poli_page_rocket",
                    "PoliPageFairing is not attached; PoliPage<'_> guard cannot resolve.",
                );
                Outcome::Error((rocket::http::Status::InternalServerError, panic_safe_infallible()))
            }
        }
    }
}

// `Infallible` has no public constructor — we never actually produce one
// (the `Outcome::Error` path is reachable only when the fairing is missing,
// which our docs flag as a programmer error). Helper compiles because the
// uninhabited type lets the compiler accept any path that "produces" one.
fn panic_safe_infallible() -> std::convert::Infallible {
    // Why: returning Infallible from a non-panicking fn is impossible by
    // definition. The match arm above is unreachable in practice (the
    // fairing always populates state in a correctly-configured app). The
    // `unreachable!()` is observable only when the user forgets `attach()`.
    unreachable!("PoliPageFairing not attached")
}
```

Wait — that's not quite right. The `Outcome::Error` arm needs a real `Infallible`. The idiomatic Rocket pattern here is to widen the associated `Error` type to a marker enum we control, OR to make `Outcome::Error` impossible by `unreachable!`-ing inside the match itself. Pick the second; the arm is only hit when the user forgot to attach the fairing.

Revised:

```rust
#[rocket::async_trait]
impl<'r> FromRequest<'r> for PoliPage<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.rocket().state::<PoliPageClient>() {
            Some(c) => Outcome::Success(PoliPage(&c.0)),
            None => {
                tracing::error!(
                    target: "poli_page_rocket",
                    "PoliPageFairing is not attached; PoliPage<'_> guard cannot resolve.",
                );
                // Why: when the fairing isn't attached, returning Forward
                // makes Rocket try the next matching route — which won't
                // exist either. The cleanest UX is a 500. We use Status
                // directly (no error payload) since Infallible has no
                // value.
                Outcome::Forward(rocket::http::Status::InternalServerError)
            }
        }
    }
}
```

(The `Outcome::Forward` variant carries a `Status` in Rocket 0.5; verify the exact API name when implementing — Rocket has minor API churn between rcs. The intent is "client missing → 500".)

- [ ] **Step 6.3: Wire in `src/lib.rs`**

```rust
pub mod state;
pub use state::{PoliPage, PoliPageClient};
```

Run `cargo test --test state_bounds` → 2 cases green.

**Acceptance**: green. Commit as `feat: PoliPageClient state newtype + PoliPage<'r> request guard`.

---

## Task 7: `PoliPageFairing`

**Files:**
- Create: `src/fairing.rs`
- Create: `src/tracing_bridge.rs`
- Create: `tests/fairing_state.rs`
- Create: `tests/fairing_invalid_config.rs`
- Modify: `src/lib.rs` (add `pub mod fairing;`)

**Goal**: the headline feature. A fairing that on ignite builds the SDK client from env (or a supplied builder) and inserts it into managed state.

- [ ] **Step 7.1: RED — `tests/fairing_state.rs`**

```rust
//! End-to-end: attach the fairing, dispatch a route that takes &State<PoliPageClient>.

use poli_page_rocket::{PoliPageClient, PoliPageFairing};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes, State};

#[get("/probe")]
fn probe(_client: &State<PoliPageClient>) -> &'static str {
    "ok"
}

#[rocket::async_test]
async fn fairing_inserts_client_into_state() {
    std::env::set_var("POLI_PAGE_API_KEY", "pp_test_demo_key");
    let r = rocket::build()
        .attach(PoliPageFairing::from_env())
        .mount("/", routes![probe]);
    let c = Client::tracked(r).await.unwrap();
    let resp = c.get("/probe").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_string().await.unwrap(), "ok");
}

#[rocket::async_test]
async fn fairing_with_explicit_builder_works() {
    let builder = poli_page::PoliPage::builder().api_key("pp_test_explicit");
    let r = rocket::build()
        .attach(PoliPageFairing::new(builder))
        .mount("/", routes![probe]);
    let c = Client::tracked(r).await.unwrap();
    assert_eq!(c.get("/probe").dispatch().await.status(), Status::Ok);
}

#[rocket::async_test]
async fn fairing_with_prebuilt_client_works() {
    let client = poli_page::PoliPage::new("pp_test_prebuilt").unwrap();
    let r = rocket::build()
        .attach(PoliPageFairing::with_client(client))
        .mount("/", routes![probe]);
    let c = Client::tracked(r).await.unwrap();
    assert_eq!(c.get("/probe").dispatch().await.status(), Status::Ok);
}
```

- [ ] **Step 7.2: RED — `tests/fairing_invalid_config.rs`**

```rust
use poli_page_rocket::PoliPageFairing;
use rocket::local::asynchronous::Client;

#[rocket::async_test]
async fn missing_api_key_fails_ignite() {
    std::env::remove_var("POLI_PAGE_API_KEY");
    let r = rocket::build().attach(PoliPageFairing::from_env());
    let res = Client::tracked(r).await;
    assert!(res.is_err(), "ignite should fail when POLI_PAGE_API_KEY is missing");
}

#[rocket::async_test]
async fn bad_key_prefix_fails_ignite() {
    std::env::set_var("POLI_PAGE_API_KEY", "sk_test_wrong_prefix");
    let r = rocket::build().attach(PoliPageFairing::from_env());
    let res = Client::tracked(r).await;
    assert!(res.is_err(), "ignite should fail when prefix is wrong");
}

#[rocket::async_test]
async fn bad_timeout_fails_ignite() {
    std::env::set_var("POLI_PAGE_API_KEY", "pp_test_ok");
    std::env::set_var("POLI_PAGE_TIMEOUT_SECS", "not-a-number");
    let r = rocket::build().attach(PoliPageFairing::from_env());
    let res = Client::tracked(r).await;
    assert!(res.is_err());
    std::env::remove_var("POLI_PAGE_TIMEOUT_SECS");
}
```

> NOTE: setting `std::env::var` in tests has a global-mutability hazard under parallel execution. The tests in this file set then `remove_var` in inverse order; running `cargo test -- --test-threads=1` is the safer mode. We add `#[serial_test::serial]` or use a `Mutex` if flakiness appears — defer until observed. Marked in spec §13.3.

- [ ] **Step 7.3: GREEN — `src/tracing_bridge.rs`**

```rust
//! Default `on_retry` / `on_error` hooks emitting structured `tracing` events.

use poli_page::{Error, RetryEvent};

pub(crate) fn on_retry(event: &RetryEvent) {
    tracing::warn!(
        target: "poli_page_rocket",
        attempt = event.attempt,
        delay_ms = u64::try_from(event.delay.as_millis()).unwrap_or(u64::MAX),
        code = event.reason.code(),
        status = event.reason.status(),
        request_id = event.reason.request_id(),
        "poli_page retry",
    );
}

pub(crate) fn on_error(err: &Error) {
    tracing::error!(
        target: "poli_page_rocket",
        code = err.code(),
        status = err.status(),
        request_id = err.request_id(),
        message = %err,
        "poli_page terminal error",
    );
}
```

- [ ] **Step 7.4: GREEN — `src/fairing.rs`**

```rust
//! Rocket fairing that builds the SDK client at ignite and inserts it
//! into managed state.

use std::time::Duration;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Rocket};

use crate::state::PoliPageClient;
use crate::tracing_bridge;

const KEY_PATTERN_PREFIXES: &[&str] = &["pp_test_", "pp_live_"];

/// Rocket fairing that builds a `poli_page::PoliPage` at ignite-time and
/// inserts it into managed state as `PoliPageClient`.
pub struct PoliPageFairing {
    source: Source,
}

enum Source {
    Env,
    Builder(Box<poli_page::PoliPageBuilder>),
    Built(poli_page::PoliPage),
}

impl PoliPageFairing {
    /// Read all options from environment variables (see spec §6.3).
    /// Ignite fails if `POLI_PAGE_API_KEY` is missing or malformed.
    pub fn from_env() -> Self { Self { source: Source::Env } }

    /// Build a fairing from an already-configured `PoliPageBuilder`.
    pub fn new(builder: poli_page::PoliPageBuilder) -> Self {
        Self { source: Source::Builder(Box::new(builder)) }
    }

    /// Wrap an already-built client (e.g. one shared with non-Rocket code).
    pub fn with_client(client: poli_page::PoliPage) -> Self {
        Self { source: Source::Built(client) }
    }
}

#[rocket::async_trait]
impl Fairing for PoliPageFairing {
    fn info(&self) -> Info {
        Info { name: "poli-page-rocket", kind: Kind::Ignite }
    }

    async fn on_ignite(self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        let client = match self.source {
            Source::Built(c) => c,
            Source::Builder(b) => {
                let b = b
                    .on_retry(tracing_bridge::on_retry)
                    .on_error(tracing_bridge::on_error);
                match b.build() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(target: "poli_page_rocket", error = %e, "PoliPage build failed");
                        return Err(rocket);
                    }
                }
            }
            Source::Env => match build_from_env() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(target: "poli_page_rocket", error = %e, "PoliPage env config invalid");
                    return Err(rocket);
                }
            },
        };
        Ok(rocket.manage(PoliPageClient(client)))
    }
}

fn build_from_env() -> Result<poli_page::PoliPage, String> {
    let api_key = std::env::var("POLI_PAGE_API_KEY")
        .map_err(|_| "POLI_PAGE_API_KEY is not set".to_owned())?;
    if !KEY_PATTERN_PREFIXES.iter().any(|p| api_key.starts_with(p)) {
        return Err(format!(
            "POLI_PAGE_API_KEY must start with pp_test_ or pp_live_; got prefix {:?}. Get a key at https://app.poli.page/settings/api-keys.",
            api_key.chars().take(8).collect::<String>(),
        ));
    }
    let mut builder = poli_page::PoliPage::builder()
        .api_key(api_key)
        .on_retry(tracing_bridge::on_retry)
        .on_error(tracing_bridge::on_error);

    if let Ok(v) = std::env::var("POLI_PAGE_BASE_URL") {
        builder = builder.base_url(v);
    }
    if let Ok(v) = std::env::var("POLI_PAGE_TIMEOUT_SECS") {
        let secs: u64 = v.parse().map_err(|_| format!("POLI_PAGE_TIMEOUT_SECS={v:?} is not a number"))?;
        builder = builder.timeout(Duration::from_secs(secs));
    }
    if let Ok(v) = std::env::var("POLI_PAGE_MAX_RETRIES") {
        let n: u32 = v.parse().map_err(|_| format!("POLI_PAGE_MAX_RETRIES={v:?} is not a number"))?;
        builder = builder.max_retries(n);
    }
    if let Ok(v) = std::env::var("POLI_PAGE_RETRY_DELAY_MS") {
        let ms: u64 = v.parse().map_err(|_| format!("POLI_PAGE_RETRY_DELAY_MS={v:?} is not a number"))?;
        builder = builder.retry_delay(Duration::from_millis(ms));
    }
    builder.build().map_err(|e| e.to_string())
}
```

- [ ] **Step 7.5: Wire in `src/lib.rs`**

```rust
pub mod fairing;
pub use fairing::PoliPageFairing;
```

- [ ] **Step 7.6: typecheck + lint + tests**

```bash
cargo clippy --all-targets -- -D warnings
cargo test --test fairing_state --test fairing_invalid_config
```

**Acceptance**: 6 assertions green. Commit as `feat: PoliPageFairing (from_env, new, with_client)`.

---

## Task 8: Public `src/lib.rs` finalisation + tracing bridge unit test

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/unit_tracing_bridge.rs`

**Goal**: the crate's public surface is finalised — only the items documented in spec §4 are exported. The tracing bridge gets its own unit test (separate from the fairing).

- [ ] **Step 8.1: Final `src/lib.rs`**

```rust
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(
    // SDK pattern: modules named after their types (PoliPageClient, etc.).
    clippy::module_name_repetitions,
    // Doc-comment busywork; covered by spec §10.
    clippy::missing_errors_doc,
    // Transitive crate-graph noise.
    clippy::multiple_crate_versions,
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! Rocket.rs fairing and responders for the [Poli Page] PDF rendering API.
//!
//! ## Quick start
//!
//! ```no_run
//! use poli_page_rocket::{PoliPageClient, PoliPageFairing, PdfResponse};
//! use rocket::{get, routes, State};
//! use serde_json::json;
//!
//! #[get("/welcome.pdf")]
//! async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, poli_page::Error> {
//!     let bytes = client.render().pdf(poli_page::ProjectModeInput {
//!         project: "getting-started".into(),
//!         template: "welcome".into(),
//!         version: Some("1.0.0".into()),
//!         data: json!({ "name": "World" }),
//!         ..Default::default()
//!     }).await?;
//!     Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
//! }
//!
//! #[rocket::launch]
//! fn rocket() -> _ {
//!     rocket::build()
//!         .attach(PoliPageFairing::from_env())
//!         .mount("/", routes![welcome])
//! }
//! ```
//!
//! [Poli Page]: https://poli.page

pub mod fairing;
pub mod responses;
pub mod state;
pub mod errors;

// Internal helpers.
pub mod headers;
mod tracing_bridge;

pub use fairing::PoliPageFairing;
pub use state::{PoliPage, PoliPageClient};
pub use responses::{DocumentRedirect, PdfResponse, PreviewResponse};

// Re-export selected SDK types users will need at the integration boundary.
// Mirrors the @poli-page/nextjs spec §7.2 pattern.
pub use poli_page::{
    DocumentDescriptor, DocumentPreviewResult, Error, InlineModeInput, PoliPage as SdkPoliPage,
    PreviewResult, ProjectModeInput, RenderInput, RetryEvent, ThumbnailFormat, ThumbnailOptions,
};
```

Note: we keep `pub mod headers` for now to satisfy the Task 2 test's import path. The alternative (re-export via `pub use crate::headers::content_disposition;` and demote the module to `pub(crate)`) is tidier; decide in PR review.

- [ ] **Step 8.2: RED — `tests/unit_tracing_bridge.rs`**

```rust
//! Verify on_retry and on_error emit events under target "poli_page_rocket".

use std::sync::{Arc, Mutex};
use std::time::Duration;

use poli_page::{Error, RetryEvent};
use tracing::subscriber::with_default;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureWriterHandle;
    fn make_writer(&'a self) -> Self::Writer { CaptureWriterHandle(self.0.clone()) }
}
struct CaptureWriterHandle(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for CaptureWriterHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

#[test]
fn on_retry_emits_event_with_attempt_and_delay() {
    let writer = CaptureWriter::default();
    let subscriber = fmt().with_writer(writer.clone()).without_time().finish();
    with_default(subscriber, || {
        let event = RetryEvent {
            attempt: 3,
            delay: Duration::from_millis(750),
            reason: Error::Api {
                status: 503, code: "INTERNAL_ERROR".into(), message: "boom".into(),
                request_id: Some("req_99".into()),
            },
        };
        // The tracing_bridge module is private; invoke via the public
        // PoliPageFairing path is unwieldy in a unit test. For this test
        // we re-export the helpers via `cfg(test)` from src/lib.rs:
        //
        //     #[cfg(test)]
        //     pub use crate::tracing_bridge::{on_retry, on_error};
        //
        // and call them directly. Alternatively, expose under a
        // `_internal` module gated on `#[doc(hidden)]`.
        poli_page_rocket::__internal_tracing_bridge::on_retry(&event);
    });
    let buf = String::from_utf8(writer.0.lock().unwrap().clone()).unwrap();
    assert!(buf.contains("poli_page_rocket"));
    assert!(buf.contains("attempt=3"));
    assert!(buf.contains("delay_ms=750"));
    assert!(buf.contains("code=\"INTERNAL_ERROR\""));
    assert!(buf.contains("status=503"));
}

#[test]
fn on_error_emits_terminal_event() {
    let writer = CaptureWriter::default();
    let subscriber = fmt().with_writer(writer.clone()).without_time().finish();
    with_default(subscriber, || {
        let err = Error::Timeout { timeout: Duration::from_secs(60) };
        poli_page_rocket::__internal_tracing_bridge::on_error(&err);
    });
    let buf = String::from_utf8(writer.0.lock().unwrap().clone()).unwrap();
    assert!(buf.contains("poli_page_rocket"));
    assert!(buf.contains("code=\"timeout\""));
}
```

- [ ] **Step 8.3: Expose the bridge for tests via a hidden module**

Add to `src/lib.rs`:

```rust
#[doc(hidden)]
pub mod __internal_tracing_bridge {
    //! Exposed for unit tests only. Not part of the stable surface.
    pub use crate::tracing_bridge::{on_error, on_retry};
}
```

Add `tracing-subscriber` to `[dev-dependencies]` if not already present (Task 1 includes it).

Run `cargo test --test unit_tracing_bridge` → 2 cases green.

**Acceptance**: green. Commit as `feat: finalise public lib.rs + tracing-bridge unit tests`.

---

## Task 9: Real-API integration test (env-gated)

**Files:**
- Create: `tests/integration_render.rs`

**Goal**: ONE happy-path round-trip against `api-develop.poli.page` rendering `getting-started/welcome`. Skipped via `#[ignore]` plus a runtime env check.

- [ ] **Step 9.1: Write the test**

```rust
//! Integration test against the develop API.
//!
//! Marked `#[ignore]` so the default `cargo test` skips it. Run with:
//!     cargo test -- --ignored
//! Requires `POLI_PAGE_API_KEY` to be set; without it the test exits
//! cleanly as a no-op rather than failing (so PR contributors without a
//! key get green local runs even with `--ignored`).

use poli_page_rocket::{PoliPageClient, PoliPageFairing, PdfResponse};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes, State};
use serde_json::json;

#[get("/welcome.pdf")]
async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, poli_page::Error> {
    let bytes = client.render().pdf(poli_page::ProjectModeInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        version: Some("1.0.0".into()),
        data: json!({ "name": "rocketrs integration test" }),
        ..Default::default()
    }).await?;
    Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
}

#[rocket::async_test]
#[ignore = "real API; opt-in via `cargo test -- --ignored` with POLI_PAGE_API_KEY set"]
async fn render_welcome_against_develop_api() {
    if std::env::var("POLI_PAGE_API_KEY").is_err() {
        eprintln!("POLI_PAGE_API_KEY not set; skipping real-API test.");
        return;
    }
    std::env::set_var(
        "POLI_PAGE_BASE_URL",
        std::env::var("POLI_PAGE_BASE_URL")
            .unwrap_or_else(|_| "https://api-develop.poli.page".into()),
    );
    let r = rocket::build()
        .attach(PoliPageFairing::from_env())
        .mount("/", routes![welcome]);
    let c = Client::tracked(r).await.expect("rocket ignites");
    let resp = c.get("/welcome.pdf").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(
        resp.content_type().expect("content-type set").to_string(),
        "application/pdf",
    );
    let body = resp.into_bytes().await.expect("body bytes");
    assert!(body.len() > 1000, "PDF body should be > 1000 bytes; was {}", body.len());
    assert_eq!(&body[..5], b"%PDF-");
}
```

- [ ] **Step 9.2: Smoke-run locally**

```bash
export POLI_PAGE_API_KEY=pp_test_…  # your dev key
cargo test --test integration_render -- --ignored
```

Without the key:

```bash
unset POLI_PAGE_API_KEY
cargo test --test integration_render -- --ignored
# → 1 ignored test runs, prints "POLI_PAGE_API_KEY not set; skipping real-API test." and passes
```

**Acceptance**: ignored by default, runs with `--ignored`, no-op when env is missing. Commit as `test: real-API integration test (env-gated, #[ignore])`.

---

## Task 10: Example app — Rocket scaffold + 10 demo routes

**Files** (all under `example-app/`):
- Create: `example-app/Cargo.toml`
- Create: `example-app/src/main.rs`
- Create: `example-app/src/routes/mod.rs`
- Create: `example-app/src/routes/demo.rs`
- Create: `example-app/src/routes/render.rs`
- Create: `example-app/src/routes/documents.rs`
- Create: `example-app/src/routes/errors.rs`
- Create: `example-app/src/bin/render_to_file.rs`
- Create: `example-app/.gitignore`

**Goal**: 9 routes covering SDK demo steps 1, 2, 4–10, one standalone binary for step 3, all calling `poli-page-rocket` for response shape.

- [ ] **Step 10.1: `example-app/Cargo.toml`**

```toml
[package]
name    = "poli-page-rocket-example-app"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
poli-page-rocket = { path = ".." }
poli-page        = "1.0.0-rc.1"
rocket           = { version = "0.5", features = ["json"] }
serde            = { version = "1", features = ["derive"] }
serde_json       = "1"
dotenvy          = "0.15"
tokio            = { version = "1", features = ["macros", "rt-multi-thread"] }
tracing          = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
futures-util     = "0.3"
bytes            = "1.7"

[patch.crates-io]
poli-page = { path = "../../sdk-rust" }

[[bin]]
name = "example-app"
path = "src/main.rs"

[[bin]]
name = "render_to_file"
path = "src/bin/render_to_file.rs"
```

- [ ] **Step 10.2: `example-app/src/main.rs`**

```rust
use poli_page_rocket::PoliPageFairing;
use rocket::fs::{FileServer, relative};

mod routes;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ = dotenvy::from_path_override("../.env");
    let _ = dotenvy::from_path_override(".env");

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,poli_page_rocket=debug".into()),
        )
        .init();

    rocket::build()
        .attach(PoliPageFairing::from_env())
        .mount("/", rocket::routes![
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
        .mount("/static", FileServer::from(relative!("static")))
        .launch()
        .await
        .map(|_| ())
}
```

- [ ] **Step 10.3: `example-app/src/routes/render.rs`**

```rust
use poli_page::ProjectModeInput;
use poli_page_rocket::{PdfResponse, PoliPageClient, PreviewResponse};
use rocket::{get, State};
use serde_json::json;

fn input() -> ProjectModeInput {
    ProjectModeInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        version: Some("1.0.0".into()),
        data: json!({ "name": "World" }),
        ..Default::default()
    }
}

#[get("/render/pdf")]
pub async fn pdf(client: &State<PoliPageClient>) -> Result<PdfResponse, poli_page::Error> {
    let bytes = client.render().pdf(input()).await?;
    Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
}

#[get("/render/stream")]
pub async fn stream(client: &State<PoliPageClient>) -> Result<PdfResponse, poli_page::Error> {
    let stream = client.render().pdf_stream(input()).await?;
    Ok(PdfResponse::stream(stream).filename("welcome.pdf").inline())
}

#[get("/render/preview")]
pub async fn preview(client: &State<PoliPageClient>) -> Result<PreviewResponse, poli_page::Error> {
    let result = client.render().preview(input()).await?;
    Ok(PreviewResponse::from(result))
}
```

- [ ] **Step 10.4: `example-app/src/routes/documents.rs`**

```rust
use poli_page::ProjectModeInput;
use poli_page_rocket::{DocumentRedirect, PoliPageClient, PreviewResponse};
use rocket::serde::json::Json;
use rocket::{delete, get, post, State};
use serde_json::{json, Value};

#[post("/documents")]
pub async fn create(client: &State<PoliPageClient>) -> Result<Json<Value>, poli_page::Error> {
    let descriptor = client.render().document(ProjectModeInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        version: Some("1.0.0".into()),
        data: json!({ "name": "Stored doc" }),
        ..Default::default()
    }).await?;
    Ok(Json(json!({
        "documentId": descriptor.document_id,
        "pageCount": descriptor.page_count,
        "sizeBytes": descriptor.size_bytes,
    })))
}

#[get("/documents/<id>")]
pub async fn get(id: &str, client: &State<PoliPageClient>) -> Result<DocumentRedirect, poli_page::Error> {
    let descriptor = client.documents().get(id).await?;
    Ok(DocumentRedirect::to(&descriptor.presigned_pdf_url))
}

#[delete("/documents/<id>")]
pub async fn delete(id: &str, client: &State<PoliPageClient>) -> Result<rocket::http::Status, poli_page::Error> {
    client.documents().delete(id).await?;
    Ok(rocket::http::Status::NoContent)
}

#[get("/documents/<id>/thumbnails")]
pub async fn thumbnails(id: &str, client: &State<PoliPageClient>) -> Result<Json<Value>, poli_page::Error> {
    let result = client.documents().thumbnails(id, &Default::default()).await?;
    Ok(Json(serde_json::to_value(result).expect("thumbnails serialise")))
}

#[get("/documents/<id>/preview")]
pub async fn preview(id: &str, client: &State<PoliPageClient>) -> Result<PreviewResponse, poli_page::Error> {
    let result = client.documents().preview(id).await?;
    Ok(PreviewResponse::from(result))
}
```

(`thumbnails` signature: verify against `sdk-rust/src/documents.rs` — it likely takes `&ThumbnailOptions`. Use `ThumbnailOptions::default()` for the demo.)

- [ ] **Step 10.5: `example-app/src/routes/errors.rs`**

```rust
use poli_page::ProjectModeInput;
use poli_page_rocket::PoliPageClient;
use rocket::{get, State};
use serde_json::json;

#[get("/errors/bad-version")]
pub async fn bad_version(client: &State<PoliPageClient>) -> Result<&'static str, poli_page::Error> {
    client.render().pdf(ProjectModeInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        version: Some("not-a-version".into()),
        data: json!({}),
        ..Default::default()
    }).await?;
    Ok("unreachable")
}
```

- [ ] **Step 10.6: `example-app/src/bin/render_to_file.rs`** — SDK demo step 3

```rust
use poli_page::{PoliPage, ProjectModeInput};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::from_path_override("../.env");
    let api_key = std::env::var("POLI_PAGE_API_KEY")?;
    let base_url = std::env::var("POLI_PAGE_BASE_URL")
        .unwrap_or_else(|_| "https://api-develop.poli.page".into());
    let client = PoliPage::builder().api_key(api_key).base_url(base_url).build()?;
    let path = "/tmp/poli-page-rocketrs-demo.pdf";
    poli_page::render_to_file(&client, ProjectModeInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        version: Some("1.0.0".into()),
        data: json!({ "name": "render_to_file demo" }),
        ..Default::default()
    }, path).await?;
    println!("Wrote {path}");
    Ok(())
}
```

- [ ] **Step 10.7: smoke**

```bash
cd example-app
cargo run --bin example-app
# in another terminal:
curl -o /tmp/welcome.pdf http://localhost:8000/render/pdf
head -c 8 /tmp/welcome.pdf
# → %PDF-1.4
```

**Acceptance**: every route returns the expected response. Commit as `feat(example-app): all 10 SDK demo routes + render_to_file binary`.

---

## Task 11: Example app — interactive demo UI at `/`

**Files:**
- Create: `example-app/static/index.html`
- Create: `example-app/src/routes/demo.rs`

**Goal**: port the symfony-bundle's `templates/demo.html` interactive dashboard to a Rocket-served static page. Same aesthetic, same 9-button layout, same JS state machine for the document lifecycle.

- [ ] **Step 11.1: Copy the HTML wholesale**

```bash
cp /Users/mickael/Projects/symfony-bundle/example-app/templates/demo.html \
   /Users/mickael/Projects/rocketrs/example-app/static/index.html
```

Then adjust any URL paths to match the Rocket route names (the symfony app uses similar `/render/pdf`, `/documents`, etc. — most paths are identical). Specifically:

- The symfony app's `POST /documents` returns descriptor JSON with `documentId`. Our Rocket route returns the same shape (Task 10.4). No change.
- `GET /documents/{id}` symfony → 302 redirect. Same here.
- All other paths match by name.

If the symfony dashboard hard-codes a CSRF token or any Symfony-specific path, strip it — Rocket has no CSRF middleware enabled in this demo.

- [ ] **Step 11.2: `example-app/src/routes/demo.rs`** — serve `index.html` at `/`

```rust
use rocket::get;
use rocket::response::content::RawHtml;

const INDEX_HTML: &str = include_str!("../../static/index.html");

#[get("/")]
pub fn index() -> RawHtml<&'static str> {
    RawHtml(INDEX_HTML)
}
```

`include_str!` resolves at compile time, so the binary is self-contained. (Alternative: rely on the `FileServer::from(relative!("static"))` mount in `main.rs` to serve `/static/index.html`, then have the `/` route redirect there. Less clean; prefer `include_str!`.)

- [ ] **Step 11.3: smoke**

```bash
cd example-app
cargo run --bin example-app
open http://localhost:8000
# Click every button. Confirm parity with the symfony-bundle dashboard:
# - "Render PDF" → embedded iframe shows the PDF
# - "Stream PDF" → same, served chunked
# - "Preview HTML" → iframe srcdoc shows the page
# - "Create document" → JSON pane shows {documentId, pageCount, sizeBytes}
# - "Get document" → 302 follows to the presigned URL
# - "Thumbnails" → JSON pane shows the thumbnail array
# - "Document preview" → iframe srcdoc
# - "Delete document" → 204 (button greys out)
# - "Trigger error" → red JSON pane shows {code: INVALID_VERSION_FORMAT, …}
```

**Acceptance**: dashboard renders, all 9 buttons work, document lifecycle gating works. Commit as `feat(example-app): interactive demo dashboard at /`.

---

## Task 12: README and CHANGELOG for v0.1.0

**Files:**
- Create: `README.md`
- Create: `CHANGELOG.md`

**Goal**: ship a README under 250 lines covering the public API + example-app pointer. Match the symfony-bundle / nextjs README structure.

- [ ] **Step 12.1: `README.md`**

Structure (mirror spec §16):

```markdown
# poli-page-rocket

[![CI](https://github.com/poli-page/rocketrs/actions/workflows/ci.yml/badge.svg)](https://github.com/poli-page/rocketrs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/poli-page-rocket.svg)](https://crates.io/crates/poli-page-rocket)
[![docs.rs](https://img.shields.io/docsrs/poli-page-rocket)](https://docs.rs/poli-page-rocket)
[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Rocket.rs fairing and responders for the [Poli Page] PDF rendering API. Thin idiomatic veneer over the official [`poli-page`] SDK — DI via Rocket's managed state, `Responder` impls with correct headers, opt-in `Responder` for the SDK's `Error` type.

## Install

```bash
cargo add poli-page-rocket poli-page rocket
```

## Quick start

[12-line snippet matching spec §16, item 3]

## The three primitives

### The fairing
[builder + from_env + with_client]

### State extraction
[&State<PoliPageClient> and PoliPage<'_>]

### Response types
[PdfResponse, PreviewResponse, DocumentRedirect — one example each]

## Environment variables

| Var | Purpose | Default |
|---|---|---|
| `POLI_PAGE_API_KEY` | API key (required) | — |
| `POLI_PAGE_BASE_URL` | Override base URL | `https://api.poli.page` |
| `POLI_PAGE_TIMEOUT_SECS` | Per-attempt timeout | SDK default (60s) |
| `POLI_PAGE_MAX_RETRIES` | Retry budget | SDK default (2) |
| `POLI_PAGE_RETRY_DELAY_MS` | Initial retry delay | SDK default (500ms) |

## Error handling

Routes returning `Result<_, poli_page::Error>` get typed JSON error responses for free. Other error types bubble to Rocket's default 500 catcher — generic exception swallowing destroys observability.

## Streaming

[render.pdf_stream → PdfResponse::stream snippet]

## Example app

See `example-app/` for an interactive demo at `http://localhost:8000/` exercising all 10 SDK demo steps.

## Contributing

See `CLAUDE.md`.

## License

MIT OR Apache-2.0 (at your option).

[Poli Page]: https://poli.page
[`poli-page`]: https://crates.io/crates/poli-page
```

- [ ] **Step 12.2: `CHANGELOG.md`**

```markdown
# Changelog

All notable changes to `poli-page-rocket` are documented here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release scaffolding.

## [0.1.0] — 2026-06-XX

### Added
- `PoliPageFairing` (`from_env`, `new(builder)`, `with_client`) builds the SDK client at ignite-time and inserts it into Rocket's managed state.
- `PoliPageClient` newtype + optional `PoliPage<'_>` request guard.
- Response types: `PdfResponse` (bytes + stream), `PreviewResponse`, `DocumentRedirect`. RFC 5987 filename encoding for non-ASCII names. Default headers: `Cache-Control: private, no-store`, `X-Content-Type-Options: nosniff`.
- `Responder` impl for `poli_page::Error` — opt-in typed JSON error mapping (4xx pass-through, 5xx pass-through, network → 502, timeout → 504, aborted → 503, internal → 500).
- Default `on_retry` / `on_error` hooks bridge to `tracing` events under target `poli_page_rocket`. Override via custom `PoliPageBuilder`.
- Example Rocket 0.5 app at `example-app/` covering all 10 SDK demo steps, with an interactive demo dashboard at `/` matching the symfony-bundle pattern.

[Unreleased]: https://github.com/poli-page/rocketrs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/poli-page/rocketrs/releases/tag/v0.1.0
```

**Acceptance**: README under 250 lines, CHANGELOG present. Commit as `docs: README and CHANGELOG for v0.1.0`.

---

## Task 13: Final pass — verify, tag, publish dry-run

**Files**: none (operational task)

- [ ] **Step 13.1: Full local CI**
  ```bash
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test
  cargo doc --no-deps
  ```
  All green.

- [ ] **Step 13.2: Verify `cargo package` shape**
  ```bash
  cargo package --list
  ```
  Verify the package only includes `src/`, `Cargo.toml`, `README.md`, `CHANGELOG.md`, `LICENSE-*`. No `tests/`, no `example-app/`, no `.github/`.

- [ ] **Step 13.3: Smoke-test example-app from a clean build**
  ```bash
  cd example-app
  cargo clean
  cargo run --bin example-app
  # Browser: http://localhost:8000 → click every button.
  ```

- [ ] **Step 13.4: Real-API integration check**
  ```bash
  export POLI_PAGE_API_KEY=pp_test_…
  cargo test -- --ignored
  # → 1 passing
  ```

- [ ] **Step 13.5: Tag v0.1.0**
  ```bash
  git tag -a v0.1.0 -m 'v0.1.0 — initial release'
  git push origin v0.1.0
  ```

- [ ] **Step 13.6: Publish dry-run** (when ready, separate decision)
  ```bash
  cargo publish --dry-run
  ```
  Inspect the staged tarball. If the SDK has stabilised, also:
  ```bash
  # Remove [patch.crates-io] from Cargo.toml.
  # Bump poli-page to the published stable version.
  cargo update -p poli-page
  cargo publish
  ```

---

## Summary — commit timeline

| Task | Commit message | Approx lines added |
|---|---|---|
| 1 | `chore: bootstrap Cargo.toml, lint, ci, smoke test` | ~200 |
| 2 | `feat: RFC 5987 filename encoding (port from symfony-bundle)` | ~100 |
| 3 | `feat: PdfResponse responder (bytes + stream)` | ~200 |
| 4 | `feat: PreviewResponse and DocumentRedirect responders` | ~150 |
| 5 | `feat: Responder impl for poli_page::Error (typed JSON map)` | ~200 |
| 6 | `feat: PoliPageClient state newtype + PoliPage<'r> request guard` | ~80 |
| 7 | `feat: PoliPageFairing (from_env, new, with_client)` | ~180 |
| 8 | `feat: finalise public lib.rs + tracing-bridge unit tests` | ~120 |
| 9 | `test: real-API integration test (env-gated, #[ignore])` | ~60 |
| 10 | `feat(example-app): all 10 SDK demo routes + render_to_file binary` | ~250 |
| 11 | `feat(example-app): interactive demo dashboard at /` | ~650 |
| 12 | `docs: README and CHANGELOG for v0.1.0` | ~180 |
| 13 | (operational; no commit) | 0 |

Total: ~2,370 lines across 12 PRs. Each reviewable in under 30 minutes. Comparable to symfony-bundle's ~3,000 lines and the next.js plan's ~2,180 lines — Rocket's tighter framework surface plus the SDK's heavier type re-export burden lands in between.

This document is the source of truth for execution order. If a PR's scope deviates from a Task, update this plan FIRST in the same PR, with reasoning in the description.
