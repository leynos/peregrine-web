//! Experimental registration and one conditional invocation.
//!
//! Registration selects one resource and freezes the effective method and
//! policy. Invocation then makes at most one admission decision and, on
//! success, calls the resource responder exactly once. The specimen exists to
//! prove that this shape is expressible without route policy tables, service
//! locators, or shared mutable application state.

use std::{fmt, sync::Arc};

use crate::spike::{
    body::ErasedBody,
    method::Method,
    policy::Policy,
    policy_enforcer::{AdmissionRequest, Decision, DenialReason, PolicyEnforcer},
    request::{RequestView, ResponseDraft},
    resource::{Resource, ResourceError},
};

/// An error raised while registering a resource.
#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    /// A protected resource cannot be registered without an enforcer.
    #[error("method {method:?} requires a policy enforcer")]
    MissingEnforcer {
        /// The frozen method whose policy requires enforcement.
        method: Method,
        /// The policy captured during the failed registration.
        policy: Policy,
    },
}

/// One resource selected for one frozen method, with its policy captured.
///
/// Values are produced by [`ResourceRunner::register`]; its fields stay private
/// so registration cannot bypass policy validation.
pub struct RegisteredResource {
    /// The shared resource retained for invocation.
    resource: Arc<dyn Resource>,
    /// The method frozen when the resource was selected.
    method: Method,
    /// The policy captured once, at registration.
    policy: Policy,
}

impl RegisteredResource {
    /// Returns the frozen method.
    #[must_use]
    pub const fn method(&self) -> Method { self.method }

    /// Returns the policy captured at registration.
    #[must_use]
    pub const fn policy(&self) -> &Policy { &self.policy }

    /// Returns the retained resource.
    #[must_use]
    pub const fn resource(&self) -> &Arc<dyn Resource> { &self.resource }
}

/// The retained resource is erased and need not implement [`fmt::Debug`], so
/// only the frozen registration decisions are rendered.
impl fmt::Debug for RegisteredResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredResource")
            .field("method", &self.method)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

/// What one invocation did.
#[derive(Debug)]
pub enum InvocationOutcome {
    /// The resource produced a response draft.
    Responded(ResponseDraft),
    /// Admission withheld the request.
    Denied(DenialReason),
    /// The frozen method has no responder in this specimen.
    MethodNotAllowed(Method),
    /// The request method does not match the frozen registration method.
    MethodMismatch {
        /// The method frozen at registration.
        registered: Method,
        /// The method carried by the request.
        requested: Method,
    },
    /// The resource reported a failure while handling the request.
    Failed(ResourceError),
}

impl InvocationOutcome {
    /// Returns the resource's response draft, if one was produced.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytes::Bytes;
    /// use peregrine_web::spike::{InvocationOutcome, ResponseDraft};
    ///
    /// let outcome = InvocationOutcome::Responded(ResponseDraft::ok(Bytes::new()));
    /// assert!(outcome.responded().is_some());
    /// ```
    #[must_use]
    pub const fn responded(&self) -> Option<&ResponseDraft> {
        match self {
            Self::Responded(draft) => Some(draft),
            _ => None,
        }
    }

    /// Returns the admission denial, if the request was withheld.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::{DenialReason, InvocationOutcome};
    ///
    /// let outcome = InvocationOutcome::Denied(DenialReason::EnforcerDenied);
    /// assert_eq!(outcome.denied(), Some(DenialReason::EnforcerDenied));
    /// ```
    #[must_use]
    pub const fn denied(&self) -> Option<DenialReason> {
        match self {
            Self::Denied(reason) => Some(*reason),
            _ => None,
        }
    }
}

/// Registers resources and performs one conditional invocation each.
#[derive(Default)]
pub struct ResourceRunner {
    /// The optional application-supplied policy hook.
    enforcer: Option<Arc<dyn PolicyEnforcer>>,
}

impl ResourceRunner {
    /// Creates a runner with no policy enforcer.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::ResourceRunner;
    ///
    /// let runner = ResourceRunner::new();
    /// let _ = runner;
    /// ```
    #[must_use]
    pub const fn new() -> Self { Self { enforcer: None } }

