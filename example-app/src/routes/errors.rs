use poli_page::RenderInput;
use poli_page_rocket::{PoliPageClient, PoliPageError};
use rocket::{get, State};
use serde_json::json;

/// Renders a template that does not exist: the API answers
/// `404 TEMPLATE_NOT_FOUND`, the SDK returns `Error::NotFound`, and the
/// crate's `PoliPageError` responder turns it into a typed JSON `404`
/// carrying `code`, `status`, `message` and `requestId`.
#[get("/errors/not-found")]
pub async fn not_found(client: &State<PoliPageClient>) -> Result<&'static str, PoliPageError> {
    client
        .render()
        .pdf(RenderInput {
            project: "getting-started".into(),
            template: "does-not-exist".into(),
            data: json!({}),
            ..Default::default()
        })
        .await?;
    Ok("unreachable")
}
