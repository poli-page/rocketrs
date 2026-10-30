//! The crate's error handler: the `Responder` impl for the SDK's
//! `poli_page::Error`, via the local `PoliPageError` newtype.
//!
//! The orphan rule blocks `impl Responder for poli_page::Error` directly
//! (both trait and type are foreign), so routes return
//! `Result<T, PoliPageError>` and `?` runs the `From<poli_page::Error>`
//! conversion.
//!
//! The handler is on by default. [`PoliPageFairing::error_handler(false)`]
//! turns it off: the responder then hands the status to Rocket's catchers
//! instead of writing the JSON body.
//!
//! [`PoliPageFairing::error_handler(false)`]: crate::PoliPageFairing::error_handler

use std::io::Cursor;

use poli_page::Error;
use rocket::http::{Header, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

/// Whether the error handler writes the JSON body. The fairing puts it in
/// managed state at ignite; when it is absent the handler is on.
pub(crate) struct ErrorHandlerSetting {
    pub(crate) enabled: bool,
}

/// Local wrapper around [`poli_page::Error`] that implements [`Responder`]:
/// the crate's error handler.
///
/// The answer has the status of the SDK's canonical payload,
/// [`poli_page::Error::to_payload`] (the API status, `503` for
/// `network_error`, `504` for `timeout`), or `500` when that status is
/// `null`. Its body is that payload as JSON, `{ code, message, status,
/// requestId }` (`status` and `requestId` are `null` when unknown), with
/// `Content-Type: application/json; charset=utf-8` and `Cache-Control:
/// private, no-store`.
///
/// When the fairing was built with
/// [`error_handler(false)`](crate::PoliPageFairing::error_handler), the
/// responder writes no body and returns that status as an error, so
/// Rocket's catcher for the status (yours, or Rocket's default) answers.
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
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let payload = self.0.to_payload();
        let status = payload
            .status
            .and_then(Status::from_code)
            .unwrap_or(Status::InternalServerError);
        let enabled = req
            .rocket()
            .state::<ErrorHandlerSetting>()
            .is_none_or(|setting| setting.enabled);
        if !enabled {
            return Err(status);
        }
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
