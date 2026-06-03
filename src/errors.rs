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
/// [`Responder`], producing a typed JSON
/// `{ code, message, status, requestId }` body with HTTP status sourced
/// from the SDK's canonical payload (503 for network, 504 for timeout,
/// the upstream HTTP status otherwise).
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
        let payload = self.0.to_payload();
        let status = payload
            .status
            .and_then(Status::from_code)
            .unwrap_or(Status::InternalServerError);
        let bytes = serde_json::to_vec(&payload).map_err(|_| Status::InternalServerError)?;
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
