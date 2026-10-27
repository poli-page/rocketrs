//! Rocket fairing that builds the SDK client at ignite and inserts it
//! into managed state as [`PoliPageClient`].

use std::sync::Mutex;
use std::time::Duration;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Rocket};

use crate::state::PoliPageClient;
use crate::tracing_bridge;

/// Longest environment name the API issues keys for.
const ENV_NAME_MAX_LEN: usize = 20;

/// Rocket fairing that builds a `poli_page::PoliPage` at ignite-time and
/// inserts it into managed state as [`PoliPageClient`].
///
/// Three constructors:
/// - [`from_env`](Self::from_env) — read all options from environment variables.
/// - [`new`](Self::new) — take a fully-configured `PoliPageBuilder`.
/// - [`with_client`](Self::with_client) — wrap an already-built client.
#[must_use]
pub struct PoliPageFairing {
    // Why: Fairing::on_ignite takes &self in Rocket 0.5 even though it
    // semantically consumes the configuration; a Mutex lets us drain the
    // Source from behind a shared reference. Ignite runs at most once per
    // attach so contention is impossible.
    source: Mutex<Source>,
}

enum Source {
    Env,
    Builder(Box<poli_page::PoliPageBuilder>),
    Built(poli_page::PoliPage),
    Drained,
}

impl PoliPageFairing {
    /// Read all options from environment variables (see spec §6.3).
    /// Ignite fails if `POLI_PAGE_API_KEY` is missing or empty. Any other
    /// value ignites: the Poli Page API is the only authority on keys, so a
    /// key that does not look like `pp_{environment}_{32 lowercase hex}` only
    /// logs one warning.
    pub fn from_env() -> Self {
        Self {
            source: Mutex::new(Source::Env),
        }
    }

    /// Build a fairing from an already-configured `PoliPageBuilder`.
    pub fn new(builder: poli_page::PoliPageBuilder) -> Self {
        Self {
            source: Mutex::new(Source::Builder(Box::new(builder))),
        }
    }

    /// Wrap an already-built client (e.g. one shared with non-Rocket code).
    pub fn with_client(client: poli_page::PoliPage) -> Self {
        Self {
            source: Mutex::new(Source::Built(client)),
        }
    }
}

#[rocket::async_trait]
impl Fairing for PoliPageFairing {
    fn info(&self) -> Info {
        Info {
            name: "poli-page-rocket",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        let source = {
            let mut guard = self
                .source
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::replace(&mut *guard, Source::Drained)
        };
        let client = match source {
            Source::Built(c) => c,
            Source::Builder(b) => {
                let b = b
                    .on_retry(tracing_bridge::on_retry)
                    .on_error(tracing_bridge::on_error);
                match b.build() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(
                            target: "poli_page_rocket",
                            error = %e,
                            "PoliPage build failed",
                        );
                        return Err(rocket);
                    }
                }
            }
            Source::Env => match build_from_env() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(
                        target: "poli_page_rocket",
                        error = %e,
                        "PoliPage env config invalid",
                    );
                    return Err(rocket);
                }
            },
            Source::Drained => {
                tracing::error!(
                    target: "poli_page_rocket",
                    "PoliPageFairing.on_ignite called twice; the second call has no config to apply.",
                );
                return Err(rocket);
            }
        };
        Ok(rocket.manage(PoliPageClient(client)))
    }
}

fn build_from_env() -> Result<poli_page::PoliPage, String> {
    let api_key = std::env::var("POLI_PAGE_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| {
            "POLI_PAGE_API_KEY is not set or empty. \
             Get a key at https://app.poli.page/settings/api-keys."
                .to_owned()
        })?;
    if !well_formed_api_key(&api_key) {
        // No part of the key is logged, not even a grammar-valid `pp_{env}_`:
        // it may be a mis-pasted secret fragment.
        tracing::warn!(
            target: "poli_page_rocket",
            "POLI_PAGE_API_KEY does not match the expected format \
             pp_{{environment}}_{{32 lowercase hex characters}}; \
             starting anyway, the Poli Page API decides whether the key is valid",
        );
    }
    let mut builder = poli_page::PoliPage::builder()
        .api_key(api_key)
        .on_retry(tracing_bridge::on_retry)
        .on_error(tracing_bridge::on_error);

    if let Ok(v) = std::env::var("POLI_PAGE_BASE_URL") {
        builder = builder.base_url(v);
    }
    if let Ok(v) = std::env::var("POLI_PAGE_TIMEOUT_SECS") {
        let secs: u64 = v
            .parse()
            .map_err(|_| format!("POLI_PAGE_TIMEOUT_SECS={v:?} is not a number"))?;
        builder = builder.timeout(Duration::from_secs(secs));
    }
    if let Ok(v) = std::env::var("POLI_PAGE_MAX_RETRIES") {
        let n: u32 = v
            .parse()
            .map_err(|_| format!("POLI_PAGE_MAX_RETRIES={v:?} is not a number"))?;
        builder = builder.max_retries(n);
    }
    if let Ok(v) = std::env::var("POLI_PAGE_RETRY_DELAY_MS") {
        let ms: u64 = v
            .parse()
            .map_err(|_| format!("POLI_PAGE_RETRY_DELAY_MS={v:?} is not a number"))?;
        builder = builder.retry_delay(Duration::from_millis(ms));
    }
    builder.build().map_err(|e| e.to_string())
}

/// Splits `pp_{env}_{secret}` at the first `_` after `pp_`.
fn split_api_key(key: &str) -> Option<(&str, &str)> {
    key.strip_prefix("pp_")?.split_once('_')
}

/// Whether `key` matches the SDK contract's full-key regex
/// `^pp_(?=[a-z][a-z0-9-]{0,19}_)[a-z][a-z0-9]*(?:-[a-z0-9]+)*_[0-9a-f]{32}$`.
fn well_formed_api_key(key: &str) -> bool {
    let Some((env, secret)) = split_api_key(key) else {
        return false;
    };
    valid_env_name(env)
        && secret.len() == 32
        && secret
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Whether `env` is an environment name: a lowercase letter, then lowercase
/// letters, digits and single inner hyphens, at most 20 characters.
fn valid_env_name(env: &str) -> bool {
    env.len() <= ENV_NAME_MAX_LEN
        && env.starts_with(|c: char| c.is_ascii_lowercase())
        && env.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
