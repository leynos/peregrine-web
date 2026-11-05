//! Regression witnesses for the fixture's pre-accumulation trailer bound.
//!
//! `drain_body` must reject an over-long trailer set before copying it, so a
//! peer cannot make the fixture accumulate unbounded trailer data. These cases
//! pin the boundary from below, above, across frames, and for repeated trailer
//! names, which `HeaderMap::len` counts individually rather than once per name.

#[path = "support/spike/frames.rs"]
mod spike_frames;
#[path = "support/spike/synthetic.rs"]
mod spike_synthetic;

use std::{
    error::Error,
    sync::{Arc, atomic::AtomicUsize},
};

use http::{HeaderMap, HeaderName, HeaderValue};
use peregrine_web::spike::{ErasedBody, ResourceError, adapt_body};
use rstest::rstest;
use spike_frames::{MAX_TRAILER_PAIRS, drain_body};
use spike_synthetic::{FrameStep, SyntheticBody};

/// Distinct trailer names available to a case; five covers the bound and one
/// pairing over it.
const TRAILER_NAMES: [&str; 5] = ["x-a", "x-b", "x-c", "x-d", "x-e"];

/// Builds a trailer map with `count` distinct single-valued names.
fn distinct_trailers(count: usize) -> Result<HeaderMap, Box<dyn Error>> {
    let names = TRAILER_NAMES
        .get(..count)
        .ok_or("the fixture defines only five trailer names")?;
    let mut headers = HeaderMap::new();
    for name in names {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static("value"),
        );
    }
    Ok(headers)
}

/// Builds a trailer map carrying one name repeated `count` times.
fn repeated_trailers(count: usize) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for _ in 0..count {
        headers.append(
            HeaderName::from_static("x-repeat"),
            HeaderValue::from_static("value"),
        );
    }
    headers
}

/// Wraps scripted frames in the erased body the fixture drains.
fn body_with(steps: Vec<FrameStep>) -> ErasedBody {
    adapt_body(SyntheticBody::new(
        steps,
        None,
        Arc::new(AtomicUsize::new(0)),
    ))
}

/// Fails the case when a fixture expectation does not hold.
fn require(condition: bool, describe: impl FnOnce() -> String) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(describe().into())
    }
}

/// Requires the drain to accept the body and carry the expected pair count.
async fn accept(body: &mut ErasedBody, expected: usize) -> Result<(), Box<dyn Error>> {
    match drain_body(body).await {
        Ok(drained) => require(drained.trailers.len() == expected, || {
            format!(
                "expected {expected} trailer pairs, got {:?}",
                drained.trailers
            )
        }),
        Err(error) => Err(format!("the fixture must accept the trailer set, got {error}").into()),
    }
}

/// Requires the drain to reject the body with the trailer-bound failure.
async fn reject(body: &mut ErasedBody) -> Result<(), Box<dyn Error>> {
    match drain_body(body).await {
        Ok(drained) => Err(format!(
            "the fixture must reject the trailer set, accepted {:?}",
            drained.trailers
        )
        .into()),
        Err(ResourceError::Failed { message }) if message.contains("trailer pairs") => Ok(()),
        Err(other) => Err(format!("expected the trailer-bound failure, got {other}").into()),
    }
}

/// One trailer arrangement whose draining the bound must decide.
///
/// The cases name what they exercise rather than how they are built, so the
/// parameterised tests below read as the boundary table they pin.
#[derive(Clone, Copy, Debug)]
enum TrailerCase {
    /// `count` distinct names carried in a single frame.
    Distinct(usize),
    /// Two frames of distinct names, with the given sizes.
    Split(usize, usize),
    /// One name repeated `count` times in a single frame.
    Repeated(usize),
}

impl TrailerCase {
    /// Builds the scripted frames this case describes.
    fn frames(self) -> Result<Vec<FrameStep>, Box<dyn Error>> {
        match self {
            Self::Distinct(count) => Ok(vec![FrameStep::Trailers(distinct_trailers(count)?)]),
            Self::Split(first, second) => Ok(vec![
                FrameStep::Trailers(distinct_trailers(first)?),
                FrameStep::Trailers(distinct_trailers(second)?),
            ]),
            Self::Repeated(count) => Ok(vec![FrameStep::Trailers(repeated_trailers(count))]),
        }
    }
}

/// Checks that trailer sets at the bound are accepted: four distinct pairs,
/// four pairs split across frames, and four repetitions of one trailer name.
#[rstest]
#[case::exactly_four_distinct_trailer_pairs_are_accepted(
    TrailerCase::Distinct(MAX_TRAILER_PAIRS),
    MAX_TRAILER_PAIRS
)]
#[case::four_pairs_split_across_frames_are_accepted(TrailerCase::Split(2, 2), MAX_TRAILER_PAIRS)]
#[case::a_repeated_trailer_name_counts_individual_values(
    TrailerCase::Repeated(MAX_TRAILER_PAIRS),
    MAX_TRAILER_PAIRS
)]
#[tokio::test]
async fn trailer_pairs_within_the_bound_are_accepted(
    #[case] case: TrailerCase,
    #[case] expected: usize,
) -> Result<(), Box<dyn Error>> {
    let mut body = body_with(case.frames()?);
    accept(&mut body, expected).await
}

/// Checks that a trailer set over the bound is rejected before accumulation,
/// whether the excess arrives as distinct names, across frames, or as one
/// repeated name.
#[rstest]
#[case::five_distinct_trailer_pairs_are_rejected(TrailerCase::Distinct(MAX_TRAILER_PAIRS + 1))]
#[case::trailer_pairs_accumulate_across_frames_before_rejection(TrailerCase::Split(3, 2))]
#[case::a_repeated_trailer_name_beyond_the_bound_is_rejected(
    TrailerCase::Repeated(MAX_TRAILER_PAIRS + 1)
)]
#[tokio::test]
async fn trailer_pairs_beyond_the_bound_are_rejected(
    #[case] case: TrailerCase,
) -> Result<(), Box<dyn Error>> {
    let mut body = body_with(case.frames()?);
    reject(&mut body).await
}
