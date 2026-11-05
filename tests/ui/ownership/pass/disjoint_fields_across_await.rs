//! A `Send` invocation future borrows disjoint context fields across an await.

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use http::HeaderMap;
use http_body_util::Empty;
use peregrine_web::spike::{
    ErasedBody,
    Method,
    Policy,
    RegisteredResource,
    RequestView,
    Resource,
    ResourceError,
    ResourceObservation,
    ResourceRunner,
    ResponseDraft,
    adapt_body,
};

/// A public resource that never touches the request body.
struct Catalogue;

#[async_trait]
impl Resource for Catalogue {
    /// Declares the resource public so registration needs no enforcer.
    fn policy(&self, _method: Method) -> &Policy { &Policy::Public }

    /// Reports allocation identities; this witness never queries them.
    fn observation(&self) -> ResourceObservation { unimplemented!("witnessed in tests") }

    /// Returns an empty response; the witness is about the future's borrows.
    async fn get(
        &self,
        _view: RequestView<'_>,
        _body: &mut ErasedBody,
    ) -> Result<ResponseDraft, ResourceError> {
        Ok(ResponseDraft::ok(Bytes::new()))
    }
}

/// Request state a handler borrows field by field.
struct Context {
    /// Immutable request metadata, borrowed by the view.
    headers: HeaderMap,
    /// The mutable request body, borrowed separately from the metadata.
    body: ErasedBody,
    /// The registration selected for this request.
    registered: RegisteredResource,
    /// The runner that owns admission.
    runner: ResourceRunner,
}

/// Asserts at compile time that a value is `Send`.
fn assert_send<T: Send>(_value: &T) {}

/// Proves the invocation future is `Send` while borrowing disjoint fields.
fn main() {
    let runner = ResourceRunner::new();
    let registered = runner
        .register(Arc::new(Catalogue), Method::Get)
        .expect("a public resource registers without an enforcer");
    let mut context = Context {
        headers: HeaderMap::new(),
        body: adapt_body(Empty::<Bytes>::new()),
        registered,
        runner,
    };

    let Context {
        headers,
        body,
        registered,
        runner,
    } = &mut context;
    let view = RequestView::new(Method::Get, "/catalogue", headers, None);
    let future = runner.invoke(registered, view, body);
    assert_send(&future);
}
