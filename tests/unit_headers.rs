//! Header-encoding behaviour (RFC 5987 / RFC 8187 `filename*` parameter).

use poli_page_rocket::headers::{content_disposition, is_ascii_safe, rfc5987_encode};

#[test]
fn is_ascii_safe_true_for_plain_ascii() {
    assert!(is_ascii_safe("invoice-123.pdf"));
}

#[test]
fn is_ascii_safe_false_for_non_ascii() {
    assert!(!is_ascii_safe("facture-éléphant.pdf"));
}

#[test]
fn is_ascii_safe_false_for_control_chars() {
    assert!(!is_ascii_safe("filename\u{0007}.pdf"));
}

#[test]
fn rfc5987_encode_percent_encodes_utf8_bytes() {
    assert_eq!(rfc5987_encode("café.pdf"), "caf%C3%A9.pdf");
}

#[test]
fn rfc5987_encode_leaves_attr_chars_alone() {
    assert_eq!(rfc5987_encode("plain.pdf"), "plain.pdf");
}

#[test]
fn content_disposition_attachment_for_ascii() {
    assert_eq!(
        content_disposition("invoice.pdf", false),
        r#"attachment; filename="invoice.pdf""#,
    );
}

#[test]
fn content_disposition_inline_when_inline_true() {
    assert_eq!(
        content_disposition("invoice.pdf", true),
        r#"inline; filename="invoice.pdf""#,
    );
}

#[test]
fn content_disposition_emits_both_fallback_and_filename_star_for_non_ascii() {
    assert_eq!(
        content_disposition("café.pdf", false),
        r#"attachment; filename="caf_.pdf"; filename*=UTF-8''caf%C3%A9.pdf"#,
    );
}

#[test]
fn content_disposition_escapes_embedded_quotes() {
    assert!(content_disposition(r#"say "hi".pdf"#, false).contains(r#"filename="say \"hi\".pdf""#));
}
