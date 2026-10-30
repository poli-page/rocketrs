//! Helpers shared by the integration tests.

use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

/// `MakeWriter` that keeps everything a `tracing_subscriber::fmt` subscriber
/// writes, so a test can assert on the emitted events.
#[derive(Clone, Default)]
pub struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl CaptureWriter {
    /// A plain-text (no ANSI, no timestamp) subscriber writing into `self`.
    pub fn subscriber(&self) -> impl tracing::Subscriber + Send + Sync {
        tracing_subscriber::fmt()
            .with_writer(self.clone())
            .with_ansi(false)
            .without_time()
            .finish()
    }

    /// Everything written so far.
    pub fn output(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureWriterHandle;
    fn make_writer(&'a self) -> Self::Writer {
        CaptureWriterHandle(self.0.clone())
    }
}

pub struct CaptureWriterHandle(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for CaptureWriterHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
