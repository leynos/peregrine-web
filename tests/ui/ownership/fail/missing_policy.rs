//! A resource implementation must supply its policy.

use async_trait::async_trait;
use bytes::Bytes;
use peregrine_web::spike::{
    AllocationId, ErasedBody, Method, Policy, RequestView, Resource, ResourceError,
    ResourceObservation, ResponseDraft,
};

/// A resource that omits the required policy method.
struct Catalogue {
    /// An unused policy field does not satisfy the trait method.
    policy: Policy,
    /// State owned by this resource instance.
    state: std::sync::Arc<()>,
}

#[async_trait]
impl Resource for Catalogue {
    fn observation(&self) -> ResourceObservation {
        ResourceObservation {
            resource: AllocationId::of(&self.state),
            dependency: AllocationId::of(&self.state),
            label: "catalogue",
        }
    }

    async fn get(
        &self,
        _view: RequestView<'_>,
        _body: &mut ErasedBody,
    ) -> Result<ResponseDraft, ResourceError> {
        let _ = Method::Get;
        Ok(ResponseDraft::ok(Bytes::new()))
    }
}

fn main() {}