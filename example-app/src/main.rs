// rocket::Error is large (~224 bytes) but #[rocket::main] dictates the
// return type, so the only fix would be a manual main() wrapper. Not
// worth it for an example app.
#![allow(clippy::result_large_err)]

//! Rocket example app for `poli-page-rocket`.
//!
//! Run with:
//! ```bash
//! POLI_PAGE_API_KEY=pp_test_... cargo run --bin example-app
//! ```
//!
//! Or set the env vars in `/Users/mickael/Projects/.env` (workspace root)
//! and `cargo run --bin example-app` — `dotenvy::from_path` is called
//! before `rocket::build()` so the file populates `std::env` for any
//! var the shell hasn't already exported (12-factor: shell wins).

use poli_page_rocket::PoliPageFairing;
use rocket::fs::{relative, FileServer};

mod routes;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // Workspace-root .env (one above the crate root) is the source of truth
    // per CLAUDE.md §10.4. `.env` next to the binary is also accepted.
    let _ = dotenvy::from_path("../../.env");
    let _ = dotenvy::from_path(".env");

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,poli_page_rocket=debug".into()),
        )
        .init();

    rocket::build()
        .attach(PoliPageFairing::from_env())
        .mount(
            "/",
            rocket::routes![
                routes::demo::index,
                routes::render::pdf,
                routes::render::stream,
                routes::render::preview,
                routes::documents::create,
                routes::documents::get,
                routes::documents::delete,
                routes::documents::thumbnails,
                routes::documents::preview,
                routes::errors::bad_version,
            ],
        )
        .mount("/static", FileServer::from(relative!("static")))
        .launch()
        .await
        .map(|_| ())
}
