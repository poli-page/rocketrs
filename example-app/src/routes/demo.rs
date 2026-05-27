//! Placeholder demo route — Task 11 replaces this with the interactive
//! HTML dashboard ported from the symfony-bundle.

use rocket::get;
use rocket::response::content::RawHtml;

#[get("/")]
pub fn index() -> RawHtml<&'static str> {
    RawHtml(
        "<!doctype html><meta charset=utf-8>\
         <title>poli-page-rocket example</title>\
         <p>Example app placeholder. Try \
         <a href=\"/render/pdf\">/render/pdf</a>, \
         <a href=\"/render/preview\">/render/preview</a>, \
         or POST /documents.</p>",
    )
}
