use poli_page::RenderInput;
use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError};
use rocket::serde::json::Json;
use rocket::{get, post, State};
use serde::Serialize;
use serde_json::json;

fn input() -> RenderInput {
    RenderInput {
        project: "getting-started".into(),
        template: "welcome".into(),
        data: json!({ "name": "SDK Demo" }),
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
        RenderInput {
            project: "getting-started".into(),
            template: "welcome".into(),
            data: json!({ "name": "SDK Demo" }),
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
