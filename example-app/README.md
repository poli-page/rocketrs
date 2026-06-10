# poli-page-rocket-example-app

Self-contained Rocket 0.5 demo for `poli-page-rocket`. Covers every SDK demo step through nine HTTP routes plus a standalone `render_to_file` binary.

## Run

```bash
POLI_PAGE_API_KEY=pp_test_...your-key... cargo run --bin example-app
```

Then open <http://localhost:8000> — interactive dashboard with one button per SDK feature.

Alternatively, drop the env vars in `/Users/mickael/Projects/.env` (workspace root); `main.rs` calls `dotenvy::from_path("../../.env")` so they load automatically. Shell-exported env vars always win.

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
| GET | `/render/preview` | `PreviewResponse` |
| POST | `/documents` | `Json<DocumentDescriptor>` |
| GET | `/documents/<id>` | `DocumentRedirect` (302 → presigned S3 URL) |
| GET | `/documents/<id>/preview` | `PreviewResponse` |
| GET | `/documents/<id>/thumbnails` | `Json<Vec<Thumbnail>>` |
| DELETE | `/documents/<id>` | 204 No Content |
| GET | `/errors/bad-version` | Triggers `INVALID_VERSION_FORMAT` → typed JSON 400 |
