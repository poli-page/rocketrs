//! The demo routes against a local fake of the Poli Page API.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use poli_page_rocket::PoliPageFairing;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};

/// A request the fake API received.
struct Seen {
    method: String,
    path: String,
    body: Value,
}

/// Starts a fake API on a free local port that records what it receives. It
/// serves `GET /v1/documents/doc_1/preview` and answers everything else with
/// the API's `404 TEMPLATE_NOT_FOUND` body.
fn fake_api() -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            let mut parts = request_line.split_whitespace();
            let method = parts.next().unwrap_or_default().to_owned();
            let path = parts.next().unwrap_or_default().to_owned();
            let mut content_length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = v.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body).unwrap();
            let (status, content_type, answer) = if path == "/v1/documents/doc_1/preview" {
                ("200 OK", "text/html", "<h1>Stored</h1>".to_owned())
            } else {
                let answer = json!({
                    "error": "TEMPLATE_NOT_FOUND",
                    "detail": "Template does-not-exist not found in project getting-started",
                    "requestId": "req_fake_404",
                });
                ("404 Not Found", "application/json", answer.to_string())
            };
            log.lock().unwrap().push(Seen {
                method,
                path,
                body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            });
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n\
                 X-Request-Id: req_fake_404\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{answer}",
                answer.len(),
            )
            .unwrap();
        }
    });
    (base_url, seen)
}

async fn client(base_url: &str) -> Client {
    let sdk = poli_page::PoliPage::builder()
        .api_key("pp_sandbox_0123456789abcdef0123456789abcdef")
        .base_url(base_url)
        .max_retries(0)
        .build()
        .unwrap();
    Client::tracked(crate::app(PoliPageFairing::with_client(sdk)))
        .await
        .unwrap()
}

#[rocket::async_test]
async fn not_found_demo_answers_404_template_not_found() {
    let (base_url, seen) = fake_api();
    let c = client(&base_url).await;

    let r = c.get("/errors/not-found").dispatch().await;
    assert_eq!(r.status(), Status::NotFound);
    assert_eq!(
        r.content_type().unwrap().to_string(),
        "application/json; charset=utf-8"
    );
    let body: Value = serde_json::from_str(&r.into_string().await.unwrap()).unwrap();
    assert_eq!(body["code"], "TEMPLATE_NOT_FOUND");
    assert_eq!(body["status"], 404);
    assert_eq!(
        body["message"],
        "Template does-not-exist not found in project getting-started"
    );
    assert_eq!(body["requestId"], "req_fake_404");

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "POST");
    assert_eq!(seen[0].path, "/v1/render");
    assert_eq!(
        seen[0].body,
        json!({ "project": "getting-started", "template": "does-not-exist", "data": {} })
    );
}

#[rocket::async_test]
async fn demo_renders_send_no_version() {
    let (base_url, seen) = fake_api();
    let c = client(&base_url).await;

    c.get("/render/pdf").dispatch().await;
    c.get("/render/stream").dispatch().await;
    c.post("/documents").dispatch().await;

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 3);
    for s in seen.iter() {
        assert_eq!(s.path, "/v1/render");
        assert_eq!(s.body["project"], "getting-started");
        assert_eq!(s.body["template"], "welcome");
        assert_eq!(s.body["data"], json!({ "name": "SDK Demo" }));
        assert!(s.body.get("version").is_none(), "version sent: {}", s.body);
    }
}

#[rocket::async_test]
async fn render_preview_and_bad_version_routes_are_gone() {
    let gone = ["/render/preview", "/errors/bad-version"];
    let rocket = crate::app(PoliPageFairing::from_env());
    for route in rocket.routes() {
        assert!(
            !gone.contains(&route.uri.path().to_string().as_str()),
            "{route} is still mounted"
        );
    }

    let (base_url, seen) = fake_api();
    let c = client(&base_url).await;
    for path in gone {
        for r in [c.get(path).dispatch().await, c.post(path).dispatch().await] {
            // Rocket's own 404 page, not a 404 relayed from the API.
            assert_eq!(r.status(), Status::NotFound, "{path}");
            assert_eq!(r.content_type(), Some(ContentType::HTML), "{path}");
        }
    }
    assert!(seen.lock().unwrap().is_empty(), "the API was called");
}

#[rocket::async_test]
async fn stored_document_preview_still_works() {
    let (base_url, seen) = fake_api();
    let c = client(&base_url).await;

    let r = c.get("/documents/doc_1/preview").dispatch().await;
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(
        r.headers().get_one("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(r.into_string().await.unwrap(), "<h1>Stored</h1>");
    assert_eq!(seen.lock().unwrap()[0].method, "GET");
}

/// Starts a fake API whose render succeeds and whose presigned storage URL
/// answers `storage_status`.
fn fake_api_with_failing_storage(storage_status: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let origin = base_url.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            let path = request_line
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_owned();
            let mut content_length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = v.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body).unwrap();
            let (status, answer) = if path == "/v1/render" {
                let descriptor = json!({
                    "documentId": "doc_1",
                    "organizationId": "org_1",
                    "projectId": "p_1",
                    "projectSlug": "getting-started",
                    "templateId": "t_1",
                    "templateSlug": "welcome",
                    "environment": "sandbox",
                    "apiKeyId": "k_1",
                    "format": "A4",
                    "orientation": "portrait",
                    "locale": "en-US",
                    "pageCount": 1,
                    "sizeBytes": 10,
                    "metadata": {},
                    "presignedPdfUrl": format!("{origin}/storage/doc_1.pdf"),
                    "expiresAt": "2026-09-29T10:00:22.000Z",
                    "createdAt": "2026-09-29T09:45:22.000Z",
                    "diagnostics": [],
                });
                ("200 OK", descriptor.to_string())
            } else {
                (storage_status, "<Error>AccessDenied</Error>".to_owned())
            };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len(),
            )
            .unwrap();
        }
    });
    base_url
}

/// A download that fails before the PDF stream exists is answered by the
/// error handler: JSON, none of the PDF headers.
#[rocket::async_test]
async fn stream_download_failure_answers_json_without_pdf_headers() {
    let base_url = fake_api_with_failing_storage("403 Forbidden");
    let c = client(&base_url).await;

    let r = c.get("/render/stream").dispatch().await;
    assert_eq!(r.status(), Status::Forbidden);
    assert_eq!(
        r.headers().get_one("content-type"),
        Some("application/json; charset=utf-8")
    );
    assert_eq!(r.headers().get_one("content-disposition"), None);
    let body: Value = serde_json::from_str(&r.into_string().await.unwrap()).unwrap();
    assert_eq!(
        body,
        json!({
            "code": "DOWNLOAD_FAILED",
            "message": "Failed to download PDF: HTTP 403",
            "status": 403,
            "requestId": null,
        })
    );
}

/// The stream route makes the SDK call before it builds the PDF responder,
/// so a render error is answered as JSON, not after a committed 200 PDF.
#[rocket::async_test]
async fn stream_render_error_answers_json_before_any_pdf_header() {
    let (base_url, _seen) = fake_api();
    let c = client(&base_url).await;

    let r = c.get("/render/stream").dispatch().await;
    assert_eq!(r.status(), Status::NotFound);
    assert_eq!(
        r.headers().get_one("content-type"),
        Some("application/json; charset=utf-8")
    );
    assert_eq!(r.headers().get_one("content-disposition"), None);
    let body: Value = serde_json::from_str(&r.into_string().await.unwrap()).unwrap();
    assert_eq!(body["code"], "TEMPLATE_NOT_FOUND");
    assert_eq!(body["status"], 404);
}
