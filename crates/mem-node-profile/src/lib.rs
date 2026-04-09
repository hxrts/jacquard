//! In-memory node profile and node-state builders for tests and examples.
//!
//! Control flow: this crate owns only node capability and node-state
//! simulation. Callers build a stable `NodeProfile`, evolve a local
//! `NodeStateSnapshot`, and then assemble a `Node` without importing any engine
//! planner or router logic.
//!
//! Most callers should start with the [`authoring`] module, especially
//! [`NodePreset`]. [`SimulatedNodeProfile`] and [`NodeStateSnapshot`]
//! remain available as the lower-level escape hatches when tests need exact
//! control over the profile/state split. Callers construct shared
//! `LinkEndpoint` values directly via `jacquard-core` or use
//! [`jacquard_adapter::opaque_endpoint`] for the common opaque-locator path.
//!
//! Module map:
//! - [`authoring`]: human-facing node authoring presets
//! - [`profile`]: low-level node capability builder
//! - [`service`]: low-level service descriptor builder
//! - [`state`]: low-level node state builder
//!
//! ```rust
//! use jacquard_adapter::opaque_endpoint;
//! use jacquard_core::{
//!     ByteCount, ControllerId, NodeId, RoutingEngineId, Tick, TransportKind,
//! };
//! use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
//!
//! let engine = RoutingEngineId::from_contract_bytes(*b"reference-mem-01");
//! let node = NodePreset::route_capable(
//!     NodePresetOptions::new(
//!         NodeIdentity::new(NodeId([3; 32]), ControllerId([3; 32])),
//!         opaque_endpoint(TransportKind::WifiAware, vec![3], ByteCount(512)),
//!         Tick(1),
//!     ),
//!     &engine,
//! )
//! .build();
//!
//! assert_eq!(node.profile.endpoints.len(), 1);
//! assert_eq!(node.profile.services.len(), 3);
//! ```
//!
//! Starter path:
//! 1. Construct an endpoint with `jacquard_adapter::opaque_endpoint`.
//! 2. Construct `NodePresetOptions` from a `NodeIdentity`, endpoint, and tick.
//! 3. Choose `NodePreset::route_capable(...)` or
//!    `NodePreset::route_capable_for_engines(...)`.
//! 4. Drop to `SimulatedNodeProfile`, `SimulatedServiceDescriptor`, or
//!    `NodeStateSnapshot` only when the low-level split matters to the test.
//!
//! Ownership:
//! - `Observed`: extension-facing node capability and node-state modeling only
//! - never plans routes or publishes canonical route truth

#![forbid(unsafe_code)]

pub mod authoring;
pub mod profile;
pub mod service;
pub mod state;

pub use authoring::{
    default_route_service_window, NodeIdentity, NodePreset, NodePresetOptions,
    DEFAULT_ROUTE_SERVICE_SCOPE_ID, DEFAULT_ROUTE_SERVICE_WINDOW_TICKS,
};
pub use profile::{SimulatedNodeProfile, DEFAULT_HOLD_CAPACITY_BYTES};
pub use service::{RouteServiceBundle, SimulatedServiceDescriptor};
pub use state::NodeStateSnapshot;
