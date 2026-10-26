//! `PoliPageFairing::from_env()` only requires a non-empty
//! `POLI_PAGE_API_KEY`. A key that does not match the contract
//! `pp_{env}_{32 lowercase hex}` ignites anyway, with exactly one warning
//! that shows at most the `pp_{env}_` part of the key.

use poli_page_rocket::PoliPageFairing;
use rocket::local::blocking::Client;
use serial_test::serial;
use tracing::subscriber::with_default;

mod common;
use common::CaptureWriter;

const HEX: &str = "0123456789abcdef0123456789abcdef";
const ENV_20: &str = "abcdefghijklmnopqrst";
const ENV_21: &str = "abcdefghijklmnopqrstu";

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
        out.contains("POLI_PAGE_API_KEY"),
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
fn malformed_keys_ignite_with_one_warning_that_hides_the_secret() {
    let upper = HEX.to_uppercase();
    let hex31 = &HEX[..31];
    let hex33 = format!("{HEX}0");
    // (key, the part that must never be logged, the key_prefix shown)
    let cases: Vec<(String, String, String)> = vec![
        // Secret too short.
        (
            "pp_sandbox_x".into(),
            "pp_sandbox_x".into(),
            "pp_sandbox_…".into(),
        ),
        // Environment names: uppercase, leading digit, double hyphen,
        // leading or trailing hyphen, too long, empty.
        (
            format!("pp_Sandbox_{HEX}"),
            HEX.into(),
            "pp_Sandbox_…".into(),
        ),
        (format!("pp_2fa_{HEX}"), HEX.into(), "pp_2fa_…".into()),
        (format!("pp_a--b_{HEX}"), HEX.into(), "pp_a--b_…".into()),
        (format!("pp_-ab_{HEX}"), HEX.into(), "pp_-ab_…".into()),
        (format!("pp_ab-_{HEX}"), HEX.into(), "pp_ab-_…".into()),
        (format!("pp_{ENV_21}_{HEX}"), HEX.into(), "none".into()),
        (format!("pp__{HEX}"), HEX.into(), "none".into()),
        // Secret: wrong case, wrong length, extra characters at either end.
        (
            format!("pp_sandbox_{upper}"),
            upper.clone(),
            "pp_sandbox_…".into(),
        ),
        (
            format!("pp_sandbox_{hex31}"),
            hex31.into(),
            "pp_sandbox_…".into(),
        ),
        (
            format!("pp_sandbox_{hex33}"),
            HEX.into(),
            "pp_sandbox_…".into(),
        ),
        (
            format!("pp_sandbox_{hex31}g"),
            hex31.into(),
            "pp_sandbox_…".into(),
        ),
        (
            format!("pp_sandbox_{HEX}x"),
            HEX.into(),
            "pp_sandbox_…".into(),
        ),
        (format!("xpp_sandbox_{HEX}"), HEX.into(), "none".into()),
        // Only the first `_`-delimited part after `pp_` is shown.
        (
            format!("pp_sandbox_s3cr3t_{HEX}"),
            "s3cr3t".into(),
            "pp_sandbox_…".into(),
        ),
        // No environment at all: the secret right after `pp_`.
        (format!("pp_{HEX}"), HEX.into(), "none".into()),
        // Not a Poli Page key.
        (format!("sk_live_{HEX}"), HEX.into(), "none".into()),
        (format!("px_sandbox_{HEX}"), HEX.into(), "none".into()),
        (format!("PP_sandbox_{HEX}"), HEX.into(), "none".into()),
        // The shown prefix stops at a 20-character environment name.
        (
            format!("pp_{ENV_20}_x"),
            format!("pp_{ENV_20}_x"),
            format!("pp_{ENV_20}_…"),
        ),
        (format!("pp_{ENV_21}_x"), ENV_21.into(), "none".into()),
    ];
    for (key, secret, shown) in cases {
        let (ignited, out) = ignite_with_key(Some(&key));
        assert!(ignited, "{key} did not ignite: {out}");
        let warned = warnings(&out);
        assert_eq!(warned.len(), 1, "{key}: expected one warning: {out}");
        assert!(
            warned[0].contains(&format!("key_prefix={shown:?}")),
            "{key}: expected key_prefix={shown:?}: {out}"
        );
        assert!(!out.contains(&key), "{key}: key logged: {out}");
        assert!(!out.contains(&secret), "{key}: secret logged: {out}");
    }
}
