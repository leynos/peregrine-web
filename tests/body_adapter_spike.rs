//! Adapter fidelity and ownership evidence for erased request bodies.
//!
//! The synthetic body is `Send` but deliberately not `Sync`, so every test that
//! compiles here also witnesses the bound the spike claims. Frames are compared
//! frame by frame, including empty data frames, because a concatenated-bytes
//! oracle cannot see a dropped empty frame or a changed frame boundary.

#[path = "support/spike/frames.rs"]
mod spike_frames;
#[path = "support/spike/synthetic.rs"]
mod spike_synthetic;

#[cfg(test)]
mod runtime_support {
    //! Test-only support that the frame and property tests share.
    //!
    //! The module is test-only so lints that distinguish test code from
    //! production code can see that the helpers here are not shipped.

    /// Builds a current-thread runtime for draining bodies in synchronous tests.
    pub(super) fn current_thread_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a current-thread runtime should build")
    }
}

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use bytes::Bytes;
use http::{
    HeaderMap,
    header::{HeaderName, HeaderValue},
};
use http_body_util::BodyExt;
use peregrine_web::spike::{BodyError, ErasedBody, adapt_body};
use proptest::prelude::*;
use runtime_support::current_thread_runtime;
use spike_frames::{CollectedFrame, collect_frames, normalise_trailers};
use spike_synthetic::{DropProbe, FrameStep, SyntheticBody, drop_counter, poll_once};

/// Trailer names that are always syntactically valid.
const TRAILER_NAMES: [&str; 3] = ["x-checksum", "x-tenant", "x-trace"];

/// A generated frame partition with optional trailers.
#[derive(Clone, Debug)]
struct FrameCase {
    /// The data chunks in emission order, including deliberately empty ones.
    chunks: Vec<Vec<u8>>,
    /// Trailer name/value pairs, or `None` when the case carries no trailers.
    trailers: Option<Vec<(&'static str, String)>>,
}

impl FrameCase {
    /// Returns a stable description used in assertion messages.
    fn description(&self) -> String {
        format!(
            "chunks={}, bytes={}, trailers={}",
            self.chunks.len(),
            self.chunks.iter().map(Vec::len).sum::<usize>(),
            self.trailers.as_ref().map_or(0, Vec::len),
        )
    }
}

/// Splits a payload into `count` contiguous chunks, including empty ones.
fn partition(payload: &[u8], count: usize) -> Vec<Vec<u8>> {
    if payload.is_empty() || count == 0 {
        return vec![Vec::new(); count];
    }
    let chunk_size = payload.len().div_ceil(count);
    let mut chunks: Vec<Vec<u8>> = payload.chunks(chunk_size).map(<[u8]>::to_vec).collect();
    chunks.resize(count, Vec::new());
    chunks
}

/// Generates an ASCII value that is always a legal header value.
fn header_value() -> impl Strategy<Value = String> {
    proptest::collection::vec(prop::sample::select(b"0123456789abcdef".to_vec()), 0..=16)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect())
}

/// Generates one trailer pair drawn from the always-valid names.
fn trailer_pair() -> impl Strategy<Value = (&'static str, String)> {
    (0..TRAILER_NAMES.len()).prop_flat_map(|index| {
        // `get` keeps the generator total even if the name table is edited.
        let name = TRAILER_NAMES.get(index).copied().unwrap_or("x-trace");
        header_value().prop_map(move |value| (name, value))
    })
}

/// Generates frame partitions inside the spike's documented bounds.
fn frame_cases() -> impl Strategy<Value = FrameCase> {
    let payload_and_count = prop_oneof![
        Just((Vec::<u8>::new(), 0usize)),
        (
            proptest::collection::vec(any::<u8>(), 0..=4096),
            1usize..=32usize
        ),
    ];
    let trailer_strategy = prop::option::of(prop::collection::vec(
        trailer_pair(),
        0..=TRAILER_NAMES.len(),
    ));
    (payload_and_count, trailer_strategy).prop_map(|((payload, count), trailers)| FrameCase {
        chunks: partition(&payload, count),
        trailers,
    })
}

