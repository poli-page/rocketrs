#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! Rocket.rs fairing and responders for the [Poli Page] PDF rendering API.
//!
//! Wraps the official [`poli_page`] SDK so a Rocket application can
//! `cargo add poli-page-rocket`, attach the fairing, and inject the SDK
//! client into routes via [`rocket::State`].
//!
//! ## Quick start
//!
//! ```no_run
//! use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError, PoliPageFairing};
//! use rocket::{get, routes, State};
//! use serde_json::json;
//!
//! #[get("/welcome.pdf")]
//! async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
//!     let bytes = client.render().pdf(poli_page::ProjectModeInput {
//!         project: "getting-started".into(),
//!         template: "welcome".into(),
//!         version: Some("1.0.0".into()),
//!         data: json!({ "name": "World" }),
//!         ..Default::default()
//!     }).await?;
//!     Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
//! }
//!
//! #[rocket::launch]
//! fn rocket() -> _ {
//!     rocket::build()
//!         .attach(PoliPageFairing::from_env())
//!         .mount("/", routes![welcome])
//! }
//! ```
//!
//! [Poli Page]: https://poli.page

pub mod errors;
pub mod fairing;
pub mod headers;
pub mod responses;
pub mod state;

mod tracing_bridge;

pub use errors::PoliPageError;
pub use fairing::PoliPageFairing;
pub use responses::{DocumentRedirect, PdfResponse, PreviewResponse};
pub use state::{PoliPage, PoliPageClient};

/// Selected SDK re-exports at the integration boundary so user code typically
/// only needs `use poli_page_rocket::...`. Mirrors the pattern from sibling
/// integrations.
pub use poli_page::{
    DocumentDescriptor, DocumentPreviewResult, Error, InlineModeInput, PoliPage as SdkPoliPage,
    PreviewResult, ProjectModeInput, RenderInput, RetryEvent, ThumbnailFormat, ThumbnailOptions,
};

#[doc(hidden)]
pub mod __internal_tracing_bridge {
    //! Exposed for unit tests only. Not part of the stable surface.
    pub use crate::tracing_bridge::{on_error, on_retry};
}
