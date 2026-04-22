//! Generic router middleware for Jacquard.
//!
//! Control flow: a host registers one or more routing engines with
//! the router, then the router owns the canonical activation and maintenance
//! flow. It collects candidates across engines, selects one admissible route,
//! asks the chosen engine to materialize its private runtime state, and only
//! then publishes the canonical route snapshot and commitments.
//!
//! Ownership:
//! - `ActorOwned`: canonical route table, lease transfer, commitment view
//! - engine-private runtime state stays below the shared engine boundary
//! - success-bearing mutations are proof-gated by typed engine evidence

#![forbid(unsafe_code)]

mod delivery;
mod middleware;
mod runtime;

pub use delivery::admitted_delivery_intent;
pub use middleware::{FixedPolicyEngine, MultiEngineRouter};
