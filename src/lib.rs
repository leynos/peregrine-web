//! `Peregrine Web` library.
//!
//! Peregrine is a Rust web framework under construction. This crate currently
//! exports a scaffolding surface rather than a runnable framework: there is no
//! router, server, authentication, or authorization provider yet.
//!
//! The [`spike`] module is an experimental, non-stable specimen used to test
//! resource-owned policy, admission ordering, erased request bodies, and
//! cancellation. Its names carry no stability promise and may change or vanish
//! in any release; the repository's evidence ledger records what the specimen
//! establishes and what it leaves open.

pub mod spike;

// TODO: Remove this stub as soon as actual library functionality exists.
/// Returns a greeting for the library.
#[must_use]
pub const fn greet() -> &'static str { "Hello from Peregrine Web!" }