    /// Creates a runner with an application-supplied policy enforcer.
    ///
    /// See the [`PolicyEnforcer`] trait for how an application supplies the
    /// hook; the spike deliberately ships no default implementation.
    #[must_use]
    pub fn with_enforcer(enforcer: Arc<dyn PolicyEnforcer>) -> Self {
        Self {
            enforcer: Some(enforcer),
        }
    }

    /// Selects a resource for a frozen method and captures its policy.
    ///
    /// # Errors
    ///
    /// Returns [`RegistrationError::MissingEnforcer`] when the captured policy
    /// requires enforcement and this runner has no enforcer.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
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
    ///     ResourceRunner,
    ///     ResponseDraft,
    /// };
    ///
    /// struct Catalogue;
    ///
    /// #[async_trait]
    /// impl Resource for Catalogue {
    ///     fn policy(&self, _method: Method) -> &Policy { &Policy::Public }
    ///     fn observation(&self) -> ResourceObservation { unimplemented!("witnessed in tests") }
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
    /// assert_eq!(registered.method(), Method::Get);
    /// assert!(registered.policy().is_public());
    /// ```
    pub fn register(
        &self,
        resource: Arc<dyn Resource>,
        method: Method,
    ) -> Result<RegisteredResource, RegistrationError> {
        let policy = resource.policy(method).clone();
        if policy.requires_enforcement() && self.enforcer.is_none() {
            return Err(RegistrationError::MissingEnforcer { method, policy });
        }
        Ok(RegisteredResource {
            resource,
            method,
            policy,
        })
    }

    /// Admits and, on success, invokes the registered resource exactly once.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use async_trait::async_trait;
    /// # use bytes::Bytes;
    /// # use http::HeaderMap;
    /// # use http_body_util::Empty;
    /// # use peregrine_web::spike::{
    /// #     adapt_body, ErasedBody, InvocationOutcome, Method, Policy, RequestView, Resource,
    /// #     ResourceError, ResourceObservation, ResourceRunner, ResponseDraft,
    /// # };
    /// # struct Catalogue;
    /// # #[async_trait]
    /// # impl Resource for Catalogue {
    /// #     fn policy(&self, _method: Method) -> &Policy { &Policy::Public }
    /// #     fn observation(&self) -> ResourceObservation {
    /// #         unimplemented!("witnessed in tests")
    /// #     }
    /// #     async fn get(&self, _view: RequestView<'_>, _body: &mut ErasedBody)
    /// #         -> Result<ResponseDraft, ResourceError> {
    /// #         Ok(ResponseDraft::ok(Bytes::from_static(b"catalogue")))
    /// #     }
    /// # }
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let runner = ResourceRunner::new();
    /// let registered = runner.register(Arc::new(Catalogue), Method::Get)?;
    /// let headers = HeaderMap::new();
    /// let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
    /// let mut body = adapt_body(Empty::<Bytes>::new());
    /// let outcome = runner.invoke(&registered, view, &mut body).await;
    /// assert!(matches!(outcome, InvocationOutcome::Responded(_)));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invoke(
        &self,
        registered: &RegisteredResource,
        view: RequestView<'_>,
        body: &mut ErasedBody,
    ) -> InvocationOutcome {
        if registered.method != view.method() {
            return InvocationOutcome::MethodMismatch {
                registered: registered.method,
                requested: view.method(),
            };
        }
        if registered.method != Method::Get {
            return InvocationOutcome::MethodNotAllowed(registered.method);
        }
        match self.admit(registered, view).await {
            Decision::Deny(reason) => InvocationOutcome::Denied(reason),
            Decision::Permit => match registered.resource.get(view, body).await {
                Ok(draft) => InvocationOutcome::Responded(draft),
                Err(error) => InvocationOutcome::Failed(error),
            },
        }
    }

    /// Computes the admission decision for a protected registration.
    ///
    /// Public registrations never reach the enforcer. Protected ones first
    /// require a presented identity, then delegate to the configured hook.
    async fn admit(&self, registered: &RegisteredResource, view: RequestView<'_>) -> Decision {
        if registered.policy.is_public() {
            return Decision::Permit;
        }
        if view.identity().is_none() {
            return Decision::Deny(DenialReason::MissingIdentity);
        }
        match &self.enforcer {
            Some(enforcer) => {
                enforcer
                    .admit(AdmissionRequest::new(registered, view))
                    .await
            }
            None => Decision::Deny(DenialReason::MissingEnforcer),
        }
    }
}
