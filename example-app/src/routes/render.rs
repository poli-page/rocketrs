use poli_page::ProjectModeInput;
use poli_page_rocket::{PdfResponse, PoliPageClient, PoliPageError, PreviewResponse};
use rocket::{get, State};
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
