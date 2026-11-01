//! A non-`Send` local is dropped before the await that requires `Send`.

use std::future::pending;
use std::rc::Rc;

/// Asserts at compile time that a value is `Send`.
fn assert_send<T: Send>(_value: &T) {}

/// A handler whose only non-`Send` value ends before the suspension point.
async fn handler() -> usize {
    let length = {
        let local = Rc::new(String::from("local"));
        local.len()
    };
    pending::<()>().await;
    length
}

fn main() {
    let future = handler();
    assert_send(&future);
}