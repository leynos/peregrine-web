//! Experimental request-body erasure shared by the spike's resources.
//!
//! The adapter erases a concrete body exactly once and keeps the result
//! `Send`-but-not-`Sync`. That bound is the interesting one: a request body may
//! cross an await inside a `Send` future, yet it must not be shared between
//! threads. The specimen uses [`UnsyncBoxBody`] to preserve that asymmetry
//! while giving Hyper one uniform response type.

use std::error::Error;

use bytes::Bytes;
use http_body::Body;
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};

/// A semantic failure while reading or forwarding a body.
#[derive(Debug, thiserror::Error)]
pub enum BodyError {
    /// The wrapped body reported its own terminal error.
    #[error("request body failed upstream")]
    Upstream {
        /// The original body error, retained for diagnostics.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
}

impl BodyError {
    /// Wraps an upstream body error without discarding its source.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io;
    ///
    /// use peregrine_web::spike::BodyError;
    ///
    /// let error = BodyError::upstream(io::Error::other("broken pipe"));
    /// assert_eq!(error.to_string(), "request body failed upstream");
    /// ```
    #[must_use]
    pub fn upstream<E>(source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::Upstream {
            source: Box::new(source),
        }
    }
}

/// The single erased body type carried across the spike boundary.
///
/// Using the unsynchronized box keeps `Send` while deliberately withholding
/// `Sync`, matching Hyper's incoming-body posture.
pub type ErasedBody = UnsyncBoxBody<Bytes, BodyError>;

/// Erases a concrete body once, tagging its errors as [`BodyError`].
///
/// The input must be `Send` so the erased body can travel inside a `Send`
/// future. It need not be `Sync`, and the returned type is not `Sync`.
///
/// # Examples
///
/// ```
/// use bytes::Bytes;
/// use http_body_util::Full;
/// use peregrine_web::spike::{ErasedBody, adapt_body};
///
/// let body: ErasedBody = adapt_body(Full::new(Bytes::from_static(b"payload")));
/// let _ = body;
/// ```
pub fn adapt_body<B>(body: B) -> ErasedBody
where
    B: Body<Data = Bytes> + Send + 'static,
    B::Error: Error + Send + Sync + 'static,
{
    UnsyncBoxBody::new(body.map_err(BodyError::upstream))
}
