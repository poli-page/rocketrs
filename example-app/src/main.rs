// rocket::Error is large (~224 bytes) but #[rocket::main] dictates the
// return type, so the only fix would be a manual main() wrapper. Not
// worth it for an example app.
#![allow(clippy::result_large_err)]

//! Rocket example app for `poli-page-rocket`.
//!
//! Run with:
//! ```bash
//! POLI_PAGE_API_KEY=pp_sandbox_... cargo run --bin example-app
//! ```
//!
//! Or put the variables in a `.env` file, next to the binary or two
//! directories up (the directory that holds the repository checkout), and
//! `cargo run --bin example-app`: `dotenvy::from_path` is called before
//! `rocket::build()`, so the file fills in any variable the shell has not
//! already exported (the shell wins).

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ = dotenvy::from_path("../../.env");
    let _ = dotenvy::from_path(".env");

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,poli_page_rocket=debug".into()),
        )
        .init();

    poli_page_rocket_example_app::rocket()
        .launch()
        .await
        .map(|_| ())
}
