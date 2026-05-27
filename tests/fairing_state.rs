//! End-to-end: attach the fairing, dispatch a route that takes
//! `&State<PoliPageClient>`.

use poli_page_rocket::{PoliPageClient, PoliPageFairing};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes, State};
use serial_test::serial;

#[get("/probe")]
fn probe(_client: &State<PoliPageClient>) -> &'static str {
    "ok"
}

#[rocket::async_test]
#[serial]
async fn fairing_inserts_client_into_state() {
    std::env::set_var("POLI_PAGE_API_KEY", "pp_test_demo_key");
    let r = rocket::build()
        .attach(PoliPageFairing::from_env())
        .mount("/", routes![probe]);
    let c = Client::tracked(r).await.unwrap();
    let resp = c.get("/probe").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_string().await.unwrap(), "ok");
    std::env::remove_var("POLI_PAGE_API_KEY");
}

#[rocket::async_test]
#[serial]
async fn fairing_with_explicit_builder_works() {
    let builder = poli_page::PoliPage::builder().api_key("pp_test_explicit");
    let r = rocket::build()
        .attach(PoliPageFairing::new(builder))
        .mount("/", routes![probe]);
    let c = Client::tracked(r).await.unwrap();
    assert_eq!(c.get("/probe").dispatch().await.status(), Status::Ok);
}

#[rocket::async_test]
#[serial]
async fn fairing_with_prebuilt_client_works() {
    let client = poli_page::PoliPage::new("pp_test_prebuilt").unwrap();
    let r = rocket::build()
        .attach(PoliPageFairing::with_client(client))
        .mount("/", routes![probe]);
    let c = Client::tracked(r).await.unwrap();
    assert_eq!(c.get("/probe").dispatch().await.status(), Status::Ok);
}
