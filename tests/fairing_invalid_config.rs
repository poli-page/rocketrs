//! `PoliPageFairing::from_env()` rejects bad config at ignite.

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
async fn missing_api_key_fails_ignite() {
    std::env::remove_var("POLI_PAGE_API_KEY");
    let r = rocket::build().attach(PoliPageFairing::from_env());
    assert_ignite_failed(Client::tracked(r).await, "POLI_PAGE_API_KEY missing");
}

#[rocket::async_test]
#[serial]
async fn bad_key_prefix_fails_ignite() {
    std::env::set_var("POLI_PAGE_API_KEY", "sk_test_wrong_prefix");
    let r = rocket::build().attach(PoliPageFairing::from_env());
    assert_ignite_failed(Client::tracked(r).await, "wrong key prefix");
    std::env::remove_var("POLI_PAGE_API_KEY");
}

#[rocket::async_test]
#[serial]
async fn bad_timeout_fails_ignite() {
    std::env::set_var("POLI_PAGE_API_KEY", "pp_test_ok");
    std::env::set_var("POLI_PAGE_TIMEOUT_SECS", "not-a-number");
    let r = rocket::build().attach(PoliPageFairing::from_env());
    assert_ignite_failed(Client::tracked(r).await, "non-numeric timeout");
    std::env::remove_var("POLI_PAGE_TIMEOUT_SECS");
    std::env::remove_var("POLI_PAGE_API_KEY");
}
