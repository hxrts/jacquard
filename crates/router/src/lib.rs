//! Mesh-only router and control/data plane surfaces for Jacquard.
//!
//! Control flow intuition: the router owns canonical route identity, lease
//! issuance, and active-route publication. It asks one mesh engine for
//! candidates and proofs, then commits the resulting canonical route state on
//! the router side. Forwarding remains mesh-engine-specific for now, so the
//! router uses a small local bridge trait instead of pretending the data plane
//! is already engine-neutral.
//!
//! Ownership:
//! - `ActorOwned`: canonical route table, lease transfer, and commitment view
//! - never an engine-private runtime owner
//! - success-bearing mutations are proof-gated by typed engine evidence

#![forbid(unsafe_code)]

mod mesh_router;
mod runtime;

pub use mesh_router::{FixedPolicyEngine, MeshOnlyRouter, MeshRouterEngineBridge};
