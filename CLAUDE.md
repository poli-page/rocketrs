# CLAUDE.md

> Instructions for Claude Code agents working in `poli-page/rocketrs`.

## 1. Repo at a glance

| Field        | Value |
| ------------ | ----- |
| Repository   | `poli-page/rocketrs` |
| Type         | Framework integration (Rocket.rs fairing + responders) |
| Language     | Rust (edition 2021) |
| MSRV         | `1.75` (Rocket 0.5 minimum) |
| Rocket       | `^0.5` (only stable line as of 2026-05) |
| Registry     | crates.io — `poli-page-rocket` |
| Depends on   | `poli-page` (crates.io, `^1.0.0-rc.1`) |
| Roadmap slot | gap-fill — Rust SDK didn't exist when `INTEGRATIONS_PLAN.md` was written |

**Source-of-truth docs (read first):**
- `docs/spec/rocket-crate-specification.md` — full design spec for v0.1.0
- `docs/plan/2026-05-27-implementation.md` — step-by-step plan
- `/Users/mickael/Projects/INTEGRATIONS_PLAN.md` — cross-repo umbrella, esp. §"Cross-cutting DX patterns"
- `/Users/mickael/Projects/symfony-bundle/CLAUDE.md` and `nextjs/CLAUDE.md` and `nestjs/CLAUDE.md` — sibling integrations; reuse decisions

## 2. The crate's job

A thin Rocket-flavored wrapper around the official Poli Page Rust SDK (`poli-page`, source at `/Users/mickael/Projects/sdk-rust/`). It provides:

