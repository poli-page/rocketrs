//! Default `on_retry` / `on_error` hooks emitting structured `tracing` events
//! under the `poli_page_rocket` target.

use poli_page::{Error, RetryEvent};

pub(crate) fn on_retry(event: &RetryEvent) {
    tracing::warn!(
        target: "poli_page_rocket",
        attempt = event.attempt,
        delay_ms = u64::try_from(event.delay.as_millis()).unwrap_or(u64::MAX),
        code = event.reason.code(),
        status = event.reason.status(),
        request_id = event.reason.request_id(),
        "poli_page retry",
    );
}

pub(crate) fn on_error(err: &Error) {
    tracing::error!(
        target: "poli_page_rocket",
        code = err.code(),
        status = err.status(),
        request_id = err.request_id(),
        message = %err,
        "poli_page terminal error",
    );
}
