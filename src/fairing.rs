//! Rocket fairing that builds the SDK client at ignite and inserts it
//! into managed state as [`PoliPageClient`].

use std::sync::Mutex;
use std::time::Duration;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Rocket};

use crate::state::PoliPageClient;
use crate::tracing_bridge;

const KEY_PATTERN_PREFIXES: &[&str] = &["pp_test_", "pp_live_"];

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
    /// Ignite fails if `POLI_PAGE_API_KEY` is missing or malformed.
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
        .map_err(|_| "POLI_PAGE_API_KEY is not set".to_owned())?;
    if !KEY_PATTERN_PREFIXES.iter().any(|p| api_key.starts_with(p)) {
        return Err(format!(
            "POLI_PAGE_API_KEY must start with pp_test_ or pp_live_; got prefix {:?}. \
             Get a key at https://app.poli.page/settings/api-keys.",
            api_key.chars().take(8).collect::<String>(),
        ));
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
