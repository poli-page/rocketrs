//! PDF response — `Responder<'r, 'static>` over `Bytes` or `PdfByteStream`.

use std::io::Cursor;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream as _;
use rocket::http::{ContentType, Header};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use crate::headers::content_disposition;

/// PDF body — either pre-buffered `Bytes` or a streaming view from the SDK.
pub enum PdfBody {
    /// Fully-buffered bytes. `Content-Length` is set on the response.
    Bytes(Bytes),
    /// Chunked stream from `client.render.pdf_stream(...)`.
    Stream(poli_page::client::PdfByteStream),
}

/// `Responder` returning an `application/pdf` body with the conventional
/// headers (`Content-Disposition` per RFC 5987, `Cache-Control: private,
/// no-store`, `X-Content-Type-Options: nosniff`).
#[must_use]
pub struct PdfResponse {
    body: PdfBody,
    filename: Option<String>,
    inline: bool,
    cache_control: Option<String>,
}

impl PdfResponse {
    /// Build a `PdfResponse` from buffered bytes.
    pub fn bytes(body: Bytes) -> Self {
        Self {
            body: PdfBody::Bytes(body),
            filename: None,
            inline: false,
            cache_control: None,
        }
    }

    /// Build a `PdfResponse` from a streaming view.
    pub fn stream(stream: poli_page::client::PdfByteStream) -> Self {
        Self {
            body: PdfBody::Stream(stream),
            filename: None,
            inline: false,
            cache_control: None,
        }
    }

    /// Suggest a filename for the browser's Save dialog.
    pub fn filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }

    /// Switch from `attachment` to `inline` disposition (browser preview).
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    /// Override the default `Cache-Control: private, no-store`.
    pub fn cache_control(mut self, value: impl Into<String>) -> Self {
        self.cache_control = Some(value.into());
        self
    }
}

impl<'r> Responder<'r, 'static> for PdfResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        let mut builder = Response::build();
        builder
            .header(ContentType::PDF)
            .header(Header::new(
                "Cache-Control",
                self.cache_control
                    .unwrap_or_else(|| "private, no-store".into()),
            ))
            .header(Header::new("X-Content-Type-Options", "nosniff"));

        let disposition = match self.filename.as_deref() {
            Some(name) => content_disposition(name, self.inline),
            None => {
                if self.inline {
                    "inline".into()
                } else {
                    "attachment".into()
                }
            }
        };
        builder.header(Header::new("Content-Disposition", disposition));

        match self.body {
            PdfBody::Bytes(b) => {
                let len = b.len();
                builder.sized_body(len, Cursor::new(b.to_vec()));
            }
            PdfBody::Stream(stream) => {
                let adapter = StreamReadAdapter {
                    inner: stream,
                    pending: None,
                };
                builder.streamed_body(adapter);
            }
        }

        builder.ok()
    }
}

// Why: PdfByteStream wraps a Pin<Box<dyn Stream + Send>>, which is structurally
// Unpin (Pin<Box<T>> is always Unpin), so safe Pin::new is sufficient and we
// avoid the unsafe escape hatch flagged by #![forbid(unsafe_code)].
struct StreamReadAdapter {
    inner: poli_page::client::PdfByteStream,
    pending: Option<Bytes>,
}

impl rocket::tokio::io::AsyncRead for StreamReadAdapter {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut rocket::tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            if let Some(b) = self.pending.as_mut() {
                let take = buf.remaining().min(b.len());
                buf.put_slice(&b[..take]);
                drop(b.split_to(take));
                if b.is_empty() {
                    self.pending = None;
                }
                return Poll::Ready(Ok(()));
            }
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Ready(Some(Ok(chunk))) => {
                    self.pending = Some(chunk);
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Err(std::io::Error::other(e.to_string())));
                }
            }
        }
    }
}
