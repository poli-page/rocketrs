use poli_page_rocket::responses::DocumentRedirect;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::{get, routes};

#[get("/redirect")]
fn redirect_route() -> DocumentRedirect {
    DocumentRedirect::to("https://example.com/x.pdf")
}

#[get("/redirect-permanent")]
fn redirect_permanent_route() -> DocumentRedirect {
    DocumentRedirect::to("https://example.com/x.pdf").permanent()
}

async fn client() -> Client {
    let r = rocket::build().mount("/", routes![redirect_route, redirect_permanent_route]);
    Client::tracked(r).await.unwrap()
}

#[rocket::async_test]
async fn redirect_default_is_302() {
    let c = client().await;
    let r = c.get("/redirect").dispatch().await;
    assert_eq!(r.status(), Status::Found);
    assert_eq!(
        r.headers().get_one("location"),
        Some("https://example.com/x.pdf")
    );
    assert_eq!(
        r.headers().get_one("cache-control"),
        Some("private, no-store")
    );
}

#[rocket::async_test]
async fn redirect_permanent_is_308() {
    let c = client().await;
    let r = c.get("/redirect-permanent").dispatch().await;
    assert_eq!(r.status(), Status::PermanentRedirect);
}
