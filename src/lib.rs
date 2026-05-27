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
//! [Poli Page]: https://poli.page

pub mod headers;
pub mod responses;
