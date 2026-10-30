//! Verify `on_retry` / `on_error` emit events under target `poli_page_rocket`.

use std::time::Duration;

use poli_page::{Error, RetryEvent};
use tracing::subscriber::with_default;

mod common;
use common::CaptureWriter;

#[test]
fn on_retry_emits_event_with_attempt_and_delay() {
    let writer = CaptureWriter::default();
    with_default(writer.subscriber(), || {
        let event = RetryEvent {
            attempt: 3,
            delay: Duration::from_millis(750),
            reason: Error::Api {
                status: 503,
                code: "INTERNAL_ERROR".into(),
                message: "boom".into(),
                request_id: Some("req_99".into()),
                source: None,
            },
        };
        poli_page_rocket::__internal_tracing_bridge::on_retry(&event);
    });
    let buf = writer.output();
    assert!(buf.contains("poli_page_rocket"), "missing target: {buf}");
    assert!(buf.contains("attempt=3"), "missing attempt: {buf}");
    assert!(buf.contains("delay_ms=750"), "missing delay: {buf}");
    assert!(buf.contains("INTERNAL_ERROR"), "missing code: {buf}");
    assert!(buf.contains("503"), "missing status: {buf}");
}

#[test]
fn on_error_emits_terminal_event() {
    let writer = CaptureWriter::default();
    with_default(writer.subscriber(), || {
        let err = Error::Timeout {
            timeout: Duration::from_secs(60),
        };
        poli_page_rocket::__internal_tracing_bridge::on_error(&err);
    });
    let buf = writer.output();
    assert!(buf.contains("poli_page_rocket"), "missing target: {buf}");
    assert!(buf.contains("timeout"), "missing code: {buf}");
}
