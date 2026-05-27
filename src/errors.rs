//! `Responder` impl for the SDK's `poli_page::Error`, via the local
//! `PoliPageError` newtype.
//!
//! The orphan rule blocks `impl Responder for poli_page::Error` directly
//! (both trait and type are foreign), so routes return
//! `Result<T, PoliPageError>` and `?` runs the `From<poli_page::Error>`
//! conversion. Mapping is documented in spec §10.1.

use std::io::Cursor;

use poli_page::Error;
use rocket::http::{Header, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

/// Local wrapper around [`poli_page::Error`] that implements
/// [`Responder`], producing a typed JSON `{ code, message, requestId }`
/// body with the documented status mapping.
///
/// Routes opt in by writing `Result<T, PoliPageError>`; the `?` operator
/// converts a `poli_page::Error` via the [`From`] impl below.
#[derive(Debug)]
pub struct PoliPageError(pub Error);

impl From<Error> for PoliPageError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}

impl<'r> Responder<'r, 'static> for PoliPageError {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        let status = status_for(&self.0);
        let body = serde_json::json!({
            "code": self.0.code(),
            "message": self.0.to_string(),
            "requestId": self.0.request_id(),
        });
        let bytes = serde_json::to_vec(&body).map_err(|_| Status::InternalServerError)?;
        Response::build()
            .status(status)
            .header(Header::new(
                "Content-Type",
                "application/json; charset=utf-8",
            ))
            .header(Header::new("Cache-Control", "private, no-store"))
            .sized_body(bytes.len(), Cursor::new(bytes))
            .ok()
    }
}

// Explicit arms document the spec mapping; the wildcard exists for forward
// compat with #[non_exhaustive]. Both intentionally map to 500.
#[allow(clippy::match_same_arms)]
fn status_for(err: &Error) -> Status {
    match err {
        Error::BadRequest { status, .. }
        | Error::Auth { status, .. }
        | Error::PermissionDenied { status, .. }
        | Error::NotFound { status, .. }
        | Error::Gone { status, .. }
        | Error::RateLimited { status, .. }
        | Error::Api { status, .. } => {
            Status::from_code(*status).unwrap_or(Status::InternalServerError)
        }
        Error::Connection { .. } | Error::Download { .. } => Status::BadGateway,
        Error::Timeout { .. } => Status::GatewayTimeout,
        Error::Aborted => Status::ServiceUnavailable,
        Error::InvalidOptions { .. } | Error::Internal { .. } => Status::InternalServerError,
        // poli_page::Error is #[non_exhaustive]; map any future variant to 500
        // so downstream builds don't break on SDK additions. Re-evaluate when
        // the SDK adds a variant we care about.
        _ => Status::InternalServerError,
    }
}
