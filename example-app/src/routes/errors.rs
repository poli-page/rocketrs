use poli_page::ProjectModeInput;
use poli_page_rocket::{PoliPageClient, PoliPageError};
use rocket::{get, State};
use serde_json::json;

/// Triggers `INVALID_VERSION_FORMAT` — the API rejects the version
/// selector, the SDK returns `Error::BadRequest`, and the crate's
/// `Responder` impl maps it to a typed JSON `400` response.
#[get("/errors/bad-version")]
pub async fn bad_version(client: &State<PoliPageClient>) -> Result<&'static str, PoliPageError> {
    client
        .render()
        .pdf(ProjectModeInput {
            project: "getting-started".into(),
            template: "welcome".into(),
            version: Some("not-a-version".into()),
            data: json!({}),
            ..Default::default()
        })
        .await?;
    Ok("unreachable")
}
