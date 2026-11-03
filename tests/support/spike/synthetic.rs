//! A scripted request body that is `Send` but deliberately not `Sync`.
//!
//! The body carries a `Cell`, which removes `Sync` without removing `Send`. Any
//! test that compiles against it therefore witnesses the bound the spike claims
//! for the erased body alias, and the scripted steps let a test inject a
//! suspension point, an empty data frame, trailers, or a terminal error.
//!
//! # Lint scope
//!
//! Each integration binary includes a different subset of the fixture modules,
//! so an item used only by a sibling binary looks unused here. The allowance is
//! confined to this fixture module rather than the whole test crate.

#![allow(
    dead_code,
    reason = "each integration binary includes a different subset of these fixtures"
)]

use std::{
    cell::Cell,
    collections::VecDeque,
    fmt,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

use bytes::Bytes;
use http::HeaderMap;
use http_body::{Body, Frame, SizeHint};

/// Maximum poll observations one synthetic body accepts before failing.
pub const MAX_POLLS: usize = 64;

/// One scripted step in a synthetic body.
#[derive(Debug)]
pub enum FrameStep {
    /// Report `Pending` once, without waking the task.
    Pending,
    /// Yield one data frame.
    Data(Bytes),
    /// Yield one trailers frame.
    Trailers(HeaderMap),
    /// Yield the body's terminal error.
    Error(&'static str),
}

/// A synthetic body that is `Send` but deliberately not `Sync`.
#[derive(Debug)]
pub struct SyntheticBody {
    /// The scripted frames, consumed in order.
    steps: VecDeque<FrameStep>,
    /// Interior mutability that makes the body non-`Sync`.
    polls: Cell<usize>,
    /// A shared poll counter observable after erasure.
    poll_counter: Arc<AtomicUsize>,
    /// Drop observer consumed with the body; never read, only dropped.
    _probe: Option<DropProbe>,
}

impl SyntheticBody {
    /// Creates a synthetic body from its script of steps.
    pub fn new(
        steps: Vec<FrameStep>,
        probe: Option<DropProbe>,
        poll_counter: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            steps: VecDeque::from(steps),
            polls: Cell::new(0),
            poll_counter,
            _probe: probe,
        }
    }
}

impl Body for SyntheticBody {
    type Data = Bytes;
    type Error = SyntheticError;

    fn size_hint(&self) -> SizeHint { SizeHint::default() }

    fn is_end_stream(&self) -> bool { self.steps.is_empty() }

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, SyntheticError>>> {
        let this = self.get_mut();
        if this.polls.get() >= MAX_POLLS {
            return Poll::Ready(Some(Err(SyntheticError::new(
                "synthetic body exceeded its poll bound",
            ))));
        }
        this.polls.set(this.polls.get() + 1);
        this.poll_counter.fetch_add(1, Ordering::SeqCst);
        match this.steps.pop_front() {
            None => Poll::Ready(None),
            Some(FrameStep::Pending) => Poll::Pending,
            Some(FrameStep::Data(bytes)) => Poll::Ready(Some(Ok(Frame::data(bytes)))),
            Some(FrameStep::Trailers(headers)) => Poll::Ready(Some(Ok(Frame::trailers(headers)))),
            Some(FrameStep::Error(message)) => Poll::Ready(Some(Err(SyntheticError::new(message)))),
        }
    }
}

/// The synthetic body's terminal error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyntheticError {
    /// A stable diagnostic message.
    message: &'static str,
}

impl SyntheticError {
    /// Creates a synthetic error with a stable message.
    #[must_use]
    pub const fn new(message: &'static str) -> Self { Self { message } }
}

impl fmt::Display for SyntheticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for SyntheticError {}

/// Increments a shared counter when dropped.
#[derive(Debug)]
pub struct DropProbe {
    /// The counter shared with the test.
    drops: Arc<AtomicUsize>,
}

impl DropProbe {
    /// Creates a probe that increments the supplied counter once.
    pub const fn new(drops: Arc<AtomicUsize>) -> Self { Self { drops } }
}

impl Drop for DropProbe {
    fn drop(&mut self) { self.drops.fetch_add(1, Ordering::SeqCst); }
}

/// Creates a drop probe and the counter it increments.
pub fn drop_counter() -> (DropProbe, Arc<AtomicUsize>) {
    let drops = Arc::new(AtomicUsize::new(0));
    (DropProbe::new(Arc::clone(&drops)), drops)
}

/// Polls a pinned future once with a no-op waker.
pub fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    future.poll(&mut context)
}
