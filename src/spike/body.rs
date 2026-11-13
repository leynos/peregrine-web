//! Experimental request-body erasure shared by the spike's resources.
//!
//! Hyper hands the specimen a concrete inbound request body. [`adapt_body`]
//! erases that body exactly once into an [`ErasedBody`], and
//! [`crate::spike::ResourceRunner::invoke`] passes it to the selected resource
//! as `&mut ErasedBody`, so a resource never names Hyper's own type. The erased
//! type stays `Send`-but-not-`Sync`: a request body may cross an await inside a
//! `Send` future, yet it must not be shared between threads. The specimen uses
//! [`UnsyncBoxBody`] to preserve that asymmetry. Response bodies are not erased
//! here; a resource returns a `ResponseDraft` and the responder builds whatever
//! body type it needs.

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
