//! Integration test against the develop API.
//!
//! Marked `#[ignore]` so the default `cargo test` skips it. Run with:
//!     cargo test -- --ignored
//! Requires `POLI_PAGE_API_KEY` to be set; without it the test exits
//! cleanly as a no-op rather than failing (so PR contributors without a
//! key get green local runs even with `--ignored`).

use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError, PoliPageFairing};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes, State};
use serde_json::json;

#[get("/welcome.pdf")]
async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let bytes = client
        .render()
        .pdf(poli_page::ProjectModeInput {
            project: "getting-started".into(),
            template: "welcome".into(),
            version: Some("1.0.0".into()),
            data: json!({ "name": "rocketrs integration test" }),
            ..Default::default()
        })
        .await?;
    Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
}

#[rocket::async_test]
#[ignore = "real API; opt-in via `cargo test -- --ignored` with POLI_PAGE_API_KEY set"]
async fn render_welcome_against_live_api() {
    // Treat unset *and* empty as "no key": CI injects the secret via
    // `POLI_PAGE_API_KEY: ${{ secrets.POLI_PAGE_DEVELOP_API_KEY }}`, which
    // expands to an empty string when the secret is absent. `var()` returns
    // `Ok("")` for an empty-but-set var, so an `is_err()`-only guard would
    // let the test proceed and ignite the fairing with an invalid key.
    if std::env::var("POLI_PAGE_API_KEY").map_or(true, |k| k.trim().is_empty()) {
        eprintln!("POLI_PAGE_API_KEY not set; skipping real-API test.");
        return;
    }
    // Forward POLI_PAGE_TEST_BASE_URL into the env the fairing reads.
    if let Ok(v) = std::env::var("POLI_PAGE_TEST_BASE_URL") {
        std::env::set_var("POLI_PAGE_BASE_URL", v);
    }
    let r = rocket::build()
        .attach(PoliPageFairing::from_env())
        .mount("/", routes![welcome]);
    let c = Client::tracked(r).await.expect("rocket ignites");
    let resp = c.get("/welcome.pdf").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(
        resp.content_type().expect("content-type set").to_string(),
        "application/pdf",
    );
    let body = resp.into_bytes().await.expect("body bytes");
    assert!(
        body.len() > 1000,
        "PDF body should be > 1000 bytes; was {}",
        body.len()
    );
    assert_eq!(&body[..5], b"%PDF-");
}
