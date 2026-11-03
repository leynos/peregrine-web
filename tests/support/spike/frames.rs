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
use http_body::Frame;
use http_body_util::BodyExt;
use peregrine_web::spike::{BodyError, ErasedBody, ResourceError};

/// Maximum accumulated request-body bytes a fixture drain accepts.
pub const MAX_BODY_BYTES: usize = 4096;

/// Maximum trailer pairs a fixture drain accepts in one body.
pub const MAX_TRAILER_PAIRS: usize = 4;

/// What one request body yielded: its data bytes and its trailer pairs.
#[derive(Debug, Default)]
pub struct DrainedBody {
    /// The concatenated data frames.
    pub data: Vec<u8>,
    /// The trailer pairs as they arrived.
    pub trailers: Vec<(String, Vec<u8>)>,
}

/// Drains an erased body, stopping at its terminal error or its first bound.
///
/// The bounds are checked before each extension, so an over-long body fails
/// without first allocating everything the peer sent. A deadline bounds time,
/// not memory.
pub async fn drain_body(body: &mut ErasedBody) -> Result<DrainedBody, ResourceError> {
    let mut drained = DrainedBody::default();
    while let Some(next) = body.frame().await {
        let frame = next.map_err(|source| ResourceError::Body {
            source: Box::new(source),
        })?;
        drained.record(frame)?;
    }
    Ok(drained)
}

impl DrainedBody {
    /// Records one frame, enforcing the matching fixture bound first.
    ///
    /// Splitting the two frame shapes keeps each bound check at the same
    /// nesting level, so the control flow stays readable as further bounds are
    /// added.
    fn record(&mut self, frame: Frame<Bytes>) -> Result<(), ResourceError> {
        match frame.into_trailers() {
            Ok(headers) => self.record_trailers(&headers),
            Err(rest) => self.record_data(rest),
        }
    }

    /// Appends one trailer frame after checking the trailer-pair bound.
    fn record_trailers(&mut self, headers: &HeaderMap) -> Result<(), ResourceError> {
        let pairs = normalise_trailers(headers);
        if self.trailers.len() + pairs.len() > MAX_TRAILER_PAIRS {
            return Err(bound_exceeded("trailer pairs", MAX_TRAILER_PAIRS));
        }
        self.trailers.extend(pairs);
        Ok(())
    }

    /// Appends one data frame after checking the accumulated-byte bound.
    fn record_data(&mut self, frame: Frame<Bytes>) -> Result<(), ResourceError> {
        let Ok(data) = frame.into_data() else {
            return Ok(());
        };
        if self.data.len() + data.len() > MAX_BODY_BYTES {
            return Err(bound_exceeded("body bytes", MAX_BODY_BYTES));
        }
        self.data.extend_from_slice(&data);
        Ok(())
    }
}

/// Builds the fixture failure for an exceeded body bound.
fn bound_exceeded(what: &str, limit: usize) -> ResourceError {
    ResourceError::Failed {
        message: format!("request {what} exceeded the fixture bound of {limit}"),
    }
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
