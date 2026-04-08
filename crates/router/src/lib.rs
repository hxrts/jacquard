//! Generic router middleware and control/data plane surfaces for Jacquard.
//!
//! Control flow intuition: the router owns canonical route identity, lease
//! issuance, and active-route publication. It asks one concrete routing engine
//! for candidates and proofs, then commits the resulting canonical route state
//! on the router side. The router itself remains engine-family-agnostic and
//! depends only on shared trait boundaries.
//!
//! Ownership:
//! - `ActorOwned`: canonical route table, lease transfer, and commitment view
//! - never an engine-private runtime owner
//! - success-bearing mutations are proof-gated by typed engine evidence

#![forbid(unsafe_code)]

mod runtime;
mod single_engine;

pub use single_engine::{FixedPolicyEngine, SingleEngineRouter};
