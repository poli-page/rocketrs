//! The error handler: on by default with the fairing, off with
//! `error_handler(false)`, and its body is exactly the SDK's `to_payload()`.

use std::time::Duration;

use poli_page::Error;
use poli_page_rocket::{PoliPageError, PoliPageFairing};
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use rocket::{get, routes};
use serde_json::{json, Value};

const KEY: &str = "pp_sandbox_0123456789abcdef0123456789abcdef";

fn error(variant: &str) -> Error {
    match variant {
        "template-not-found" => Error::NotFound {
            status: 404,
            code: "TEMPLATE_NOT_FOUND".into(),
            message: "Template 'does-not-exist' not found in project 'getting-started'.".into(),
            request_id: Some("req_404".into()),
            source: None,
        },
        "network" => Error::Connection {
            message: "Network request failed".into(),
            source: Box::<dyn std::error::Error + Send + Sync>::from("connection refused"),
        },
        "timeout" => Error::Timeout {
            timeout: Duration::from_millis(1500),
        },
        "internal" => Error::Internal {
            message: "unexpected answer".into(),
            status: None,
        },
        other => panic!("unknown variant {other}"),
    }
}

#[get("/fail/<variant>")]
fn fail(variant: &str) -> Result<&'static str, PoliPageError> {
    Err(error(variant).into())
}

fn sdk() -> poli_page::PoliPage {
    poli_page::PoliPage::builder().api_key(KEY).build().unwrap()
}

async fn client(fairing: PoliPageFairing) -> Client {
    let rocket = rocket::build().attach(fairing).mount("/", routes![fail]);
    Client::tracked(rocket).await.unwrap()
}

async fn json_answer(r: LocalResponse<'_>) -> (Status, Value) {
    let status = r.status();
    let content_type = r.content_type().expect("a Content-Type");
    assert_eq!(
        (content_type.top().as_str(), content_type.sub().as_str()),
        ("application", "json"),
        "media type"
    );
    let body = r.into_string().await.unwrap();
    (status, serde_json::from_str(&body).unwrap())
}

#[rocket::async_test]
async fn on_by_default_with_the_fairing() {
    for fairing in [
        PoliPageFairing::with_client(sdk()),
        PoliPageFairing::new(poli_page::PoliPage::builder().api_key(KEY)),
    ] {
        let c = client(fairing).await;
        let (status, body) = json_answer(c.get("/fail/template-not-found").dispatch().await).await;
        assert_eq!(status, Status::NotFound);
        assert_eq!(body["code"], "TEMPLATE_NOT_FOUND");
    }
}

#[rocket::async_test]
async fn off_hands_the_status_to_rocket_catchers() {
    let c = client(PoliPageFairing::with_client(sdk()).error_handler(false)).await;
    let r = c.get("/fail/template-not-found").dispatch().await;
    assert_eq!(r.status(), Status::NotFound);
    // Rocket's default catcher, not the JSON payload.
    assert_eq!(r.content_type(), Some(ContentType::HTML));
    let body = r.into_string().await.unwrap();
    assert!(!body.contains("TEMPLATE_NOT_FOUND"), "{body}");

    let r = c.get("/fail/internal").dispatch().await;
    assert_eq!(r.status(), Status::InternalServerError);
    assert_eq!(r.content_type(), Some(ContentType::HTML));
}

#[rocket::async_test]
async fn off_then_on_again() {
    let c = client(
        PoliPageFairing::with_client(sdk())
            .error_handler(false)
            .error_handler(true),
    )
    .await;
    let (status, _) = json_answer(c.get("/fail/timeout").dispatch().await).await;
    assert_eq!(status, Status::GatewayTimeout);
}

#[rocket::async_test]
async fn api_error_body_is_the_payload() {
    let c = client(PoliPageFairing::with_client(sdk())).await;
    let (status, body) = json_answer(c.get("/fail/template-not-found").dispatch().await).await;
    assert_eq!(status, Status::NotFound);
    assert_eq!(
        body,
        json!({
            "code": "TEMPLATE_NOT_FOUND",
            "message": "Template 'does-not-exist' not found in project 'getting-started'.",
            "status": 404,
            "requestId": "req_404",
        })
    );
}

#[rocket::async_test]
async fn network_error_answers_503() {
    let c = client(PoliPageFairing::with_client(sdk())).await;
    let (status, body) = json_answer(c.get("/fail/network").dispatch().await).await;
    assert_eq!(status, Status::ServiceUnavailable);
    assert_eq!(
        body,
        json!({
            "code": "network_error",
            "message": "Network request failed",
            "status": 503,
            "requestId": null,
        })
    );
}

#[rocket::async_test]
async fn timeout_answers_504() {
    let c = client(PoliPageFairing::with_client(sdk())).await;
    let (status, body) = json_answer(c.get("/fail/timeout").dispatch().await).await;
    assert_eq!(status, Status::GatewayTimeout);
    assert_eq!(
        body,
        json!({
            "code": "timeout",
            "message": "Request timed out after 1500ms",
            "status": 504,
            "requestId": null,
        })
    );
}

#[rocket::async_test]
async fn sdk_error_without_status_answers_500_with_null_status() {
    let c = client(PoliPageFairing::with_client(sdk())).await;
    let (status, body) = json_answer(c.get("/fail/internal").dispatch().await).await;
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(
        body,
        json!({
            "code": "INTERNAL_ERROR",
            "message": "unexpected answer",
            "status": null,
            "requestId": null,
        })
    );
}

#[rocket::async_test]
async fn body_is_exactly_to_payload() {
    let c = client(PoliPageFairing::with_client(sdk())).await;
    for variant in ["template-not-found", "network", "timeout", "internal"] {
        let (_, body) = json_answer(c.get(format!("/fail/{variant}")).dispatch().await).await;
        let expected = serde_json::to_value(error(variant).to_payload()).unwrap();
        assert_eq!(body, expected, "{variant}");
    }
}
