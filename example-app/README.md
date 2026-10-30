# poli-page-rocket-example-app

Self-contained Rocket 0.5 demo for `poli-page-rocket`. Covers every SDK demo step through nine HTTP routes plus a standalone `render_to_file` binary.

## Run

```bash
POLI_PAGE_API_KEY=pp_sandbox_...your-key... cargo run --bin example-app
```

Then open <http://localhost:8000> — interactive dashboard with one button per SDK feature.

Alternatively, put the variables in a `.env` file next to the binary, or two directories up (the directory that holds the repository checkout): `main.rs` calls `dotenvy::from_path("../../.env")` and `dotenvy::from_path(".env")`, so they load automatically. Shell-exported variables always win.

## Standalone binary — SDK demo step 3

```bash
cargo run --bin render_to_file
# Writes /tmp/poli-page-rocketrs-demo.pdf
```

## Routes

| Method | Path | Returns |
|---|---|---|
| GET | `/` | Interactive HTML dashboard |
| GET | `/render/pdf` | `PdfResponse::bytes` |
| GET | `/render/stream` | `PdfResponse::stream` |
| POST | `/render/file` | `Json` `{ path, sizeBytes }` after `poli_page::render_to_file` |
| POST | `/documents` | `Json<DocumentDescriptor>` |
| GET | `/documents/<id>` | `DocumentRedirect` (302 → presigned S3 URL) |
| GET | `/documents/<id>/preview` | `PreviewResponse` |
| GET | `/documents/<id>/thumbnails` | `Json<Vec<Thumbnail>>` |
| DELETE | `/documents/<id>` | 204 No Content |
| GET | `/errors/not-found` | Renders template `does-not-exist`; the crate's error handler answers `404` with the SDK error's `to_payload()`: `{ code: "TEMPLATE_NOT_FOUND", message, status: 404, requestId }` |

## Tests

`cargo test` runs the route tests (`src/tests.rs`) and the API contract's `impl` cases (`tests/contract.rs`), which mount these routes with the SDK pointed at a local mock server. The contract cases need `POLI_CONTRACT=<path to contract.json>`; without it they are skipped (see the crate README, "Contract tests").
