//! Owned counterparts for every escape the negative fixtures reject.

use std::sync::Arc;

use bytes::Bytes;

/// Asserts at compile time that a value is `Send`.
fn assert_send<T: Send>(_value: &T) {}

/// Owned counterpart of a response body capturing a request borrow.
fn response_body(path: &str) -> Bytes { Bytes::from(path.to_owned()) }

/// Owned counterpart of `Rc`-backed shared state.
fn shared_state() -> Arc<String> { Arc::new(String::from("shared")) }

/// Owned counterpart of non-`Send` state retained across an await.
async fn owned_state_across_await() -> usize {
    let state = String::from("owned");
    tokio::task::yield_now().await;
    state.len()
}

/// Owned counterpart of a task capturing a borrowed request view.
fn spawn_owned_path(path: &str) -> tokio::task::JoinHandle<usize> {
    let owned = path.to_owned();
    tokio::spawn(async move { owned.len() })
}

/// Owned counterpart of simultaneous mutable loans.
fn sequential_loans(value: &mut Vec<u8>) -> usize {
    value.extend_from_slice(b"first");
    let first = value.len();
    value.extend_from_slice(b"second");
    first + value.len()
}

fn main() {
    let body = response_body("/catalogue");
    assert_eq!(
        body,
        Bytes::from_static(b"/catalogue"),
        "the owned copy must reproduce the borrowed request path"
    );
    assert_send(&shared_state());
    let future = owned_state_across_await();
    assert_send(&future);

    // Spawning needs a runtime; the witness is that the future is `Send` and
    // owns everything it captures.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a current-thread runtime should build");
    runtime.block_on(async {
        assert_eq!(owned_state_across_await().await, 5, "owned state should survive the await");
        assert_eq!(
            spawn_owned_path("/billing")
                .await
                .expect("the owned task should join"),
            8,
            "the owned copy should outlive the borrowed request path",
        );
    });

    let mut buffer = Vec::new();
    assert_eq!(
        sequential_loans(&mut buffer),
        16,
        "sequential loans must observe both writes"
    );
}