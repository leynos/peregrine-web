//! Deterministic and generated body cases shared by the adapter suites.
//!
//! The named witnesses and the generated property must describe the same frame
//! classes, so the case construction lives here and both binaries include it.
//! A case is a list of data chunks plus optional trailers; the helpers turn it
//! into scripted steps and into the frame sequence the adapter must reproduce.
//!
//! # Lint scope
//!
//! Several integration binaries include this module through `#[path]`, and each
//! compiles its own copy, so an item that only a sibling binary uses looks
//! unused here. The allowance is confined to this fixture module rather than
//! the whole test crate; a module with a single consumer carries none.
//! Splitting the fixtures per consumer would duplicate them, and per-item
//! allowances would scatter the same suppression through ordinary fixture code.

#![allow(
    dead_code,
    reason = "each integration binary includes a different subset of these fixtures"
)]

use std::sync::{Arc, atomic::AtomicUsize};

use bytes::Bytes;
use http::{
    HeaderMap,
    header::{HeaderName, HeaderValue},
};
use peregrine_web::spike::{ErasedBody, adapt_body};
use proptest::prelude::*;

use crate::{
    spike_frames::{CollectedFrame, normalize_trailers},
    spike_synthetic::{DropProbe, FrameStep, SyntheticBody},
};

/// Trailer names that are always syntactically valid.
const TRAILER_NAMES: [&str; 3] = ["x-checksum", "x-tenant", "x-trace"];

/// Builds a current-thread runtime for draining bodies in synchronous tests.
#[cfg(test)]
pub fn current_thread_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a current-thread runtime should build")
}

/// A frame partition with optional trailers.
#[derive(Clone, Debug)]
pub struct FrameCase {
    /// The data chunks in emission order, including deliberately empty ones.
    pub chunks: Vec<Vec<u8>>,
    /// Trailer name/value pairs, or `None` when the case carries no trailers.
    pub trailers: Option<Vec<(&'static str, String)>>,
}

impl FrameCase {
    /// Returns a stable description used in assertion messages.
    pub fn description(&self) -> String {
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
pub fn frame_cases() -> impl Strategy<Value = FrameCase> {
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

/// Builds the scripted steps for one case.
pub fn steps_for(case: &FrameCase) -> Vec<FrameStep> {
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
pub fn headers_for(pairs: &[(&'static str, String)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs {
        if let Ok(parsed) = HeaderValue::from_str(value) {
            headers.append(HeaderName::from_static(name), parsed);
        }
    }
    headers
}

/// Builds the frame sequence a case must reproduce, independently of the body.
pub fn expected_frames(case: &FrameCase) -> Vec<CollectedFrame> {
    let mut frames: Vec<CollectedFrame> = case
        .chunks
        .iter()
        .cloned()
        .map(|chunk| CollectedFrame::Data(Bytes::from(chunk)))
        .collect();
    if let Some(pairs) = case.trailers.as_ref() {
        frames.push(CollectedFrame::Trailers(normalize_trailers(&headers_for(
            pairs,
        ))));
    }
    frames
}

/// Sorts trailer pairs so header-map iteration order cannot flake a comparison.
pub fn normalised(frames: Vec<CollectedFrame>) -> Vec<CollectedFrame> {
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
pub fn synthetic_body(
    steps: Vec<FrameStep>,
    probe: Option<DropProbe>,
) -> (ErasedBody, Arc<AtomicUsize>) {
    let polls = Arc::new(AtomicUsize::new(0));
    let body: ErasedBody = adapt_body(SyntheticBody::new(steps, probe, Arc::clone(&polls)));
    (body, polls)
}

/// Builds an erased body with no scripted steps.
pub fn empty_body() -> ErasedBody {
    let (body, _polls) = synthetic_body(Vec::new(), None);
    body
}
