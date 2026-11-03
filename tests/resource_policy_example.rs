//! Loads the published example so `cargo test` exercises its demonstrations.
//!
//! The example is a runnable binary, so nothing would otherwise check that its
//! assertions still hold. Including it as a module runs the same code path the
//! reader is invited to run with `cargo run --example resource_policy_spike`.

#[path = "../examples/resource_policy_spike.rs"]
#[expect(
    dead_code,
    reason = "the example's `main` is unused when the file is loaded as a module"
)]
mod published_example;

use std::error::Error;

#[test]
fn published_example_demonstrations_hold() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(published_example::demonstrate())
}
