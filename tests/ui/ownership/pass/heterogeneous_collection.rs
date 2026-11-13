//! A heterogeneous collection of resources shares one erased interface.

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use peregrine_web::spike::{
    AllocationId, ErasedBody, Method, Policy, RequestView, Resource, ResourceError,
    ResourceObservation, ResourceRunner, ResponseDraft,
};

/// A catalogue-shaped resource.
struct Catalogue {
    /// The policy this resource owns.
    policy: Policy,
    /// State owned by this resource instance.
    state: Arc<()>,
}

#[async_trait]
impl Resource for Catalogue {
    /// Declares the catalogue resource public.
    fn policy(&self, _method: Method) -> &Policy { &self.policy }

    /// Reports the allocation identity the erased interface observes.
    fn observation(&self) -> ResourceObservation {
        ResourceObservation {
            resource: AllocationId::of(&self.state),
            dependency: AllocationId::of(&self.state),
            label: "catalogue",
        }
    }

    /// Returns the catalogue body.
    async fn get(
        &self,
        _view: RequestView<'_>,
        _body: &mut ErasedBody,
    ) -> Result<ResponseDraft, ResourceError> {
        Ok(ResponseDraft::ok(Bytes::from_static(b"catalogue")))
    }
}

/// A billing-shaped resource with a different concrete type.
struct Billing {
    /// The policy this resource owns.
    policy: Policy,
    /// State owned by this resource instance.
    state: Arc<()>,
}

#[async_trait]
impl Resource for Billing {
    /// Declares the billing resource public.
    fn policy(&self, _method: Method) -> &Policy { &self.policy }

    /// Reports the allocation identity the erased interface observes.
    fn observation(&self) -> ResourceObservation {
        ResourceObservation {
            resource: AllocationId::of(&self.state),
            dependency: AllocationId::of(&self.state),
            label: "billing",
        }
    }

    /// Returns the billing body.
    async fn get(
        &self,
        _view: RequestView<'_>,
        _body: &mut ErasedBody,
    ) -> Result<ResponseDraft, ResourceError> {
        Ok(ResponseDraft::ok(Bytes::from_static(b"billing")))
    }
}

/// Registers two different concrete resources through one erased interface.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = ResourceRunner::new();
    let resources: Vec<Arc<dyn Resource>> = vec![
        Arc::new(Catalogue {
            policy: Policy::Public,
            state: Arc::new(()),
        }),
        Arc::new(Billing {
            policy: Policy::Public,
            state: Arc::new(()),
        }),
    ];
    let mut registered = Vec::new();
    for resource in resources {
        registered.push(runner.register(resource, Method::Get)?);
    }
    assert_eq!(
        registered.len(),
        2,
        "each concrete resource type must register through the erased interface"
    );
    Ok(())
}