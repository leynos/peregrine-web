//! The policy-admission hook and its read-only request view.
//!
//! Admission is asynchronous so the specimen can suspend between the policy
//! decision and the single resource invocation. The hook receives only the
//! selected registration and immutable request facts; it cannot reach another
//! resource through a locator.

use std::sync::Arc;

use async_trait::async_trait;

use crate::spike::{
    invoke::RegisteredResource,
    method::Method,
    policy::Policy,
    request::{AllocationId, RequestView, ResourceObservation},
    resource::Resource,
};

/// The admission result for a protected invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    /// The request may proceed to the resource.
    Permit,
    /// The request must not reach the resource.
    Deny(DenialReason),
}

impl Decision {
    /// Reports whether the decision permits the invocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::{Decision, DenialReason};
    ///
    /// assert!(Decision::Permit.is_permitted());
    /// assert!(!Decision::Deny(DenialReason::EnforcerDenied).is_permitted());
    /// ```
    #[must_use]
    pub const fn is_permitted(self) -> bool { matches!(self, Self::Permit) }
}

/// Why admission withheld a request from its resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DenialReason {
    /// The policy requires an identity and the request carried none.
    MissingIdentity,
    /// The configured enforcer denied the request.
    EnforcerDenied,
    /// The policy requires enforcement but no enforcer is configured.
    MissingEnforcer,
}

impl DenialReason {
    /// Returns the stable diagnostic spelling for this reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::DenialReason;
    ///
    /// assert_eq!(DenialReason::MissingIdentity.as_str(), "missing_identity");
    /// assert_eq!(DenialReason::EnforcerDenied.as_str(), "enforcer_denied");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingIdentity => "missing_identity",
            Self::EnforcerDenied => "enforcer_denied",
            Self::MissingEnforcer => "missing_enforcer",
        }
    }
}

/// The application-supplied policy hook consulted for protected resources.
#[async_trait]
pub trait PolicyEnforcer: Send + Sync {
    /// Decides whether one request may invoke its registered resource.
    async fn admit(&self, request: AdmissionRequest<'_>) -> Decision;
}

/// A read-only view of one registration presented to the policy enforcer.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use async_trait::async_trait;
/// use bytes::Bytes;
/// use http::HeaderMap;
/// use peregrine_web::spike::{
///     AdmissionRequest,
///     AllocationId,
///     ErasedBody,
///     Method,
///     Policy,
///     RequestView,
///     Resource,
///     ResourceError,
///     ResourceObservation,
///     ResourceRunner,
///     ResponseDraft,
/// };
///
/// struct Catalogue;
///
/// #[async_trait]
/// impl Resource for Catalogue {
///     fn policy(&self, _method: Method) -> &Policy { &Policy::Public }
///
///     fn observation(&self) -> ResourceObservation {
///         let state = Arc::new(());
///         ResourceObservation {
///             resource: AllocationId::of(&state),
///             dependency: AllocationId::of(&state),
///             label: "catalogue",
///         }
///     }
///
///     async fn get(
///         &self,
///         _view: RequestView<'_>,
///         _body: &mut ErasedBody,
///     ) -> Result<ResponseDraft, ResourceError> {
///         Ok(ResponseDraft::ok(Bytes::new()))
///     }
/// }
///
/// let runner = ResourceRunner::new();
/// let registered = runner
///     .register(Arc::new(Catalogue), Method::Get)
///     .expect("a public resource registers without an enforcer");
/// let headers = HeaderMap::new();
/// let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
/// let request = AdmissionRequest::new(&registered, view);
///
/// assert_eq!(request.method(), Method::Get);
/// assert!(request.policy().is_public());
/// assert_eq!(request.view().path(), "/catalogue");
/// assert_eq!(request.resource_observation().label, "catalogue");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct AdmissionRequest<'a> {
    /// The registration selected for this invocation.
    registered: &'a RegisteredResource,
    /// The immutable request metadata being admitted.
    view: RequestView<'a>,
}

impl<'a> AdmissionRequest<'a> {
    /// Creates an admission request for one registration and view.
    #[must_use]
    pub const fn new(registered: &'a RegisteredResource, view: RequestView<'a>) -> Self {
        Self { registered, view }
    }

    /// Returns the frozen method captured at registration.
    #[must_use]
    pub const fn method(self) -> Method { self.registered.method() }

    /// Returns the policy captured at registration.
    #[must_use]
    pub const fn policy(self) -> &'a Policy { self.registered.policy() }

    /// Returns the resource retained by registration.
    #[must_use]
    pub const fn resource(self) -> &'a Arc<dyn Resource> { self.registered.resource() }

    /// Returns the identity of the retained resource allocation.
    #[must_use]
    pub fn resource_handle(self) -> AllocationId { AllocationId::of(self.resource()) }

    /// Returns the identity observations declared by the retained resource.
    #[must_use]
    pub fn resource_observation(self) -> ResourceObservation {
        self.registered.resource().observation()
    }

    /// Returns the immutable request metadata.
    #[must_use]
    pub const fn view(self) -> RequestView<'a> { self.view }
}
