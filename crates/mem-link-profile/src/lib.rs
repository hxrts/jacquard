//! In-memory link-profile, carrier, retention, and runtime-effect builders.
//!
//! Control flow: this crate models endpoints and link state, carries
//! frames over an in-memory network, and exposes deterministic retention and
//! runtime-effect adapters for tests. It does not plan routes or interpret mesh
//! policy; it only provides reusable in-memory infrastructure.
//!
//! Ownership:
//! - `Observed`: link capability and transport observation surface only
//! - never mints canonical route truth or performs routing decisions

#![forbid(unsafe_code)]

mod effects;
mod endpoint;
mod frame_carrier;
mod link_state;
mod protocol;
mod retention;

pub use effects::InMemoryRuntimeEffects;
pub use endpoint::SharedInMemoryNetwork;
pub use frame_carrier::InMemoryTransport;
pub use link_state::SimulatedLinkProfile;
pub use protocol::{ble_endpoint, opaque_endpoint};
pub use retention::InMemoryRetentionStore;
