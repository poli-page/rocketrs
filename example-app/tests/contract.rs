//! Conformance of the example app with the Poli Page API contract
//! (<https://github.com/poli-page/api-contract>), its `kind: "impl"` cases.
//!
//! CI copies the pinned `contract.json` with the `poli-page/api-contract`
//! action and sets `POLI_CONTRACT` to its path. Every `impl` case runs as its
//! own test, named by the case id:
//!
//! 1. A local HTTP/1.1 server on `127.0.0.1:0` answers the case's `responses`
//!    in order (`drop` closes the connection without an answer, `hang` never
//!    answers) and records each request. `{{mock}}` in the case becomes its
//!    origin.
//! 2. The example app's routes ([`poli_page_rocket_example_app::app`]) are
//!    mounted with a `PoliPageFairing` built the documented way,
//!    `PoliPageFairing::new(builder)`, from an SDK builder with a test key and
//!    the server as `base_url`. The SDK's retry settings stay at their
//!    defaults, and the fairing's error handler stays on, as in the app.
//! 3. The case's `request` goes through Rocket's local client, and the app's
//!    answer (`status`, `location`, `contentType`, `json`) and the recorded
//!    SDK requests (method, path, `Authorization`, `Idempotency-Key`) are
//!    compared with the case's `expect`. The answer is compared as the app
//!    sends it: no `policy.normalizations` apply to it.
//!
//! `kind: "sdk"` cases belong to the SDK and are not run here.
//!
//! `tests/contract-skip.json` maps the case ids allowed to fail to the reason.
//! A listed case that passes, or a listed id that is not an `impl` case of
//! the contract, fails the run.
//!
//! Locally: `POLI_CONTRACT=/path/to/contract.json cargo test --test contract`
//! from `example-app/`. Without it the suite is skipped, except when `CI` is
//! set, where a missing contract is an error.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use libtest_mimic::{Arguments, Failed, Trial};
use poli_page_rocket::PoliPageFairing;
use rocket::config::LogLevel;
use rocket::http::Method;
use rocket::local::asynchronous::Client;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// The contract layout this file reads.
const KNOWN_FORMAT_VERSION: u64 = 1;

/// The app's API key: an environment key, `pp_sandbox_` + 32 hex.
const TEST_API_KEY: &str = "pp_sandbox_0123456789abcdef0123456789abcdef";

/// No case comes near this; it only turns a stuck case into a failure.
const CASE_TIMEOUT: Duration = Duration::from_mins(1);

const SKIP_LIST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/contract-skip.json");

