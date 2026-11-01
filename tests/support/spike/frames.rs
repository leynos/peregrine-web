//! Frame-level helpers shared by the spike's body fixtures and assertions.
//!
//! Both the synthetic and the loopback fixtures need to describe what a body
//! actually yielded, and the responder needs to consume a request body without
//! losing its frame boundaries. Keeping the conversion in one module means the
//! tests and the fixture agree on what "the same frames" means.
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

use bytes::Bytes;
use http::HeaderMap;
use http_body_util::BodyExt;
use peregrine_web::spike::{BodyError, ErasedBody, ResourceError};

/// What one request body yielded: its data bytes and its trailer pairs.
#[derive(Debug, Default)]
pub struct DrainedBody {
    /// The concatenated data frames.
    pub data: Vec<u8>,
    /// The trailer pairs as they arrived.
    pub trailers: Vec<(String, Vec<u8>)>,
}

/// Drains an erased body, stopping at its terminal error.
pub async fn drain_body(body: &mut ErasedBody) -> Result<DrainedBody, ResourceError> {
    let mut drained = DrainedBody::default();
    while let Some(next) = body.frame().await {
        let frame = next.map_err(|source| ResourceError::Body {
            source: Box::new(source),
        })?;
        match frame.into_trailers() {
            Ok(headers) => drained.trailers.extend(normalise_trailers(&headers)),
            Err(rest) => {
                if let Ok(data) = rest.into_data() {
                    drained.data.extend_from_slice(&data);
                }
            }
        }
    }
    Ok(drained)
}

/// One collected body frame in a comparable form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollectedFrame {
    /// A data frame and its bytes.
    Data(Bytes),
    /// A trailers frame normalised to ordered name/value pairs.
    Trailers(Vec<(String, Vec<u8>)>),
}

/// Collects every frame from an erased body, preserving frame boundaries.
pub async fn collect_frames(body: &mut ErasedBody) -> Result<Vec<CollectedFrame>, BodyError> {
    let mut frames = Vec::new();
    while let Some(next) = body.frame().await {
        let frame = next?;
        match frame.into_trailers() {
            Ok(headers) => frames.push(CollectedFrame::Trailers(normalise_trailers(&headers))),
            Err(rest) => {
                if let Ok(data) = rest.into_data() {
                    frames.push(CollectedFrame::Data(data));
                }
            }
        }
    }
    Ok(frames)
}

/// Converts trailers into an ordered, comparable representation.
pub fn normalise_trailers(headers: &HeaderMap) -> Vec<(String, Vec<u8>)> {
    headers
        .iter()
        .map(|(name, value)| (name.as_str().to_owned(), value.as_bytes().to_vec()))
        .collect()
}
