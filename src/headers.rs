//! RFC 5987 / RFC 8187 `Content-Disposition` `filename*` encoding.
//!
//! Ported character-for-character from the symfony-bundle's
//! `PoliPageResponseFactory::makeDisposition` (which itself uses
//! `Symfony\Component\HttpFoundation\HeaderUtils::makeDisposition`).
//! The bundle's tests are the canonical reference; cases live in
//! `tests/unit_headers.rs`.

use std::fmt::Write as _;

/// Returns `true` when every byte of `s` is a printable ASCII character
/// (`0x20..=0x7E`). Control characters and any byte ≥ `0x7F` count as
/// non-ASCII for the purposes of `filename` encoding.
#[must_use]
pub fn is_ascii_safe(s: &str) -> bool {
    s.bytes().all(|b| (0x20..=0x7E).contains(&b))
}

/// Percent-encode a string for use in the `filename*=UTF-8''<value>`
/// parameter (RFC 5987 / 8187). Encodes any byte outside the `attr-char`
/// set; the conservative implementation here percent-encodes anything that
/// isn't an unreserved URL character (letters, digits, `-`, `_`, `.`, `~`).
#[must_use]
pub fn rfc5987_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Build a `Content-Disposition` header value. If `filename` is ASCII-safe
/// the result is `attachment; filename="<escaped>"`; otherwise both an
/// ASCII fallback and an RFC 5987 `filename*` are emitted.
#[must_use]
pub fn content_disposition(filename: &str, inline: bool) -> String {
    let disposition = if inline { "inline" } else { "attachment" };
    if is_ascii_safe(filename) {
        return format!(r#"{disposition}; filename="{}""#, escape_quotes(filename));
    }
    let ascii_fallback: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encoded = rfc5987_encode(filename);
    format!(
        r#"{disposition}; filename="{}"; filename*=UTF-8''{}"#,
        escape_quotes(&ascii_fallback),
        encoded,
    )
}

fn escape_quotes(s: &str) -> String {
    s.replace('\\', r"\\").replace('"', r#"\""#)
}
