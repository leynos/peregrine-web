//! Adapter fidelity and ownership evidence for erased request bodies.
//!
//! The synthetic body is `Send` but deliberately not `Sync`, so every test that
//! compiles here also witnesses the bound the spike claims. Frames are compared
//! frame by frame, including empty data frames, because a concatenated-bytes
//! oracle cannot see a dropped empty frame or a changed frame boundary. The
//! generated partition property lives in `body_adapter_property`; both binaries
//! build their cases from `tests/support/spike/cases.rs`.

#[path = "support/spike/cases.rs"]
mod spike_cases;
#[path = "support/spike/frames.rs"]
mod spike_frames;
#[path = "support/spike/synthetic.rs"]
mod spike_synthetic;

use std::{sync::atomic::Ordering, task::Poll};

use bytes::Bytes;
use http_body_util::BodyExt;
use peregrine_web::spike::BodyError;
use spike_cases::{
    FrameCase,
    current_thread_runtime,
    empty_body,
    expected_frames,
    headers_for,
    normalised,
    steps_for,
    synthetic_body,
};
use spike_frames::{CollectedFrame, collect_frames};
use spike_synthetic::{FrameStep, MAX_POLLS, drop_counter, poll_once};

/// Builds the four documented frame classes as concrete cases.
///
/// The classes cross body emptiness with trailer presence, so a witness exists
/// for each combination rather than only for the generated mixture.
fn frame_classes() -> Vec<(&'static str, FrameCase)> {
    let trailers = || Some(vec![("x-checksum", "abc123".to_owned())]);
    vec![
        (
            "empty body without trailers",
            FrameCase {
                chunks: Vec::new(),
                trailers: None,
            },
        ),
        (
            "empty body with trailers",
            FrameCase {
                chunks: Vec::new(),
                trailers: trailers(),
            },
        ),
        (
            "non-empty body without trailers",
            FrameCase {
                chunks: vec![b"payload".to_vec()],
                trailers: None,
            },
        ),
        (
            "non-empty body with trailers",
            FrameCase {
                chunks: vec![b"payload".to_vec()],
                trailers: trailers(),
            },
        ),
    ]
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

#[test]
fn every_documented_frame_class_is_reproduced() {
    let runtime = current_thread_runtime();
    for (class, case) in frame_classes() {
        let (mut body, _polls) = synthetic_body(steps_for(&case), None);
        let frames = runtime
            .block_on(collect_frames(&mut body))
            .expect("a documented frame class must not fail");

        assert_eq!(
            normalised(frames),
            normalised(expected_frames(&case)),
            "the {class} class must be reproduced frame by frame"
        );
    }
}

#[test]
fn empty_body_with_trailers_yields_only_the_trailers_frame() {
    let case = FrameCase {
        chunks: Vec::new(),
        trailers: Some(vec![("x-checksum", "abc123".to_owned())]),
    };
    let runtime = current_thread_runtime();
    let (mut body, _polls) = synthetic_body(steps_for(&case), None);

    let frames = runtime
        .block_on(collect_frames(&mut body))
        .expect("an empty body with trailers must not fail");

    assert_eq!(
        frames.len(),
        1,
        "an empty body with trailers must yield exactly one frame, got {frames:?}"
    );
    match frames.as_slice() {
        [CollectedFrame::Trailers(pairs)] => {
            let expected = vec![("x-checksum".to_owned(), b"abc123".to_vec())];
            assert_eq!(
                pairs, &expected,
                "the only frame must carry the scripted trailer unchanged"
            );
        }
        other => panic!("the only frame must be a trailers frame, got {other:?}"),
    }
}

#[test]
fn synthetic_poll_bound_is_enforced() {
    let steps = (0..MAX_POLLS).map(|_| FrameStep::Pending).collect();
    let (mut body, polls) = synthetic_body(steps, None);

    for index in 0..MAX_POLLS {
        let mut frame = Box::pin(body.frame());
        assert!(
            poll_once(frame.as_mut()).is_pending(),
            "poll {index} is within the synthetic bound and must stay pending"
        );
    }

    let mut beyond = Box::pin(body.frame());
    match poll_once(beyond.as_mut()) {
        Poll::Ready(Some(Err(BodyError::Upstream { source }))) => assert_eq!(
            source.to_string(),
            "synthetic body exceeded its poll bound",
            "the poll after the bound must report the fixture's own error"
        ),
        other => panic!("the poll after the bound must fail, got {other:?}"),
    }
    assert_eq!(
        polls.load(Ordering::SeqCst),
        MAX_POLLS,
        "the observed poll count must stop at the bound"
    );
}
