//! Interactive demo dashboard at `GET /`. Ports the symfony-bundle's
//! `templates/demo.html` byte-for-byte (URL paths match our routes 1:1).
//!
//! `include_str!` resolves at compile time so the binary is self-contained;
//! editing `static/index.html` requires a rebuild. For hot-reload during
//! development the file is also served via `FileServer::from(relative!("static"))`
//! at `/static/index.html` (see `main.rs`).

use rocket::get;
use rocket::response::content::RawHtml;

const INDEX_HTML: &str = include_str!("../../static/index.html");

#[get("/")]
pub fn index() -> RawHtml<&'static str> {
    RawHtml(INDEX_HTML)
}
