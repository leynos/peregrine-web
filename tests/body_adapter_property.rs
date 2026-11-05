//! Generated frame-partition evidence for the erased body adapter.
//!
//! The case generator lives in `tests/support/spike/cases.rs`, so this property
//! and the named witnesses in `body_adapter_spike` describe the same frame
//! classes. Each generated case is compared frame by frame and by concatenated
//! bytes, because a byte-only oracle cannot see a dropped empty frame.

#[path = "support/spike/cases.rs"]
mod spike_cases;
#[path = "support/spike/frames.rs"]
mod spike_frames;
#[path = "support/spike/synthetic.rs"]
mod spike_synthetic;

use proptest::prelude::*;
use spike_cases::{
    current_thread_runtime,
    expected_frames,
    frame_cases,
    normalised,
    steps_for,
    synthetic_body,
};
use spike_frames::{CollectedFrame, collect_frames};

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// Checks that generated frame partitions survive adaptation frame by frame.
    ///
    /// Each proptest case compares the normalised frame sequence and the
    /// concatenated data bytes, so a dropped empty frame or a moved boundary is
    /// still visible even when the total payload matches.
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
