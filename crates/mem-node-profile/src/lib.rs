//! In-memory node profile and node-state builders for tests and examples.
//!
//! Control flow: this crate owns only node capability and node-state
//! simulation. Callers build a stable `NodeProfile`, evolve a local
//! `NodeStateSnapshot`, and then assemble a `Node` without importing any mesh
//! planner or router logic.
//!
//! Ownership:
//! - `Observed`: extension-facing node capability and node-state modeling only
//! - never plans routes or publishes canonical route truth

#![forbid(unsafe_code)]

pub mod profile;
pub mod services;
pub mod state;

pub use profile::SimulatedNodeProfile;
pub use services::SimulatedServiceDescriptor;
pub use state::NodeStateSnapshot;