// ---------------------------------------------------------------------------
// Contract format 1 (README of poli-page/api-contract)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contract {
    policy: Policy,
    cases: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Policy {
    /// The keys of an error answer, in the contract's order.
    impl_error_body: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // id, kind and title are read from the raw value.
struct ImplCase {
    id: String,
    kind: String,
    title: String,
    request: AppRequest,
    responses: Vec<Value>,
    expect: Expect,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AppRequest {
    method: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Expect {
    status: u16,
    location: Option<String>,
    content_type: Option<String>,
    json: Option<Value>,
    requests: Option<Vec<ExpectedRequest>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ExpectedRequest {
    method: String,
    path: String,
    auth: Auth,
    idempotency_key: IdempotencyKey,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
enum Auth {
    Bearer,
    None,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
enum IdempotencyKey {
    Stable,
    Absent,
}

fn load_contract(path: &str) -> Result<Contract, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let raw: Value = serde_json::from_str(&text).map_err(|e| format!("{path} is not JSON: {e}"))?;
    let version = raw
        .get("formatVersion")
        .ok_or_else(|| format!("{path} is not a contract.json: it has no formatVersion"))?;
    if version.as_u64() != Some(KNOWN_FORMAT_VERSION) {
        return Err(format!(
            "contract.json formatVersion {version} is not supported (this test reads format \
             {KNOWN_FORMAT_VERSION}): update example-app/tests/contract.rs"
        ));
    }
    serde_json::from_value(raw).map_err(|e| format!("{path}: unexpected contract shape: {e}"))
}

fn load_skip_list() -> Result<BTreeMap<String, String>, String> {
    let text =
        std::fs::read_to_string(SKIP_LIST).map_err(|e| format!("cannot read {SKIP_LIST}: {e}"))?;
    let raw: Map<String, Value> = serde_json::from_str(&text).map_err(|e| {
        format!("tests/contract-skip.json must be an object of case id to reason: {e}")
    })?;
    let mut skip = BTreeMap::new();
    for (id, reason) in raw {
        match reason.as_str() {
            Some(reason) if !reason.trim().is_empty() => {
                skip.insert(id, reason.to_owned());
            }
            _ => return Err(format!("tests/contract-skip.json: \"{id}\" needs a reason")),
        }
    }
    Ok(skip)
}

// ---------------------------------------------------------------------------
// Placeholders: {{mock}}, {{httpDate+N}}
// ---------------------------------------------------------------------------

fn substitute_str(text: &str, mock: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start + 2..].find("}}") else {
            break;
        };
        let name = &rest[start + 2..start + 2 + len];
        out.push_str(&rest[..start]);
        if name == "mock" {
            out.push_str(mock);
        } else if let Some(seconds) = name.strip_prefix("httpDate+") {
            let seconds: u64 = seconds
                .parse()
                .map_err(|e| format!("{{{{{name}}}}}: {e}"))?;
            let at = SystemTime::now() + Duration::from_secs(seconds);
            out.push_str(&httpdate::fmt_http_date(at));
        } else {
            return Err(format!("unknown contract placeholder {{{{{name}}}}}"));
        }
        rest = &rest[start + 2 + len + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn substitute(value: &Value, mock: &str) -> Result<Value, String> {
    Ok(match value {
        Value::String(s) => Value::String(substitute_str(s, mock)?),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| substitute(item, mock))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| Ok((k.clone(), substitute(v, mock)?)))
                .collect::<Result<_, String>>()?,
        ),
        other => other.clone(),
    })
}

// ---------------------------------------------------------------------------
// Mock server
// ---------------------------------------------------------------------------

enum Step {
    Answer {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    Drop,
    Hang,
}

fn parse_step(raw: &Value) -> Result<Step, String> {
    let obj = raw
        .as_object()
        .ok_or_else(|| format!("response step is not an object: {raw}"))?;
    if let Some(fault) = obj.get("fault") {
        return match fault.as_str() {
            Some("drop") => Ok(Step::Drop),
            Some("hang") => Ok(Step::Hang),
            _ => Err(format!("unknown fault {fault}")),
        };
    }
    let status = obj
        .get("status")
        .and_then(Value::as_u64)
        .and_then(|s| u16::try_from(s).ok())
        .ok_or_else(|| format!("response step has no status: {raw}"))?;
    let mut headers = Vec::new();
    if let Some(h) = obj.get("headers") {
        let h = h.as_object().ok_or("response headers must be an object")?;
        for (name, value) in h {
            let value = value
                .as_str()
                .ok_or_else(|| format!("header {name} must be a string"))?;
            headers.push((name.clone(), value.to_owned()));
        }
    }
    let has = |headers: &[(String, String)], name: &str| {
        headers.iter().any(|(n, _)| n.eq_ignore_ascii_case(name))
    };
    let bodies = ["json", "text", "base64"]
        .iter()
        .filter(|k| obj.contains_key(**k))
        .count();
    if bodies > 1 {
        return Err(format!("response step has more than one body: {raw}"));
    }
    let body = if let Some(v) = obj.get("json") {
        if !has(&headers, "content-type") {
            headers.push(("content-type".into(), "application/json".into()));
        }
        serde_json::to_vec(v).map_err(|e| e.to_string())?
    } else if let Some(v) = obj.get("text") {
        v.as_str()
            .ok_or("text must be a string")?
            .as_bytes()
            .to_vec()
    } else if let Some(v) = obj.get("base64") {
        base64::engine::general_purpose::STANDARD
            .decode(v.as_str().ok_or("base64 must be a string")?)
            .map_err(|e| format!("invalid base64 body: {e}"))?
    } else {
        Vec::new()
    };
    Ok(Step::Answer {
        status,
        headers,
        body,
    })
}

/// A request the mock server received.
struct Recorded {
    method: String,
    /// The request target: path and query.
    path: String,
    authorization: Option<String>,
    idempotency_key: Option<String>,
}

type Requests = Arc<Mutex<Vec<Recorded>>>;

struct MockServer {
    requests: Requests,
    accept: tokio::task::JoinHandle<()>,
}

impl MockServer {
    fn start(listener: TcpListener, steps: Vec<Step>) -> Self {
        let requests: Requests = Arc::default();
        let steps = Arc::new(steps);
        let accept = {
            let requests = Arc::clone(&requests);
            tokio::spawn(async move {
                // Dropping the set (when this task is aborted) aborts every
                // connection, hung ones included.
                let mut connections = tokio::task::JoinSet::new();
                while let Ok((stream, _)) = listener.accept().await {
                    connections.spawn(serve_connection(
                        stream,
                        Arc::clone(&steps),
                        Arc::clone(&requests),
                    ));
                }
            })
        };
        Self { requests, accept }
    }

    fn close(self) -> Vec<Recorded> {
        self.accept.abort();
        let mut requests = self.requests.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *requests)
    }
}

async fn serve_connection(mut stream: TcpStream, steps: Arc<Vec<Step>>, requests: Requests) {
    let mut buf = Vec::new();
    loop {
        let Some(request) = read_request(&mut stream, &mut buf).await else {
            return;
        };
        let index = {
            let mut recorded = requests.lock().unwrap_or_else(PoisonError::into_inner);
            recorded.push(request);
            recorded.len() - 1
        };
        let written = match steps.get(index) {
            // More requests than the case has answers: make it visible in the diff.
            None => {
                let body = json!({ "error": "MOCK_EXHAUSTED" })
                    .to_string()
                    .into_bytes();
                let headers = [("content-type".to_owned(), "application/json".to_owned())];
                write_answer(&mut stream, 599, &headers, &body).await
            }
            // Dropping the stream closes the connection without an answer.
            Some(Step::Drop) => return,
            Some(Step::Hang) => std::future::pending().await,
            Some(Step::Answer {
                status,
                headers,
                body,
            }) => write_answer(&mut stream, *status, headers, body).await,
        };
        if written.is_err() {
            return;
        }
    }
}

/// Reads one HTTP/1.1 request; `None` once the client closes the connection.
async fn read_request(stream: &mut TcpStream, buf: &mut Vec<u8>) -> Option<Recorded> {
    let head_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        let mut chunk = [0u8; 8192];
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    buf.drain(..head_end + 4);
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next()?.split(' ');
    let method = request_line.next()?.to_owned();
    let path = request_line.next()?.to_owned();
    let mut authorization = None;
    let mut idempotency_key = None;
    let mut length = 0usize;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        let value = value.trim().to_owned();
        match name.trim().to_ascii_lowercase().as_str() {
            "authorization" => authorization = Some(value),
            "idempotency-key" => idempotency_key = Some(value),
            // The SDK sends a Content-Length with every body it builds.
            "content-length" => length = value.parse().ok()?,
            _ => {}
        }
    }
    while buf.len() < length {
        let mut chunk = [0u8; 8192];
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    buf.drain(..length);
    Some(Recorded {
        method,
        path,
        authorization,
        idempotency_key,
    })
}

async fn write_answer(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {status} Mock\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!("content-length: {}\r\n\r\n", body.len()));
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

// ---------------------------------------------------------------------------
// One case
// ---------------------------------------------------------------------------

/// The app's answer to the case's request.
struct Answer {
    status: u16,
    location: Option<String>,
    content_type: Option<String>,
    body: Vec<u8>,
}

/// The media type of a `Content-Type` value, without its parameters.
fn media_type(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn is_lowercase_uuid_v4(text: &str) -> bool {
    let bytes = text.as_bytes();
    let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, &b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => hex(b),
        })
        && bytes[14] == b'4'
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
}

async fn call_app(origin: &str, request: &AppRequest) -> Result<Answer, String> {
    let sdk = poli_page::PoliPage::builder()
        .api_key(TEST_API_KEY)
        .base_url(origin);
    let rocket = poli_page_rocket_example_app::app(PoliPageFairing::new(sdk))
        .configure(rocket::Config::figment().merge(("log_level", LogLevel::Off)));
    let client = Client::tracked(rocket)
        .await
        .map_err(|e| format!("the example app did not ignite: {e}"))?;
    let method = Method::from_str(&request.method)
        .map_err(|()| format!("unknown request method {}", request.method))?;
    let response = client.req(method, request.path.clone()).dispatch().await;
    let header = |name: &str| response.headers().get_one(name).map(str::to_owned);
    let (location, content_type) = (header("location"), header("content-type"));
    let status = response.status().code;
    let body = response.into_bytes().await.unwrap_or_default();
    Ok(Answer {
        status,
        location,
        content_type,
        body,
    })
}

fn compare(
    case: &ImplCase,
    answer: &Answer,
    requests: &[Recorded],
    error_body: &[String],
) -> Vec<String> {
    let expect = &case.expect;
    let mut problems = Vec::new();
    if answer.status != expect.status {
        problems.push(format!(
            "status: expected {}, got {}",
            expect.status, answer.status
        ));
    }
    if let Some(location) = &expect.location {
        if answer.location.as_ref() != Some(location) {
            problems.push(format!(
                "location: expected {location:?}, got {:?}",
                answer.location
            ));
        }
    }
    if let Some(content_type) = &expect.content_type {
        let got = answer.content_type.as_deref().map(media_type);
        if got.as_deref() != Some(media_type(content_type).as_str()) {
            problems.push(format!(
                "contentType: expected {content_type:?}, got {:?}",
                answer.content_type
            ));
        }
    }
    if let Some(expected) = &expect.json {
        match serde_json::from_slice::<Value>(&answer.body) {
            Err(e) => problems.push(format!(
                "json: the body is not JSON ({e}): {:?}",
                String::from_utf8_lossy(&answer.body)
            )),
            Ok(got) => {
                if &got != expected {
                    problems.push(format!("json: expected {expected}, got {got}"));
                }
                // An error answer has exactly the keys of policy.implErrorBody.
                if expect.status >= 400 {
                    let keys: BTreeSet<&str> = got
                        .as_object()
                        .map(|o| o.keys().map(String::as_str).collect())
                        .unwrap_or_default();
                    let want: BTreeSet<&str> = error_body.iter().map(String::as_str).collect();
                    if keys != want {
                        problems.push(format!(
                            "json: an error answer has the keys {want:?} \
                             (policy.implErrorBody), got {keys:?}"
                        ));
                    }
                }
            }
        }
    }
    if let Some(expected) = &expect.requests {
        problems.extend(compare_requests(expected, requests));
    }
    problems
}

fn compare_requests(expected: &[ExpectedRequest], requests: &[Recorded]) -> Vec<String> {
    let mut problems = Vec::new();
    if requests.len() != expected.len() {
        let got: Vec<String> = requests
            .iter()
            .map(|r| format!("{} {}", r.method, r.path))
            .collect();
        problems.push(format!(
            "requests: expected {}, got {}: {got:?}",
            expected.len(),
            requests.len()
        ));
    }
    let bearer = format!("Bearer {TEST_API_KEY}");
    let mut stable_keys = BTreeSet::new();
    for (i, (want, got)) in expected.iter().zip(requests).enumerate() {
        if got.method != want.method {
            problems.push(format!(
                "requests[{i}].method: expected {}, got {}",
                want.method, got.method
            ));
        }
        if got.path != want.path {
            problems.push(format!(
                "requests[{i}].path: expected {}, got {}",
                want.path, got.path
            ));
        }
        let auth_ok = match want.auth {
            Auth::Bearer => got.authorization.as_deref() == Some(bearer.as_str()),
            Auth::None => got.authorization.is_none(),
        };
        if !auth_ok {
            // Never print the header: it carries the key.
            let seen = match &got.authorization {
                None => "no Authorization header",
                Some(v) if v == &bearer => "Authorization: Bearer <the test key>",
                Some(_) => "another Authorization header",
            };
            problems.push(format!(
                "requests[{i}].auth: expected {:?}, got {seen}",
                want.auth
            ));
        }
        match (want.idempotency_key, &got.idempotency_key) {
            (IdempotencyKey::Absent, None) => {}
            (IdempotencyKey::Absent, Some(key)) => problems.push(format!(
                "requests[{i}].idempotencyKey: expected absent, got {key:?}"
            )),
            (IdempotencyKey::Stable, None) => problems.push(format!(
                "requests[{i}].idempotencyKey: expected a key, got none"
            )),
            (IdempotencyKey::Stable, Some(key)) => {
                if !is_lowercase_uuid_v4(key) {
                    problems.push(format!(
                        "requests[{i}].idempotencyKey: expected a lowercase UUID v4, got {key:?}"
                    ));
                }
                stable_keys.insert(key.clone());
            }
        }
    }
    // The case's request makes one SDK call: its attempts share one key.
    if stable_keys.len() > 1 {
        problems.push(format!(
            "idempotencyKey: the attempts of one call carry different keys {stable_keys:?}"
        ));
    }
    problems
}

async fn run_case(raw: &Value, error_body: &[String]) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("cannot bind the mock server: {e}"))?;
    let origin = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let case: ImplCase = serde_json::from_value(substitute(raw, &origin)?)
        .map_err(|e| format!("cannot read the case: {e}"))?;
    let steps = case
        .responses
        .iter()
        .map(parse_step)
        .collect::<Result<Vec<_>, _>>()?;
    let server = MockServer::start(listener, steps);
    let answer = call_app(&origin, &case.request).await;
    let requests = server.close();
    let problems = compare(&case, &answer?, &requests, error_body);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

fn run_case_blocking(raw: &Value, error_body: &[String]) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;
    runtime.block_on(async {
        tokio::time::timeout(CASE_TIMEOUT, run_case(raw, error_body))
            .await
            .unwrap_or_else(|_| Err(format!("the case did not finish in {CASE_TIMEOUT:?}")))
    })
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

fn failing(name: &str, message: String) -> Trial {
    Trial::test(name, move || Err(Failed::from(message)))
}

fn trials() -> Vec<Trial> {
    let Some(path) = std::env::var("POLI_CONTRACT")
        .ok()
        .filter(|p| !p.is_empty())
    else {
        if std::env::var("CI").is_ok_and(|v| !v.is_empty()) {
            return vec![failing(
                "api_contract::poli_contract_is_set",
                "POLI_CONTRACT is not set: in CI the \"API contract\" step must provide \
                 contract.json (poli-page/api-contract action, output `path`)"
                    .into(),
            )];
        }
        return vec![Trial::test(
            "api_contract (skipped: set POLI_CONTRACT to a contract.json path)",
            || Ok(()),
        )
        .with_ignored_flag(true)];
    };
    let contract = match load_contract(&path) {
        Ok(c) => c,
        Err(e) => return vec![failing("api_contract::load", e)],
    };
    let skip = match load_skip_list() {
        Ok(s) => s,
        Err(e) => return vec![failing("api_contract::skip_list", e)],
    };
    let error_body = Arc::new(contract.policy.impl_error_body);

    let mut trials = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    let mut unknown_kinds = Vec::new();
    for raw in contract.cases {
        let kind = raw["kind"].as_str().unwrap_or("<none>").to_owned();
        let id = raw["id"].as_str().unwrap_or("<no id>").to_owned();
        match kind.as_str() {
            "impl" => {}
            "sdk" => continue,
            other => {
                unknown_kinds.push(format!("{id}: {other}"));
                continue;
            }
        }
        ids.push(id.clone());
        // Any placeholder anywhere in the case must be one the contract defines.
        if let Err(e) = substitute(&raw, "http://127.0.0.1:1") {
            trials.push(failing(&id, e));
            continue;
        }
        let error_body = Arc::clone(&error_body);
        match skip.get(&id).cloned() {
            None => trials.push(Trial::test(&id, move || {
                run_case_blocking(&raw, &error_body).map_err(Failed::from)
            })),
            Some(reason) => {
                let id2 = id.clone();
                trials.push(
                    Trial::test(&id, move || match run_case_blocking(&raw, &error_body) {
                        Ok(()) => Err(Failed::from(format!(
                            "{id2} passes now: remove it from tests/contract-skip.json"
                        ))),
                        Err(_) => Ok(()),
                    })
                    .with_kind(format!("expected to fail: {reason}")),
                );
            }
        }
    }

    let impl_count = ids.len();
    trials.push(Trial::test(
        "api_contract::has_impl_cases_and_known_kinds",
        move || {
            if impl_count == 0 {
                return Err("the contract has no kind \"impl\" case".into());
            }
            if !unknown_kinds.is_empty() {
                return Err(format!(
                    "case kinds other than \"sdk\" and \"impl\": {unknown_kinds:?}"
                )
                .into());
            }
            Ok(())
        },
    ));
    trials.push(Trial::test(
        "api_contract::skip_list_ids_exist",
        move || {
            let stale: Vec<&String> = skip.keys().filter(|id| !ids.contains(id)).collect();
            if stale.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "ids that are not impl cases of the contract, remove them from \
                 tests/contract-skip.json: {stale:?}"
                )
                .into())
            }
        },
    ));
    trials
}

fn main() {
    let args = Arguments::from_args();
    libtest_mimic::run(&args, trials()).exit();
}
