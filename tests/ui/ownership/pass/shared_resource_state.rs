//! Two registrations can share one retained resource allocation.

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use peregrine_web::spike::{
    AllocationId, ErasedBody, Method, Policy, RequestView, Resource, ResourceError,
    ResourceObservation, ResourceRunner, ResponseDraft,
};

/// A resource whose state is shared rather than duplicated.
struct Catalogue {
    /// The policy this resource owns.
    policy: Policy,
    /// State shared with any clone of the handle.
    state: Arc<u8>,
}

#[async_trait]
impl Resource for Catalogue {
    fn policy(&self, _method: Method) -> &Policy { &self.policy }

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
        Ok(ResponseDraft::ok(Bytes::new()))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resource = Arc::new(Catalogue {
        policy: Policy::Public,
        state: Arc::new(7),
    });
    let erased: Arc<dyn Resource> = resource;
    let runner = ResourceRunner::new();
    let first = runner.register(Arc::clone(&erased), Method::Get)?;
    let second = runner.register(Arc::clone(&erased), Method::Get)?;
    assert_eq!(
        AllocationId::of(first.resource()),
        AllocationId::of(second.resource()),
        "both registrations must retain the same allocation"
    );
    Ok(())
}