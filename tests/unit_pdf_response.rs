//! `PdfResponse` produces the documented headers and body.

use bytes::Bytes;
use poli_page_rocket::responses::PdfResponse;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use rocket::{get, routes};

const PDF_STUB: &[u8] = b"%PDF-1.4\nstub\n";

#[get("/bytes")]
fn bytes_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB)).filename("invoice.pdf")
}

#[get("/bytes-inline")]
fn bytes_inline_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB))
        .filename("invoice.pdf")
        .inline()
}

#[get("/bytes-non-ascii")]
fn bytes_non_ascii_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB)).filename("café.pdf")
}

#[get("/bytes-no-filename")]
fn bytes_no_filename_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB))
}

#[get("/bytes-cache-override")]
fn bytes_cache_override_route() -> PdfResponse {
    PdfResponse::bytes(Bytes::from_static(PDF_STUB))
        .filename("x.pdf")
        .cache_control("public, max-age=60")
}

async fn client() -> Client {
    let rocket = rocket::build().mount(
        "/",
        routes![
            bytes_route,
            bytes_inline_route,
            bytes_non_ascii_route,
            bytes_no_filename_route,
            bytes_cache_override_route,
        ],
    );
    Client::tracked(rocket).await.unwrap()
}

#[rocket::async_test]
async fn pdf_response_sets_application_pdf_and_attachment() {
    let c = client().await;
    let r = c.get("/bytes").dispatch().await;
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(r.content_type(), Some(ContentType::PDF));
    assert_eq!(
        r.headers().get_one("content-disposition"),
        Some(r#"attachment; filename="invoice.pdf""#),
    );
    assert_eq!(
        r.headers().get_one("cache-control"),
        Some("private, no-store"),
    );
    assert_eq!(
        r.headers().get_one("x-content-type-options"),
        Some("nosniff")
    );
    assert_eq!(r.into_bytes().await.unwrap(), PDF_STUB);
}

#[rocket::async_test]
async fn pdf_response_uses_inline_when_requested() {
    let c = client().await;
    let r = c.get("/bytes-inline").dispatch().await;
    assert_eq!(
        r.headers().get_one("content-disposition"),
        Some(r#"inline; filename="invoice.pdf""#),
    );
}

#[rocket::async_test]
async fn pdf_response_rfc5987_encodes_non_ascii_filename() {
    let c = client().await;
    let r = c.get("/bytes-non-ascii").dispatch().await;
    assert_eq!(
        r.headers().get_one("content-disposition"),
        Some(r#"attachment; filename="caf_.pdf"; filename*=UTF-8''caf%C3%A9.pdf"#),
    );
}

#[rocket::async_test]
async fn pdf_response_omits_filename_when_none_given() {
    let c = client().await;
    let r = c.get("/bytes-no-filename").dispatch().await;
    assert_eq!(
        r.headers().get_one("content-disposition"),
        Some("attachment")
    );
}

#[rocket::async_test]
async fn pdf_response_honors_cache_control_override() {
    let c = client().await;
    let r = c.get("/bytes-cache-override").dispatch().await;
    assert_eq!(
        r.headers().get_one("cache-control"),
        Some("public, max-age=60")
    );
}
