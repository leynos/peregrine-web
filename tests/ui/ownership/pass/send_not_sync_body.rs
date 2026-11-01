//! An erased body may be `Send` without being `Sync`, and may cross an await.

use std::cell::Cell;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use http_body::{Body, Frame};
use http_body_util::BodyExt;
use peregrine_web::spike::{ErasedBody, adapt_body};

/// A body with interior mutability: `Send` but deliberately not `Sync`.
struct Synthetic {
    /// Poll observations, held in a `Cell` to remove `Sync`.
    polls: Cell<usize>,
}

impl Body for Synthetic {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        let this = self.get_mut();
        this.polls.set(this.polls.get() + 1);
        Poll::Ready(None)
    }
}

/// Asserts at compile time that a value is `Send`.
fn assert_send<T: Send>(_value: &T) {}

/// Consumes one frame, suspending the future while the body is alive.
async fn consume(mut body: ErasedBody) -> usize {
    let frame = body.frame().await;
    usize::from(frame.is_some())
}

fn main() {
    let body = adapt_body(Synthetic { polls: Cell::new(0) });
    let future = consume(body);
    assert_send(&future);
}