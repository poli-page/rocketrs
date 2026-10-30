//! Integration test against the live API (the SDK default base URL).
//!
//! Marked `#[ignore]` so the default `cargo test` skips it. Run with:
//!     cargo test -- --ignored
//! It only runs with a sandbox key (`POLI_PAGE_API_KEY=pp_sandbox_…`), so
//! it never renders in an environment whose PDFs count toward the quota;
//! with any other key, or none, it exits cleanly as a no-op (so PR
//! contributors without a key get green local runs even with `--ignored`).

use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError, PoliPageFairing};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes, State};
use serde_json::json;

#[get("/welcome.pdf")]
async fn welcome(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let bytes = client
        .render()
        .pdf(poli_page::RenderInput {
            project: "getting-started".into(),
            template: "welcome".into(),
            data: json!({ "name": "rocketrs integration test" }),
            ..Default::default()
        })
        .await?;
    Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
}

#[rocket::async_test]
#[ignore = "real API; opt-in via `cargo test -- --ignored` with POLI_PAGE_API_KEY=pp_sandbox_…"]
async fn render_welcome_against_live_api() {
    // A test-only guard against counted environments. It also covers the
    // CI case where the secret is absent and expands to an empty string.
    if !std::env::var("POLI_PAGE_API_KEY").is_ok_and(|k| k.starts_with("pp_sandbox_")) {
        eprintln!("POLI_PAGE_API_KEY is not a pp_sandbox_ key; skipping real-API test.");
        return;
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
