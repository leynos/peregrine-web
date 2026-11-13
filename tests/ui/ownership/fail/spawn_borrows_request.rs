//! A spawned `'static` task cannot capture a borrowed request view.

use peregrine_web::spike::RequestView;

fn main() {
    let headers = http::HeaderMap::new();
    let view = RequestView::new(peregrine_web::spike::Method::Get, "/items", &headers, None);
    let _handle = tokio::spawn(async move { view.path().to_owned() });
}