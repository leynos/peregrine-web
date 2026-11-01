//! The dyn-compatible resource contract exercised by the spike.
//!
//! A resource owns its policy and injects its own dependencies. The responder
//! future is `Send` and borrows immutable request metadata and a mutable body
//! separately, which is the ownership shape the spike is testing.

use async_trait::async_trait;

use crate::spike::{
    body::{BodyError, ErasedBody},
    method::Method,
    policy::Policy,
    request::{RequestView, ResourceObservation, ResponseDraft},
};

/// A failure returned by a resource responder.
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    /// The frozen method has no responder in this specimen.
    #[error("method {method:?} is not supported by this resource")]
    MethodNotAllowed {
        /// The unsupported frozen method.
        method: Method,
    },
    /// Reading the request body failed.
    #[error("resource body failed")]
    Body {
        /// The semantic body failure.
        #[source]
        source: Box<BodyError>,
    },
    /// The resource reported an application failure.
    #[error("resource failed: {message}")]
    Failed {
        /// A bounded diagnostic message.
        message: String,
    },
}

/// A dyn-compatible resource that owns its policy and `GET` behaviour.
///
/// Implementations are shared `Send + Sync` objects. The responder future is
/// `Send`; it borrows the immutable request view and the mutable erased body
/// separately so the body does not need to be `Sync`.
///
/// # Examples
///
/// ```
/// use async_trait::async_trait;
/// use bytes::Bytes;
/// use peregrine_web::spike::{
///     ErasedBody,
///     Method,
///     Policy,
///     RequestView,
///     Resource,
///     ResourceError,
///     ResourceObservation,
///     ResponseDraft,
/// };
///
/// struct Greeting;
///
/// #[async_trait]
/// impl Resource for Greeting {
///     fn policy(&self, _method: Method) -> &Policy { &Policy::Public }
///
///     fn observation(&self) -> ResourceObservation { unimplemented!("witnessed in tests") }
///
///     async fn get(
///         &self,
///         _view: RequestView<'_>,
///         _body: &mut ErasedBody,
///     ) -> Result<ResponseDraft, ResourceError> {
///         Ok(ResponseDraft::ok(Bytes::from_static(b"hello")))
///     }
/// }
///
/// let boxed: std::sync::Arc<dyn Resource> = std::sync::Arc::new(Greeting);
/// assert!(boxed.policy(Method::Get).is_public());
/// ```
#[async_trait]
pub trait Resource: Send + Sync {
    /// Returns the immutable policy for the frozen method.
    ///
    /// Resources must keep the returned policy stable after construction. The
    /// specimen cannot enforce that contract for a dishonest implementation.
    fn policy(&self, method: Method) -> &Policy;

    /// Returns identity observations taken from this resource's own state.
    fn observation(&self) -> ResourceObservation;

    /// Handles a frozen `GET` request.
    ///
    /// # Errors
    ///
    /// Returns a [`ResourceError`] when the body cannot be consumed or the
    /// resource application rejects the request.
    async fn get(
        &self,
        view: RequestView<'_>,
        body: &mut ErasedBody,
    ) -> Result<ResponseDraft, ResourceError>;
}