- A **`PoliPageFairing`** that builds a `PoliPage` client at ignite-time and inserts it into Rocket's managed state.
- A typed wrapper `PoliPageClient` in managed state (newtype over the SDK's `PoliPage`) so routes read it via `&State<PoliPageClient>` (or the sugar request guard `PoliPage<'_>`).
- Three `Responder<'r, 'static>` types — **`PdfResponse`**, **`PreviewResponse`**, **`DocumentRedirect`** — that set the correct headers (Content-Type, RFC 5987 Content-Disposition, Cache-Control: `private, no-store`, X-Content-Type-Options: `nosniff`).
- A `Responder` impl for the SDK's `poli_page::Error` that maps to a typed JSON response (4xx → same status; 5xx → same; network/timeout → 502).
- Bridging of the SDK's `on_retry` / `on_error` `Fn` hooks into `tracing` events under the `poli_page_rocket` target.
- An `example-app/` Rocket 0.5 project mirroring the symfony-bundle's 10-step demo, served at `GET /` with the same interactive dashboard UI.

**This crate does NOT** reimplement HTTP transport, retries, error classification, idempotency keys, presigned-URL handling, or `reqwest::Client` plumbing — the SDK already owns that. Bug in those areas? Fix it in `sdk-rust`, not here.

**This crate does NOT** ship: a blocking-feature surface, a `rocket_contrib`-style template helper, a database integration, Swagger generation, or a `cargo` subcommand. See spec §17.

## 3. Working language

- **Code, comments, file names, commit messages, PR descriptions, repository documentation**: English.
- **Day-to-day conversation with Mickael/Xavier**: French, tutoiement.
- **Conversation in this Claude Code session**: French is fine for the chat; artifacts stay English.

## 4. TDD is mandatory

RED → GREEN → refactor for every change. Tests live in `tests/` (Rust's built-in `#[test]` harness) under three buckets:
- `tests/unit_*.rs` — pure-function unit tests (header encoding, error mapping, response shape) — no Rocket boot, run in milliseconds.
- `tests/fairing_*.rs` — Rocket boots a `rocket::local::asynchronous::Client`, attaches the fairing with a stub config, asserts state extraction and route wiring.
- `tests/integration_render.rs` — single happy-path test against `api-develop.poli.page`. Skipped via `#[ignore]` + runtime env check (see §10.4).

### What to test (integration-specific!)

- **Fairing ignite**: `PoliPageFairing::from_env()` reads `POLI_PAGE_API_KEY` and inserts a `PoliPageClient` into managed state. `PoliPageFairing::new(builder)` accepts a fully-configured builder. Invalid key → ignite fails with the documented `tracing::error!` and Rocket's `Failure` outcome.
- **State extraction**: a `#[get("/")] fn route(client: &State<PoliPageClient>)` resolves the client through `Client::dispatch`.
- **`Responder` impls**: each of `PdfResponse` / `PreviewResponse` / `DocumentRedirect` produces a `Response` with the right `Content-Type`, RFC 5987 `Content-Disposition`, `Cache-Control: private, no-store`, `X-Content-Type-Options: nosniff`. Cover ASCII and non-ASCII filenames (port the symfony-bundle's filename-encoding cases verbatim).
- **`Responder for poli_page::Error`**: every `Error::*` variant maps to the expected status (`BadRequest` → 400, `Auth` → 401, `PermissionDenied` → 403, `NotFound` → 404, `Gone` → 410, `RateLimited` → 429, `Api` → status pass-through, `Connection` / `Timeout` / `Aborted` / `InvalidOptions` → 502, `Download` → 502, `Internal` → 500). Body is `{ code, message, requestId }` JSON. `Cache-Control: private, no-store`.
- **Tracing bridge**: `on_retry` and `on_error` hooks emit a `tracing::warn!` (retry) / `tracing::error!` (terminal) under target `poli_page_rocket` with the fields documented in spec §10.

### What NOT to test (the SDK already does)

- HTTP transport behavior (reqwest / hyper / TLS edge cases).
- Retry policy (exponential backoff, max attempts, `Retry-After` parsing, jitter, never-retry-4xx).
- 4xx / 5xx → `Error` mapping inside the SDK.
- Idempotency-Key auto-generation.
- Stream chunking / `PdfByteStream` correctness.
- API contract drift — the SDK's `tests/` suite owns that (wiremock-based).

Re-testing these here doubles maintenance burden. **If you find yourself reaching for `wiremock`, stop — you're doing the SDK's job.**

## 5. Robustness over shortcuts

Mickael's hard rule (validated across symfony / next / nest): **no hacks to make a test pass or a corner case go away**. Fix root causes. If a workaround is genuinely required (framework bug, SDK quirk), document it inline with a `// Why:` comment naming the constraint.

Concretely:
- **No `.unwrap()` in `src/`** — every error path returns `Result<T, poli_page::Error>` or a typed local error. Tests are free to `unwrap`.
- **No `#[allow(clippy::...)]` in `src/`** to silence warnings. Fix the cause. The crate-level allow list (rare) lives in `lib.rs` with a comment.
- **No `.expect("can't happen")`** without a `// Why:` proof in the comment that it really can't.

## 6. Code conventions

- **Rust 2021**, MSRV `1.75` (Rocket 0.5 baseline).
- **rustfmt** default settings (no custom `rustfmt.toml` beyond a `max_width = 100` if useful — match `sdk-rust`'s config first).
- **clippy** with `#![warn(clippy::pedantic)]` at the crate root, denied in CI via `-D warnings`. Allow-list lives in `lib.rs` with one-line reasons per allow.
- **`#![forbid(unsafe_code)]`** at the crate root.
- **`#![warn(missing_docs)]`** — every public item carries a doc comment.
- **No commented-out code, no `TODO` without a linked issue, no debug prints (`dbg!`, `println!`) in committed code.**
- **Default to no comments.** Add one only when the *why* is non-obvious. Comments restating *what* the code does are noise.

## 7. Commits and PRs

- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`.
- **One concern per PR**, reviewable in under 30 minutes.
- PR description: what changed, why, how it was tested.
- CI must be green before merge.

## 8. CI

Workflow: `.github/workflows/ci.yml`. Matrix: Rust `stable` / `beta` / `1.75` (MSRV) on `ubuntu-latest`. Each step auto-skips if the relevant file is missing (so a freshly scaffolded repo is green from day one). Don't change that behaviour.

Local mirror:
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

## 9. Unpublished-SDK note

The Rust SDK `poli-page` is at `1.0.0-rc.1` on crates.io (and locally at `/Users/mickael/Projects/sdk-rust/`). For dev against unreleased SDK changes, use a **Cargo path override** in `~/.cargo/config.toml` OR a `[patch.crates-io]` block in this repo's **dev-only `Cargo.toml`** addition. The crate's published `Cargo.toml` keeps `poli-page = "1.0.0-rc.1"` (or pinned newer once stable), unmodified.

Two clean primitives for the dev override:

1. **Cargo path dependency override in `[patch.crates-io]`**:
   ```toml
   [patch.crates-io]
   poli-page = { path = "../sdk-rust" }
   ```
   Lives in this repo's `Cargo.toml` while the SDK is `1.0.0-rc.*`. Removed on the same PR that bumps the requirement to a published stable release.

2. **`.cargo/config.toml`** with `paths = ["../sdk-rust"]` — workspace-level, no change to `Cargo.toml`. Use this if a clean published manifest matters before stable.

Either way, the integration's published `Cargo.toml` is correct from day one. See spec §12.

## 10. Known gotchas (battle-tested — don't relearn the hard way)

These caught us once in sibling integrations or surface from Rocket / Tokio specifics. Recorded so future agents don't burn a session rediscovering them.

### 10.1 Use `rocket::local::asynchronous::Client`, never `blocking::Client`

`cargo test` runs tests in parallel by default. Rocket's `blocking::Client` spins up its own Tokio runtime per test — under parallel execution that compounds into "runtime within runtime" panics and lingering tasks across tests. The async client cooperates with the test's own `#[tokio::test]` runtime cleanly. The client is dropped at the end of each test; Tokio handles teardown when the runtime drops.

**Do NOT** mix `#[rocket::async_test]` and `#[tokio::test]` in the same file — pick one, document the choice, and apply across siblings.

### 10.2 Nothing in `src/` may `.unwrap()` or `.expect()`

Library code is consumed in long-running async services where a panic kills the worker. Every fallible path returns `Result`. Tests are free to unwrap. The `clippy::unwrap_used` and `clippy::expect_used` lints are `deny`-ed for `src/` only (configured under `[lints.clippy]` in `Cargo.toml`).

### 10.3 `Responder` for `poli_page::Error` is opt-in, not global

Rocket's catcher system is per-status-code, not per-error-type, so we can't auto-catch `poli_page::Error` the way NestJS's global exception filter does. Instead, **routes opt in** by returning `Result<PdfResponse, poli_page::Error>` (the SDK's error type implements `Responder` thanks to our crate). The `?` operator does the work.

If a route uses `let pdf = client.render.pdf(...).await.map_err(|_| ...)?` to convert to a different error type, the SDK error never reaches our `Responder` impl and the user owns the response shape. This is the documented behaviour and matches Next.js's "only `PoliPageError` gets mapped" rule (§10.4 of `nextjs/CLAUDE.md`).

**Do NOT** widen this to a Rocket `#[catch(default)]` — it would swallow every error type from every route and destroy observability. Same trade-off the symfony/next/nest sessions resolved on.

### 10.4 Single root `.env`, no per-app `.env.local`

The example app's `main.rs` loads the workspace root `.env` (`/Users/mickael/Projects/.env`) at boot via `dotenvy::from_path_override(...)` — real env vars (shell exports) still win. **Do NOT** use the legacy `dotenv` crate (unmaintained since 2020); `dotenvy` is the maintained fork.

**Do NOT** introduce a `.env.local` in `example-app/` or instruct users to `cp .env .env.local`. This was an explicit hard requirement from Mickael during the symfony-bundle session. See `INTEGRATIONS_PLAN.md` §"Cross-cutting DX patterns" §2.

### 10.5 No CLI beyond `cargo run` on the example app

Rocket has no per-app command-line entry point that user code attaches to (the `rocket` CLI doesn't exist; `cargo run` is the launcher). The example-app's `cargo run --bin example-app` IS the smoke test. The SDK's `render_to_file` helper becomes a standalone binary at `example-app/src/bin/render_to_file.rs` (run via `cargo run --bin render_to_file`), not a Rocket route. **Don't try to invent a CLI** — match the next.js / nest.js stance (§10.6 of `nextjs/CLAUDE.md`, §10.4 of `nestjs/CLAUDE.md`).

### 10.6 The interactive demo UI is mandatory, not optional

`GET /` in the example app returns a single-page HTML dashboard with one button per SDK feature, inline `<iframe>` previews, JSON pretty-print, and a document-lifecycle state machine in client JS. Aesthetic copied from `/Users/mickael/Projects/symfony-bundle/example-app/templates/demo.html` (white surface, indigo `#4f5d99`, Manrope + IBM Plex Sans + JetBrains Mono).

Implementation: a static HTML file under `example-app/static/index.html` served via Rocket's `FileServer::from(relative!("static"))`, OR an inline `const DEMO_HTML: &str = include_str!("../static/index.html");` returned from `#[get("/")] fn demo() -> RawHtml<&'static str>`. Either works — prefer `include_str!` so the binary is self-contained.

### 10.7 Integration test gating

Single real-API test at `tests/integration_render.rs`, marked `#[ignore]` by default so the default `cargo test` is fast and offline. Run with `cargo test -- --ignored` when `POLI_PAGE_API_KEY` is set. Inside the test, a runtime check `if std::env::var("POLI_PAGE_API_KEY").is_err() { return; }` exits cleanly so `cargo test -- --ignored` without the key is a no-op rather than a failure. See spec §13.2.

## 11. When stuck

- Re-read `docs/spec/rocket-crate-specification.md` first; most "open questions" are answered there or in §18 "Resolved decisions".
- Compare with `sdk-rust` at `/Users/mickael/Projects/sdk-rust/`.
- Compare with `@poli-page/nextjs`, `@poli-page/nestjs`, and the Symfony bundle — same product, different framework, decisions you can copy directly.
- Look at industry benchmarks: `sentry-rust`'s `sentry-rocket` crate (closest in shape — third-party SDK + Rocket fairing), `rocket_db_pools` (managed state pattern), `rocket_cors` (fairing pattern), `rocket-okapi` (responder + catcher pattern). The bar.
- Ask Mickael early. A two-line message is faster than a half-day rebuilding the wrong thing.
- If a CI failure looks unrelated to your change, check `main` first before assuming you caused it.
