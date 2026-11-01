//! Immutable request facts, owned response drafts, and allocation identity.
//!
//! The view is deliberately `Copy` and borrows only `Sync` metadata. The
//! request body is passed separately so a `Send`-but-not-`Sync` body can cross
//! an await without forcing the metadata to carry the body's weaker bound.

use std::sync::Arc;

use bytes::Bytes;
use http::{HeaderMap, StatusCode};

use crate::spike::method::Method;

/// An authenticated principal supplied by the application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Identity(String);

impl Identity {
    /// Wraps an application-supplied principal name.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::Identity;
    ///
    /// let identity = Identity::new("user-42");
    /// assert_eq!(identity.principal(), "user-42");
    /// ```
    #[must_use]
    pub fn new(principal: impl Into<String>) -> Self { Self(principal.into()) }

    /// Returns the principal name.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::Identity;
    ///
    /// let identity = Identity::new("service:invoices");
    /// assert_eq!(identity.principal(), "service:invoices");
    /// ```
    #[must_use]
    pub fn principal(&self) -> &str { &self.0 }
}

/// Immutable HTTP metadata borrowed for one invocation.
#[derive(Clone, Copy, Debug)]
pub struct RequestView<'a> {
    /// The frozen effective method.
    method: Method,
    /// The request path.
    path: &'a str,
    /// The immutable request headers.
    headers: &'a HeaderMap,
    /// The authenticated identity, when the application supplied one.
    identity: Option<&'a Identity>,
}

impl<'a> RequestView<'a> {
    /// Creates an immutable request view.
    ///
    /// # Examples
    ///
    /// ```
    /// use http::HeaderMap;
    /// use peregrine_web::spike::{Method, RequestView};
    ///
    /// let headers = HeaderMap::new();
    /// let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
    /// assert_eq!(view.method(), Method::Get);
    /// assert_eq!(view.path(), "/catalogue");
    /// ```
    #[must_use]
    pub const fn new(
        method: Method,
        path: &'a str,
        headers: &'a HeaderMap,
        identity: Option<&'a Identity>,
    ) -> Self {
        Self {
            method,
            path,
            headers,
            identity,
        }
    }

    /// Returns the frozen method carried by the view.
    #[must_use]
    pub const fn method(self) -> Method { self.method }

    /// Returns the request path.
    #[must_use]
    pub const fn path(self) -> &'a str { self.path }

    /// Returns the immutable request headers.
    #[must_use]
    pub const fn headers(self) -> &'a HeaderMap { self.headers }

    /// Returns the authenticated identity when one was supplied.
    #[must_use]
    pub const fn identity(self) -> Option<&'a Identity> { self.identity }
}

/// An owned response produced by a resource responder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseDraft {
    /// The response status.
    status: StatusCode,
    /// The owned response body.
    body: Bytes,
}

impl ResponseDraft {
    /// Creates a response draft from a status and owned body.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytes::Bytes;
    /// use http::StatusCode;
    /// use peregrine_web::spike::ResponseDraft;
    ///
    /// let draft = ResponseDraft::new(StatusCode::CREATED, Bytes::from_static(b"ok"));
    /// assert_eq!(draft.status(), StatusCode::CREATED);
    /// assert_eq!(draft.body(), &Bytes::from_static(b"ok"));
    /// ```
    #[must_use]
    pub const fn new(status: StatusCode, body: Bytes) -> Self { Self { status, body } }

    /// Creates a `200 OK` response draft.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytes::Bytes;
    /// use http::StatusCode;
    /// use peregrine_web::spike::ResponseDraft;
    ///
    /// let draft = ResponseDraft::ok(Bytes::from_static(b"ok"));
    /// assert_eq!(draft.status(), StatusCode::OK);
    /// ```
    #[must_use]
    pub const fn ok(body: Bytes) -> Self { Self::new(StatusCode::OK, body) }

    /// Returns the response status.
    #[must_use]
    pub const fn status(&self) -> StatusCode { self.status }

    /// Returns the owned response body.
    #[must_use]
    pub const fn body(&self) -> &Bytes { &self.body }

    /// Splits the draft into its status and body.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytes::Bytes;
    /// use http::StatusCode;
    /// use peregrine_web::spike::ResponseDraft;
    ///
    /// let draft = ResponseDraft::ok(Bytes::from_static(b"ok"));
    /// let (status, body) = draft.into_parts();
    /// assert_eq!(status, StatusCode::OK);
    /// assert_eq!(body, Bytes::from_static(b"ok"));
    /// ```
    #[must_use]
    pub fn into_parts(self) -> (StatusCode, Bytes) { (self.status, self.body) }
}

/// An opaque identity for one retained allocation.
///
/// The specimen uses allocation identity to witness that the resource retained
/// by registration is the object observed before and after an await.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocationId(usize);

impl AllocationId {
    /// Derives an identity from a retained [`Arc`] allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// use peregrine_web::spike::AllocationId;
    ///
    /// let value = Arc::new(7_u8);
    /// let same = Arc::clone(&value);
    /// assert_eq!(AllocationId::of(&value), AllocationId::of(&same));
    /// assert_ne!(AllocationId::of(&value), AllocationId::of(&Arc::new(7_u8)));
    /// ```
    #[must_use]
    pub fn of<T: ?Sized>(allocation: &Arc<T>) -> Self {
        Self(Arc::as_ptr(allocation).cast::<()>() as usize)
    }
}

/// An observation drawn from a resource's own state and injected dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceObservation {
    /// Identity of the resource's own retained state allocation.
    pub resource: AllocationId,
    /// Identity of the resource's injected dependency allocation.
    pub dependency: AllocationId,
    /// A label owned by the resource instance.
    pub label: &'static str,
}
