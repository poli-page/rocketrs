use poli_page::{ProjectModeInput, ThumbnailOptions};
use poli_page_rocket::{DocumentRedirect, PoliPageClient, PoliPageError, PreviewResponse};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, State};
use serde_json::{json, Value};

#[post("/documents")]
pub async fn create(client: &State<PoliPageClient>) -> Result<Json<Value>, PoliPageError> {
    let descriptor = client
        .render()
        .document(ProjectModeInput {
            project: "getting-started".into(),
            template: "welcome".into(),
            version: Some("1.0.0".into()),
            data: json!({ "name": "Stored doc" }),
            ..Default::default()
        })
        .await?;
    Ok(Json(json!({
        "documentId": descriptor.document_id,
        "pageCount": descriptor.page_count,
        "sizeBytes": descriptor.size_bytes,
    })))
}

#[get("/documents/<id>")]
pub async fn get(
    id: &str,
    client: &State<PoliPageClient>,
) -> Result<DocumentRedirect, PoliPageError> {
    let descriptor = client.documents().get(id).await?;
    Ok(DocumentRedirect::to(&descriptor.presigned_pdf_url))
}

#[delete("/documents/<id>")]
pub async fn delete(id: &str, client: &State<PoliPageClient>) -> Result<Status, PoliPageError> {
    client.documents().delete(id).await?;
    Ok(Status::NoContent)
}

#[get("/documents/<id>/thumbnails")]
pub async fn thumbnails(
    id: &str,
    client: &State<PoliPageClient>,
) -> Result<Json<Value>, PoliPageError> {
    let result = client
        .documents()
        .thumbnails(id, ThumbnailOptions::new(200))
        .await?;
    // Thumbnail is Deserialize-only in the SDK; map by hand.
    let value = json!(result
        .into_iter()
        .map(|t| json!({
            "page": t.page,
            "width": t.width,
            "height": t.height,
            "contentType": t.content_type,
            "data": t.data,
        }))
        .collect::<Vec<_>>());
    Ok(Json(value))
}

#[get("/documents/<id>/preview")]
pub async fn preview(
    id: &str,
    client: &State<PoliPageClient>,
) -> Result<PreviewResponse, PoliPageError> {
    let result = client.documents().preview(id).await?;
    Ok(PreviewResponse::from(result))
}
