//! Compile-time ownership contract for the resource-policy spike.
//!
//! Positive fixtures must compile against the exported spike module, and
//! negative fixtures must fail for one named ownership, borrow, or trait
//! requirement. The reviewed `.stderr` files are evidence in their own right:
//! they record which diagnostic the pinned compiler emitted.

/// Runs the reviewed compile-pass and compile-fail fixtures.
#[test]
fn ownership_contract_holds() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/ownership/pass/*.rs");
    cases.compile_fail("tests/ui/ownership/fail/*.rs");
}
