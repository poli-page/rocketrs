//! SDK demo step 3 — render to a file via the SDK's free function.
//!
//! Run with:
//! ```bash
//! POLI_PAGE_API_KEY=pp_test_... cargo run --bin render_to_file
//! ```

use poli_page::{PoliPage, ProjectModeInput};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::from_path("../../.env");
    let _ = dotenvy::from_path(".env");

    let api_key = std::env::var("POLI_PAGE_API_KEY")?;
    let base_url = std::env::var("POLI_PAGE_BASE_URL")
        .unwrap_or_else(|_| "https://api-develop.poli.page".into());
    let client = PoliPage::builder()
        .api_key(api_key)
        .base_url(base_url)
        .build()?;
    let path = "/tmp/poli-page-rocketrs-demo.pdf";
    poli_page::render_to_file(
        &client,
        ProjectModeInput {
            project: "getting-started".into(),
            template: "welcome".into(),
            version: Some("1.0.0".into()),
            data: json!({ "name": "render_to_file demo" }),
            ..Default::default()
        },
        path,
    )
    .await?;
    println!("Wrote {path}");
    Ok(())
}