/// Builds the scripted steps for one generated case.
fn steps_for(case: &FrameCase) -> Vec<FrameStep> {
    let mut steps: Vec<FrameStep> = case
        .chunks
        .iter()
        .cloned()
        .map(|chunk| FrameStep::Data(Bytes::from(chunk)))
        .collect();
    if let Some(pairs) = case.trailers.as_ref() {
        steps.push(FrameStep::Trailers(headers_for(pairs)));
    }
    steps
}

/// Builds the trailers frame a case emits.
fn headers_for(pairs: &[(&'static str, String)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs {
        if let Ok(parsed) = HeaderValue::from_str(value) {
            headers.append(HeaderName::from_static(name), parsed);
        }
    }
    headers
}

/// Builds the frame sequence a case must reproduce, independently of the body.
fn expected_frames(case: &FrameCase) -> Vec<CollectedFrame> {
    let mut frames: Vec<CollectedFrame> = case
        .chunks
        .iter()
        .cloned()
        .map(|chunk| CollectedFrame::Data(Bytes::from(chunk)))
        .collect();
    if let Some(pairs) = case.trailers.as_ref() {
        frames.push(CollectedFrame::Trailers(normalise_trailers(&headers_for(
            pairs,
        ))));
    }
    frames
}

/// Sorts trailer pairs so header-map iteration order cannot flake a comparison.
fn normalised(frames: Vec<CollectedFrame>) -> Vec<CollectedFrame> {
    frames
        .into_iter()
        .map(|frame| match frame {
            CollectedFrame::Trailers(mut pairs) => {
                pairs.sort();
                CollectedFrame::Trailers(pairs)
            }
            data @ CollectedFrame::Data(_) => data,
        })
        .collect()
}

/// Builds an erased synthetic body with an optional drop probe.
fn synthetic_body(
    steps: Vec<FrameStep>,
    probe: Option<DropProbe>,
) -> (ErasedBody, Arc<AtomicUsize>) {
    let polls = Arc::new(AtomicUsize::new(0));
    let body: ErasedBody = adapt_body(SyntheticBody::new(steps, probe, Arc::clone(&polls)));
    (body, polls)
}

/// Builds an erased body with no scripted steps.
fn empty_body() -> ErasedBody {
    let (body, _polls) = synthetic_body(Vec::new(), None);
    body
}

#[test]
fn empty_input_yields_no_frames() {
    let runtime = current_thread_runtime();
    let mut body = empty_body();

    let frames = runtime
        .block_on(collect_frames(&mut body))
        .expect("an empty body must not fail");

    assert_eq!(
        frames,
        Vec::new(),
        "an empty body must emit no frames at all"
    );
}

#[test]
fn several_data_frames_preserve_boundaries() {
    let runtime = current_thread_runtime();
    let (mut body, _polls) = synthetic_body(
        vec![
            FrameStep::Data(Bytes::from_static(b"first")),
            FrameStep::Data(Bytes::new()),
            FrameStep::Data(Bytes::from_static(b"third")),
        ],
        None,
    );

    let frames = runtime
        .block_on(collect_frames(&mut body))
        .expect("data frames must not fail");

    let expected = vec![
        CollectedFrame::Data(Bytes::from_static(b"first")),
        CollectedFrame::Data(Bytes::new()),
        CollectedFrame::Data(Bytes::from_static(b"third")),
    ];
    assert_eq!(
        frames, expected,
        "the adapter must preserve every frame, including the empty one"
    );
}

#[test]
fn trailers_are_forwarded_verbatim() {
    let runtime = current_thread_runtime();
    let pairs = [("x-checksum", "abc123"), ("x-tenant", "acme")];
    let (mut body, _polls) = synthetic_body(
        vec![
            FrameStep::Data(Bytes::from_static(b"payload")),
            FrameStep::Trailers(headers_for(
                &pairs.map(|(name, value)| (name, value.to_owned())),
            )),
        ],
        None,
    );

    let frames = runtime
        .block_on(collect_frames(&mut body))
        .expect("a trailers frame must not fail");
    let trailers = frames
        .iter()
        .find_map(|frame| match frame {
            CollectedFrame::Trailers(found) => Some(found.clone()),
            CollectedFrame::Data(_) => None,
        })
        .expect("the trailers frame must survive adaptation");

    let mut expected = vec![
        ("x-checksum".to_owned(), b"abc123".to_vec()),
        ("x-tenant".to_owned(), b"acme".to_vec()),
    ];
    expected.sort();
    let mut actual = trailers;
    actual.sort();
    assert_eq!(
        actual, expected,
        "the adapter must forward every trailer name and value unchanged"
    );
}

#[test]
fn pending_then_ready_is_polled_again() {
    let (mut body, polls) = synthetic_body(
        vec![
            FrameStep::Pending,
            FrameStep::Data(Bytes::from_static(b"after-pending")),
        ],
        None,
    );

    let mut first = Box::pin(body.frame());
    assert!(
        poll_once(first.as_mut()).is_pending(),
        "a pending body must report pending before it yields a frame"
    );
    drop(first);

    let runtime = current_thread_runtime();
    let frames = runtime
        .block_on(collect_frames(&mut body))
        .expect("the resumed body must yield its frame");
    assert_eq!(
        frames,
        vec![CollectedFrame::Data(Bytes::from_static(b"after-pending"))],
        "resuming after a pending poll must not lose the next frame"
    );
    assert_eq!(
        polls.load(Ordering::SeqCst),
        3,
        "polls must cover the pending report, the resumed frame, and end of stream"
    );
}

#[test]
fn terminal_error_is_reported_once() {
    let runtime = current_thread_runtime();
    let (mut body, _polls) = synthetic_body(
        vec![
            FrameStep::Data(Bytes::from_static(b"partial")),
            FrameStep::Error("connection reset"),
        ],
        None,
    );

    let error = runtime
        .block_on(collect_frames(&mut body))
        .expect_err("a terminal error must surface instead of a clean end");

    match error {
        BodyError::Upstream { source } => assert_eq!(
            source.to_string(),
            "connection reset",
            "the adapter must retain the upstream error text"
        ),
    }
}

#[test]
fn dropping_a_partially_consumed_body_releases_it_once() {
    let runtime = current_thread_runtime();
    let (probe, drops) = drop_counter();
    let (mut body, _polls) = synthetic_body(
        vec![
            FrameStep::Data(Bytes::from_static(b"first")),
            FrameStep::Data(Bytes::from_static(b"second")),
        ],
        Some(probe),
    );

    let first = runtime
        .block_on(body.frame())
        .expect("the first frame must be available")
        .expect("the first frame must not fail");
    match first.into_data() {
        Ok(data) => assert_eq!(
            data,
            Bytes::from_static(b"first"),
            "the first frame must be the scripted data"
        ),
        Err(frame) => panic!("the first frame must be a data frame, got {frame:?}"),
    }
    assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "a partially consumed body must still own its resources"
    );

    drop(body);

    assert_eq!(
        drops.load(Ordering::SeqCst),
        1,
        "dropping the erased body must release its resources exactly once"
    );
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    #[test]
    fn generated_partitions_are_reproduced_frame_by_frame(case in frame_cases()) {
        let runtime = current_thread_runtime();
        let (mut body, _polls) = synthetic_body(steps_for(&case), None);
        let frames = runtime
            .block_on(collect_frames(&mut body))
            .expect("generated frames must not fail");

        prop_assert_eq!(
            normalised(frames.clone()),
            normalised(expected_frames(&case)),
            "frame sequence differed for {}",
            case.description(),
        );
        let actual_bytes: Vec<u8> = frames
            .into_iter()
            .filter_map(|frame| match frame {
                CollectedFrame::Data(bytes) => Some(bytes.to_vec()),
                CollectedFrame::Trailers(_) => None,
            })
            .flatten()
            .collect();
        let expected_bytes: Vec<u8> = case.chunks.iter().flatten().copied().collect();
        prop_assert_eq!(
            actual_bytes,
            expected_bytes,
            "concatenated bytes differed for {}",
            case.description(),
        );
    }
}
