//! `PoliPageClient` satisfies the bounds Rocket's managed state requires.

use poli_page_rocket::PoliPageClient;

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

fn assert_clone<T: Clone>() {}

#[test]
fn poli_page_client_is_send_sync_static() {
    assert_send_sync_static::<PoliPageClient>();
}

#[test]
fn poli_page_client_is_clone() {
    assert_clone::<PoliPageClient>();
}
