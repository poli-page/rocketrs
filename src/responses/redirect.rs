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
        Self {
            url: url.into(),
            permanent: false,
        }
    }

    /// Upgrade to a 308 permanent redirect.
    pub fn permanent(mut self) -> Self {
        self.permanent = true;
        self
    }
}

impl From<&poli_page::DocumentDescriptor> for DocumentRedirect {
    fn from(d: &poli_page::DocumentDescriptor) -> Self {
        Self::to(&d.presigned_pdf_url)
    }
}

impl<'r> Responder<'r, 'static> for DocumentRedirect {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .status(if self.permanent {
                Status::PermanentRedirect
            } else {
                Status::Found
            })
            .header(Header::new("Location", self.url))
            .header(Header::new("Cache-Control", "private, no-store"))
            .ok()
    }
}
