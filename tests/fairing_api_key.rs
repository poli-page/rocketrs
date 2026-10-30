//! `PoliPageFairing::from_env()` only requires a non-empty
//! `POLI_PAGE_API_KEY`. A key that does not match the contract
//! `pp_{env}_{32 lowercase hex}` ignites anyway, with exactly one warning
//! that names the expected format and shows no part of the key, not even a
//! grammar-valid `pp_{env}_` (it may be a mis-pasted secret fragment).

use poli_page_rocket::PoliPageFairing;
use rocket::local::blocking::Client;
use serial_test::serial;
use tracing::subscriber::with_default;

mod common;
use common::CaptureWriter;

const HEX: &str = "0123456789abcdef0123456789abcdef";
const ENV_20: &str = "abcdefghijklmnopqrst";
const ENV_21: &str = "abcdefghijklmnopqrstu";

/// The whole malformed-key warning line: a message and no field.
const MALFORMED_KEY_WARNING: &str = "WARN poli_page_rocket: POLI_PAGE_API_KEY does not match \
     the expected format pp_{environment}_{32 lowercase hex characters}; starting anyway, \
     the Poli Page API decides whether the key is valid";

/// Ignites a fairing that reads `POLI_PAGE_API_KEY` (`None`: unset) and
/// returns whether ignition succeeded, plus everything that was logged.
fn ignite_with_key(key: Option<&str>) -> (bool, String) {
    match key {
        Some(k) => std::env::set_var("POLI_PAGE_API_KEY", k),
        None => std::env::remove_var("POLI_PAGE_API_KEY"),
    }
    let writer = CaptureWriter::default();
    let ignited = with_default(writer.subscriber(), || {
        match Client::tracked(rocket::build().attach(PoliPageFairing::from_env())) {
            Ok(_) => true,
            Err(e) => {
                // Rocket 0.5 panics on dropping an uninspected `Error`.
                let _ = e.kind();
                false
            }
        }
    });
    std::env::remove_var("POLI_PAGE_API_KEY");
    (ignited, writer.output())
}

fn warnings(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| l.trim_start().starts_with("WARN"))
        .collect()
}

/// Text of `key` that must never be logged: the key, each of its lines, the
/// `{a}_{b}_` prefix of each line and each `_`-separated part, when three or
/// more characters long (shorter ones, like `pp` or `x`, occur in the
/// message).
fn key_text(key: &str) -> Vec<String> {
    let mut text = vec![key.to_owned(), key.trim().to_owned()];
    for line in key.lines() {
        text.push(line.to_owned());
        let parts: Vec<&str> = line.split('_').collect();
        if parts.len() >= 3 {
            text.push(format!("{}_{}_", parts[0], parts[1]));
        }
        text.extend(parts.iter().map(|p| (*p).to_owned()));
    }
    text.retain(|t| t.chars().count() >= 3);
    text
}

#[test]
#[serial]
fn missing_key_fails_ignition_without_warning() {
    let (ignited, out) = ignite_with_key(None);
    assert!(!ignited, "ignited with no key: {out}");
    assert!(warnings(&out).is_empty(), "unexpected warning: {out}");
    assert!(
        out.contains("POLI_PAGE_API_KEY"),
        "error names the var: {out}"
    );
}

#[test]
#[serial]
fn empty_key_fails_ignition_without_warning() {
    let (ignited, out) = ignite_with_key(Some(""));
    assert!(!ignited, "ignited with an empty key: {out}");
    assert!(warnings(&out).is_empty(), "unexpected warning: {out}");
    // The fairing's own check, not the SDK's "api_key is required".
    assert!(
        out.contains(
            "POLI_PAGE_API_KEY is not set or empty. \
             Get a key at https://app.poli.page/settings/api-keys."
        ),
        "error names the var: {out}"
    );
}

#[test]
#[serial]
fn well_formed_keys_ignite_without_warning() {
    for env in [
        "sandbox", "live", "staging", "eu-prod", "a", "a1-b2-c3", ENV_20,
    ] {
        let key = format!("pp_{env}_{HEX}");
        let (ignited, out) = ignite_with_key(Some(&key));
        assert!(ignited, "{key} did not ignite: {out}");
        assert!(warnings(&out).is_empty(), "{key} warned: {out}");
    }
}

#[test]
#[serial]
fn malformed_keys_ignite_with_one_warning_that_shows_no_key_text() {
    let upper = HEX.to_uppercase();
    let hex31 = &HEX[..31];
    let hex16 = &HEX[..16];
    let keys: Vec<String> = vec![
        // A grammar-valid environment with a bad secret: too short, wrong
        // case, wrong length, extra characters at either end.
        "pp_sandbox_x".into(),
        "pp_eu-prod_x".into(),
        format!("pp_{ENV_20}_x"),
        format!("pp_sandbox_{upper}"),
        format!("pp_sandbox_{hex31}"),
        format!("pp_sandbox_{HEX}0"),
        format!("pp_sandbox_{hex31}g"),
        format!("pp_sandbox_{HEX}x"),
        format!("xpp_sandbox_{HEX}"),
        format!("pp_sandbox_s3cr3t_{HEX}"),
        // A hex fragment in the environment slot is grammar-valid.
        format!("pp_abc123_{hex16}"),
        // A trailing newline, or a line before the key.
        format!("pp_sandbox_{HEX}\n"),
        format!("pp_sandbox_{HEX}\r\n"),
        format!("x\npp_sandbox_{HEX}"),
        // Environment names: uppercase (first, inner, after a hyphen),
        // punctuation, non-ASCII, leading digit, double hyphen, leading or
        // trailing hyphen, too long (in bytes too), empty.
        format!("pp_sa.box_{HEX}"),
        format!("pp_sändbox_{HEX}"),
        format!("pp_Sandbox_{HEX}"),
        format!("pp_sAndbox_{HEX}"),
        format!("pp_eu-Prod_{HEX}"),
        format!("pp_2fa_{HEX}"),
        format!("pp_0123456789abcdef_{HEX}"),
        format!("pp_a--b_{HEX}"),
        format!("pp_-ab_{HEX}"),
        format!("pp_ab-_{HEX}"),
        format!("pp_{ENV_21}_{HEX}"),
        format!("pp_{ENV_21}_x"),
        format!("pp_{}_x", "ä".repeat(11)),
        format!("pp__{HEX}"),
        // No environment at all, or not a Poli Page key.
        format!("pp_{HEX}"),
        format!("sk_live_{HEX}"),
        format!("px_sandbox_{HEX}"),
        format!("PP_sandbox_{HEX}"),
    ];
    for key in keys {
        let (ignited, out) = ignite_with_key(Some(&key));
        assert!(ignited, "{key:?} did not ignite: {out}");
        let warned = warnings(&out);
        assert_eq!(warned.len(), 1, "{key:?}: expected one warning: {out}");
        assert_eq!(
            warned[0].trim(),
            MALFORMED_KEY_WARNING,
            "{key:?}: the warning has no field, so no key text"
        );
        for text in key_text(&key) {
            assert!(!out.contains(&text), "{key:?}: {text:?} logged: {out}");
        }
    }
}
