//! Verify the status map and body shape of `Responder for PoliPageError`.

use std::time::Duration;

use poli_page::Error;
use poli_page_rocket::errors::PoliPageError;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes};

fn make_error(variant: &str) -> Error {
    match variant {
        "bad-request" => Error::BadRequest {
            status: 400,
            code: "INVALID_VERSION_FORMAT".into(),
            message: "bad".into(),
            request_id: Some("req_1".into()),
        },
        "auth" => Error::Auth {
            status: 401,
            code: "INVALID_API_KEY".into(),
            message: "x".into(),
            request_id: None,
        },
        "perm" => Error::PermissionDenied {
            status: 403,
            code: "FORBIDDEN".into(),
            message: "x".into(),
            request_id: None,
        },
        "not-found" => Error::NotFound {
            status: 404,
            code: "NOT_FOUND".into(),
            message: "x".into(),
            request_id: None,
        },
        "gone" => Error::Gone {
            status: 410,
            code: "GONE".into(),
            message: "x".into(),
            request_id: None,
        },
        "rate" => Error::RateLimited {
            status: 429,
            code: "QUOTA_EXCEEDED".into(),
            message: "x".into(),
            request_id: None,
        },
        "api503" => Error::Api {
            status: 503,
            code: "INTERNAL_ERROR".into(),
            message: "x".into(),
            request_id: None,
        },
        "conn" => Error::Connection {
            message: "dns".into(),
            source: Box::<dyn std::error::Error + Send + Sync>::from("inner"),
        },
        "timeout" => Error::Timeout {
            timeout: Duration::from_secs(60),
        },
        "aborted" => Error::Aborted,
        "invalid-options" => Error::InvalidOptions {
            message: "x".into(),
        },
        "download" => Error::Download {
            message: "s3".into(),
            status: Some(403),
            source: None,
        },
        "internal" => Error::Internal {
            message: "x".into(),
            status: None,
        },
        other => panic!("unknown variant: {other}"),
    }
}

#[get("/<variant>")]
fn err_route(variant: &str) -> Result<&'static str, PoliPageError> {
    Err(make_error(variant).into())
}

async fn client() -> Client {
    let r = rocket::build().mount("/", routes![err_route]);
    Client::tracked(r).await.unwrap()
}

async fn body(r: rocket::local::asynchronous::LocalResponse<'_>) -> serde_json::Value {
    let s = r.into_string().await.unwrap();
    serde_json::from_str(&s).unwrap()
}

#[rocket::async_test]
async fn bad_request_maps_to_status_400_with_code_and_request_id() {
    let c = client().await;
    let r = c.get("/bad-request").dispatch().await;
    assert_eq!(r.status(), Status::BadRequest);
    assert_eq!(
        r.content_type().unwrap().to_string(),
        "application/json; charset=utf-8"
    );
    assert_eq!(
        r.headers().get_one("cache-control"),
        Some("private, no-store")
    );
    let j = body(r).await;
    assert_eq!(j["code"], "INVALID_VERSION_FORMAT");
    // Message is the bare reason from the variant, not the Display prefix.
    assert_eq!(j["message"], "bad");
    assert_eq!(j["status"], 400);
    assert_eq!(j["requestId"], "req_1");
}

#[rocket::async_test]
async fn auth_maps_to_401() {
    let c = client().await;
    assert_eq!(
        c.get("/auth").dispatch().await.status(),
        Status::Unauthorized
    );
}

#[rocket::async_test]
async fn permission_denied_maps_to_403() {
    let c = client().await;
    assert_eq!(c.get("/perm").dispatch().await.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn not_found_maps_to_404() {
    let c = client().await;
    assert_eq!(
        c.get("/not-found").dispatch().await.status(),
        Status::NotFound
    );
}

#[rocket::async_test]
async fn gone_maps_to_410() {
    let c = client().await;
    assert_eq!(c.get("/gone").dispatch().await.status(), Status::Gone);
}

#[rocket::async_test]
async fn rate_limited_maps_to_429() {
    let c = client().await;
    assert_eq!(
        c.get("/rate").dispatch().await.status(),
        Status::TooManyRequests
    );
}

#[rocket::async_test]
async fn api_503_passes_through() {
    let c = client().await;
    assert_eq!(
        c.get("/api503").dispatch().await.status(),
        Status::ServiceUnavailable
    );
}

#[rocket::async_test]
async fn connection_error_maps_to_503() {
    let c = client().await;
    assert_eq!(
        c.get("/conn").dispatch().await.status(),
        Status::ServiceUnavailable
    );
}

#[rocket::async_test]
async fn timeout_maps_to_504() {
    let c = client().await;
    assert_eq!(
        c.get("/timeout").dispatch().await.status(),
        Status::GatewayTimeout
    );
}

#[rocket::async_test]
async fn aborted_maps_to_500() {
    // Aborted has no upstream status; payload.status is None → 500 fallback.
    let c = client().await;
    assert_eq!(
        c.get("/aborted").dispatch().await.status(),
        Status::InternalServerError
    );
}

#[rocket::async_test]
async fn invalid_options_maps_to_500() {
    let c = client().await;
    assert_eq!(
        c.get("/invalid-options").dispatch().await.status(),
        Status::InternalServerError,
    );
}

#[rocket::async_test]
async fn download_propagates_storage_status() {
    // The Download fixture uses status=Some(403); the new payload-driven
    // design propagates the storage status verbatim (was hard-mapped to 502
    // pre-rollout). Downloads with status=None fall back to 500 — impls
    // that want 502 for that case override post-payload.
    let c = client().await;
    assert_eq!(
        c.get("/download").dispatch().await.status(),
        Status::Forbidden
    );
}

#[rocket::async_test]
async fn internal_maps_to_500() {
    let c = client().await;
    assert_eq!(
        c.get("/internal").dispatch().await.status(),
        Status::InternalServerError
    );
}

#[rocket::async_test]
async fn null_request_id_for_reserved_variants() {
    let c = client().await;
    let r = c.get("/conn").dispatch().await;
    let j = body(r).await;
    assert!(j["requestId"].is_null());
}
