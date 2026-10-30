//! Rocket example app for `poli-page-rocket`.
//!
//! The binary (`src/main.rs`) launches [`rocket()`]; the tests mount the
//! same routes with [`app`] and a fairing whose SDK client points at a
//! local fake of the API.

use poli_page_rocket::PoliPageFairing;
use rocket::fs::{relative, FileServer};
use rocket::{Build, Rocket};

pub mod routes;

/// The demo app: every route, mounted on a Rocket that `fairing` gives its
/// SDK client. The fairing's error handler answers the routes' SDK errors.
#[must_use]
pub fn app(fairing: PoliPageFairing) -> Rocket<Build> {
    rocket::build()
        .attach(fairing)
        .mount(
            "/",
            rocket::routes![
                routes::demo::index,
                routes::render::pdf,
                routes::render::stream,
                routes::render::file,
                routes::documents::create,
                routes::documents::get,
                routes::documents::delete,
                routes::documents::thumbnails,
                routes::documents::preview,
                routes::errors::not_found,
            ],
        )
        .mount("/static", FileServer::from(relative!("static")))
}

/// The app as the binary runs it: the SDK client is configured from
/// `POLI_PAGE_API_KEY` and the other `POLI_PAGE_*` variables.
#[must_use]
pub fn rocket() -> Rocket<Build> {
    app(PoliPageFairing::from_env())
}

#[cfg(test)]
mod tests;
