//! HTML preview response.

use rocket::http::Header;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

/// `Responder` returning `text/html; charset=utf-8` with the conventional
/// `Cache-Control: private, no-store` and `X-Content-Type-Options: nosniff`
/// headers.
#[must_use]
pub struct PreviewResponse {
    html: String,
    cache_control: Option<String>,
}

impl PreviewResponse {
    /// Build a `PreviewResponse` from an owned HTML string.
    pub fn new(html: impl Into<String>) -> Self {
        Self {
            html: html.into(),
            cache_control: None,
        }
    }

    /// Override the default `Cache-Control: private, no-store`.
    pub fn cache_control(mut self, value: impl Into<String>) -> Self {
        self.cache_control = Some(value.into());
        self
    }
}

impl From<poli_page::PreviewResult> for PreviewResponse {
    fn from(r: poli_page::PreviewResult) -> Self {
        Self::new(r.html)
    }
}

impl From<poli_page::DocumentPreviewResult> for PreviewResponse {
    fn from(r: poli_page::DocumentPreviewResult) -> Self {
        Self::new(r.html)
    }
}

impl<'r> Responder<'r, 'static> for PreviewResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        let len = self.html.len();
        Response::build()
            .header(Header::new("Content-Type", "text/html; charset=utf-8"))
            .header(Header::new(
                "Cache-Control",
                self.cache_control
                    .unwrap_or_else(|| "private, no-store".into()),
            ))
            .header(Header::new("X-Content-Type-Options", "nosniff"))
            .sized_body(len, std::io::Cursor::new(self.html))
            .ok()
    }
}
