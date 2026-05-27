//! Verify `on_retry` / `on_error` emit events under target `poli_page_rocket`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use poli_page::{Error, RetryEvent};
use tracing::subscriber::with_default;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureWriterHandle;
    fn make_writer(&'a self) -> Self::Writer {
        CaptureWriterHandle(self.0.clone())
    }
}

struct CaptureWriterHandle(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for CaptureWriterHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn on_retry_emits_event_with_attempt_and_delay() {
    let writer = CaptureWriter::default();
    let subscriber = fmt()
        .with_writer(writer.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    with_default(subscriber, || {
        let event = RetryEvent {
            attempt: 3,
            delay: Duration::from_millis(750),
            reason: Error::Api {
                status: 503,
                code: "INTERNAL_ERROR".into(),
                message: "boom".into(),
                request_id: Some("req_99".into()),
            },
        };
        poli_page_rocket::__internal_tracing_bridge::on_retry(&event);
    });
    let buf = String::from_utf8(writer.0.lock().unwrap().clone()).unwrap();
    assert!(buf.contains("poli_page_rocket"), "missing target: {buf}");
    assert!(buf.contains("attempt=3"), "missing attempt: {buf}");
    assert!(buf.contains("delay_ms=750"), "missing delay: {buf}");
    assert!(buf.contains("INTERNAL_ERROR"), "missing code: {buf}");
    assert!(buf.contains("503"), "missing status: {buf}");
}

#[test]
fn on_error_emits_terminal_event() {
    let writer = CaptureWriter::default();
    let subscriber = fmt()
        .with_writer(writer.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    with_default(subscriber, || {
        let err = Error::Timeout {
            timeout: Duration::from_secs(60),
        };
        poli_page_rocket::__internal_tracing_bridge::on_error(&err);
    });
    let buf = String::from_utf8(writer.0.lock().unwrap().clone()).unwrap();
    assert!(buf.contains("poli_page_rocket"), "missing target: {buf}");
    assert!(buf.contains("timeout"), "missing code: {buf}");
}
