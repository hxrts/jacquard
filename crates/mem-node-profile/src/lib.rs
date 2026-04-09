//! In-memory node profile and node-state builders for tests and examples.
//!
//! Control flow: this crate owns only node capability and node-state
//! simulation. Callers build a stable `NodeProfile`, evolve a local
//! `NodeStateSnapshot`, and then assemble a `Node` without importing any mesh
//! planner or router logic.
//!
//! Most callers should start with the [`authoring`] module, especially
//! [`ReferenceNode`]. [`SimulatedNodeProfile`] and [`NodeStateSnapshot`]
//! remain available as the lower-level escape hatches when tests need exact
//! control over the profile/state split. Callers construct shared
//! `LinkEndpoint` values directly via `jacquard-core`.
//!
//! Module map:
//! - [`authoring`]: human-facing node authoring presets
//! - [`profile`]: low-level node capability builder
//! - [`service`]: low-level service descriptor builder
//! - [`state`]: low-level node state builder
//!
//! ```rust
//! use jacquard_core::{
//!     ByteCount, ControllerId, EndpointLocator, LinkEndpoint, NodeId,
//!     RoutingEngineId, Tick, TransportKind,
//! };
//! use jacquard_mem_node_profile::ReferenceNode;
//!
//! let engine = RoutingEngineId::from_contract_bytes(*b"reference-mem-01");
//! let node = ReferenceNode::route_capable(
//!     NodeId([3; 32]),
//!     ControllerId([3; 32]),
//!     LinkEndpoint::new(
//!         TransportKind::WifiAware,
//!         EndpointLocator::Opaque(vec![3]),
//!         ByteCount(512),
//!     ),
//!     &engine,
//!     Tick(1),
//! )
//! .build();
//!
//! assert_eq!(node.profile.endpoints.len(), 1);
//! assert_eq!(node.profile.services.len(), 3);
//! ```
//!
//! Ownership:
//! - `Observed`: extension-facing node capability and node-state modeling only
//! - never plans routes or publishes canonical route truth

#![forbid(unsafe_code)]

pub mod authoring;
pub mod profile;
pub mod service;
pub mod state;

pub use authoring::ReferenceNode;
pub use profile::{SimulatedNodeProfile, DEFAULT_HOLD_CAPACITY_BYTES};
pub use service::SimulatedServiceDescriptor;
pub use state::NodeStateSnapshot;
