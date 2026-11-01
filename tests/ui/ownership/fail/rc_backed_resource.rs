//! A resource backed by `Rc` cannot satisfy the `Send + Sync` requirement.

use std::rc::Rc;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use peregrine_web::spike::{
    AllocationId, ErasedBody, Method, Policy, RequestView, Resource, ResourceError,
    ResourceObservation, ResponseDraft,
};

/// A resource that shares its state with `Rc`.
struct Catalogue {
    /// The policy this resource owns.
    policy: Policy,
    /// Shared state that is neither `Send` nor `Sync`.
    state: Rc<String>,
    /// A `Send` allocation used only to build observations.
    handle: Arc<()>,
}

#[async_trait]
impl Resource for Catalogue {
    fn policy(&self, _method: Method) -> &Policy { &self.policy }

    fn observation(&self) -> ResourceObservation {
        ResourceObservation {
            resource: AllocationId::of(&self.handle),
            dependency: AllocationId::of(&self.handle),
            label: "catalogue",
        }
    }

    async fn get(
        &self,
        _view: RequestView<'_>,
        _body: &mut ErasedBody,
    ) -> Result<ResponseDraft, ResourceError> {
        Ok(ResponseDraft::ok(Bytes::from(self.state.as_str())))
    }
}

fn main() {}