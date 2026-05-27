//! Managed-state newtype + optional `PoliPage<'r>` request guard.

use rocket::request::{FromRequest, Outcome, Request};

/// Newtype wrapping the SDK client for Rocket's managed state.
///
/// `Clone` is cheap — the SDK's `PoliPage` is `Arc`-internal, so cloning is
/// an atomic refcount bump.
#[derive(Clone)]
#[must_use]
pub struct PoliPageClient(pub poli_page::PoliPage);

impl PoliPageClient {
    /// Borrow the underlying SDK client.
    pub fn client(&self) -> &poli_page::PoliPage {
        &self.0
    }

    /// Borrow the `render` namespace (`pdf`, `pdf_stream`, `preview`, `document`).
    pub fn render(&self) -> &poli_page::Render {
        &self.0.render
    }

    /// Borrow the `documents` namespace (`get`, `preview`, `thumbnails`, `delete`).
    pub fn documents(&self) -> &poli_page::Documents {
        &self.0.documents
    }
}

/// Optional sugar: a request guard that resolves to `&poli_page::PoliPage`.
///
/// Routes can either take `&State<PoliPageClient>` (the canonical Rocket
/// pattern) or `PoliPage<'_>` (less typing). Both work.
pub struct PoliPage<'r>(pub &'r poli_page::PoliPage);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for PoliPage<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(c) = req.rocket().state::<PoliPageClient>() {
            Outcome::Success(PoliPage(&c.0))
        } else {
            tracing::error!(
                target: "poli_page_rocket",
                "PoliPageFairing is not attached; PoliPage<'_> guard cannot resolve.",
            );
            // Infallible has no value to construct, so Forward is the only
            // option here. In a correctly-configured app this branch is
            // unreachable; logging plus a 500 surfaces the misconfig.
            Outcome::Forward(rocket::http::Status::InternalServerError)
        }
    }
}
