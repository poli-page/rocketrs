use poli_page::ProjectModeInput;
use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError, PreviewResponse};
use rocket::serde::json::Json;
use rocket::{get, post, State};
use serde::Serialize;
use serde_json::json;

fn input() -> ProjectModeInput {
    ProjectModeInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        version: Some("1.0.0".into()),
        data: json!({ "name": "World" }),
        ..Default::default()
    }
}

#[get("/render/pdf")]
pub async fn pdf(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let bytes = client.render().pdf(input()).await?;
    Ok(PdfResponse::bytes(bytes).filename("welcome.pdf").inline())
}

#[get("/render/stream")]
pub async fn stream(client: &State<PoliPageClient>) -> Result<PdfResponse, PoliPageError> {
    let stream = client.render().pdf_stream(input()).await?;
    Ok(PdfResponse::stream(stream).filename("welcome.pdf").inline())
}

#[get("/render/preview")]
pub async fn preview(client: &State<PoliPageClient>) -> Result<PreviewResponse, PoliPageError> {
    let result = client.render().preview(input()).await?;
    Ok(PreviewResponse::from(result))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFileResult {
    path: String,
    size_bytes: u64,
}

/// Demo step 3: `poli_page::render_to_file` — stream the PDF straight to disk,
/// memory-bounded regardless of size.
#[post("/render/file")]
pub async fn file(client: &State<PoliPageClient>) -> Result<Json<RenderFileResult>, PoliPageError> {
    let path = std::path::Path::new("output").join("welcome.pdf");

    poli_page::render_to_file(
        client.client(),
        ProjectModeInput {
            project: "getting-started".into(),
            template: "welcome".into(),
            version: Some("1.0.0".into()),
            data: json!({ "name": "render_to_file demo" }),
            ..Default::default()
        },
        &path,
    )
    .await?;

    let size_bytes = tokio::fs::metadata(&path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(Json(RenderFileResult {
        path: path.to_string_lossy().into_owned(),
        size_bytes,
    }))
}
