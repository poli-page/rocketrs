use poli_page_rocket::responses::PreviewResponse;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes};

#[get("/preview")]
fn preview_route() -> PreviewResponse {
    PreviewResponse::new("<h1>Hi</h1>")
}

#[get("/preview-cache-override")]
fn preview_cache_override_route() -> PreviewResponse {
    PreviewResponse::new("<h1>Hi</h1>").cache_control("public, max-age=300")
}

async fn client() -> Client {
    let r = rocket::build().mount("/", routes![preview_route, preview_cache_override_route]);
    Client::tracked(r).await.unwrap()
}

#[rocket::async_test]
async fn preview_sets_text_html_and_no_store() {
    let c = client().await;
    let r = c.get("/preview").dispatch().await;
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(
        r.headers().get_one("content-type"),
        Some("text/html; charset=utf-8"),
    );
    assert_eq!(
        r.headers().get_one("cache-control"),
        Some("private, no-store")
    );
    assert_eq!(
        r.headers().get_one("x-content-type-options"),
        Some("nosniff")
    );
    assert_eq!(r.into_string().await.unwrap(), "<h1>Hi</h1>");
}

#[rocket::async_test]
async fn preview_honors_cache_control_override() {
    let c = client().await;
    let r = c.get("/preview-cache-override").dispatch().await;
    assert_eq!(
        r.headers().get_one("cache-control"),
        Some("public, max-age=300")
    );
}
