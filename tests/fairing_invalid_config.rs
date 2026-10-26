//! `PoliPageFairing::from_env()` rejects bad config at ignite. The API key
//! checks live in `fairing_api_key.rs`.

use poli_page_rocket::PoliPageFairing;
use rocket::local::asynchronous::Client;
use serial_test::serial;

/// Rocket's `Error` panics on drop unless inspected (Rocket 0.5 quirk).
/// `.kind()` marks the error as handled.
fn assert_ignite_failed(res: Result<Client, rocket::Error>, ctx: &str) {
    match res {
        Ok(_) => panic!("expected ignite to fail: {ctx}"),
        Err(e) => {
            let _ = e.kind();
        }
    }
}

#[rocket::async_test]
#[serial]
async fn bad_timeout_fails_ignite() {
    std::env::set_var(
        "POLI_PAGE_API_KEY",
        "pp_sandbox_0123456789abcdef0123456789abcdef",
    );
    std::env::set_var("POLI_PAGE_TIMEOUT_SECS", "not-a-number");
    let r = rocket::build().attach(PoliPageFairing::from_env());
    assert_ignite_failed(Client::tracked(r).await, "non-numeric timeout");
    std::env::remove_var("POLI_PAGE_TIMEOUT_SECS");
    std::env::remove_var("POLI_PAGE_API_KEY");
}
