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

/// Same cases as poli-page/django#1: control characters stripped, `\` and `"`
/// escaped as quoted-pairs, dual notation for non-ASCII names.
#[test]
fn content_disposition_is_rfc6266_safe() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "double-quote-is-escaped",
            r#"say "hi".pdf"#,
            r#"attachment; filename="say \"hi\".pdf""#,
        ),
        (
            "backslash-is-escaped",
            r"a\b.pdf",
            r#"attachment; filename="a\\b.pdf""#,
        ),
        (
            "crlf-is-stripped",
            "evil.pdf\r\nSet-Cookie: sid=1",
            r#"attachment; filename="evil.pdfSet-Cookie: sid=1""#,
        ),
        (
            "control-chars-are-stripped",
            "tab\there\u{0}\u{1f}\u{7f}.pdf",
            r#"attachment; filename="tabhere.pdf""#,
        ),
        (
            "parameter-injection-stays-inside-the-quoted-string",
            r#"x.pdf"; filename="pwn.exe"#,
            r#"attachment; filename="x.pdf\"; filename=\"pwn.exe""#,
        ),
        (
            "non-ascii-uses-rfc5987-dual-notation",
            "résumé François.pdf",
            r#"attachment; filename="r_sum_ Fran_ois.pdf"; filename*=UTF-8''r%C3%A9sum%C3%A9%20Fran%C3%A7ois.pdf"#,
        ),
        (
            "non-ascii-fallback-is-escaped",
            r#"résumé "final"\v2.pdf"#,
            r#"attachment; filename="r_sum_ \"final\"\\v2.pdf"; filename*=UTF-8''r%C3%A9sum%C3%A9%20%22final%22%5Cv2.pdf"#,
        ),
        (
            "non-ascii-control-chars-are-stripped-from-both-forms",
            "résumé\r\n\u{85}.pdf",
            r#"attachment; filename="r_sum_.pdf"; filename*=UTF-8''r%C3%A9sum%C3%A9.pdf"#,
        ),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(id, filename, expected)| {
            let got = content_disposition(filename, false);
            (got != *expected).then(|| format!("{id}: expected {expected:?}, got {got:?}"))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn content_disposition_inline_is_escaped_and_stripped() {
    assert_eq!(
        content_disposition("q\"\r\n.pdf", true),
        r#"inline; filename="q\".pdf""#,
    );
}
