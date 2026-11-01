//! A `Send` future borrows disjoint context fields across a real suspension.

use std::future::pending;

/// Request and response state that a handler borrows separately.
#[derive(Default)]
struct Context {
    /// The request-side field borrowed first.
    request: String,
    /// The response-side field borrowed after the await.
    response: Vec<u8>,
}

/// Asserts at compile time that a value is `Send`.
fn assert_send<T: Send>(_value: &T) {}

/// A handler whose future must stay `Send` across a suspension point.
async fn handler(context: &mut Context) -> usize {
    let request = &mut context.request;
    request.push_str("/items");
    pending::<()>().await;
    let response = &mut context.response;
    response.extend_from_slice(request.as_bytes());
    response.len()
}

fn main() {
    let mut context = Context::default();
    let future = handler(&mut context);
    assert_send(&future);
}