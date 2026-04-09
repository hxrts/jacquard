//! Reference host bridge for Jacquard integration tests and examples.
//!
//! This crate demonstrates the intended host-side composition:
//! - choose or construct a small topology
//! - build one router plus one or more engines through [`ClientBuilder`]
//! - attach one bridge-owned transport driver
//! - queue outbound transport commands during synchronous routing work
//! - stamp ingress with Jacquard logical time at the bridge boundary
//! - advance the router through explicit synchronous rounds
//!
//! `clients` contains client builders. `bridge` contains the host-bridge
//! surface. `topology` contains reusable route-capable `Node` and `Link`
//! builders for tests and examples. In-memory profile types from
//! `mem-link-profile` and `mem-node-profile` are re-exported so downstream test
//! crates only depend on this crate.
//!
//! Starter path:
//! 1. Build a small topology with [`topology::node`] and [`topology::link`].
//! 2. Use [`ClientBuilder`] or `ClientBuildOptions` to choose engines and queue
//!    configuration.
//! 3. Bind the resulting [`HostBridge`] and drive explicit rounds.
//!
//! ```rust
//! use std::collections::BTreeMap;
//!
//! use jacquard_core::{
//!     Configuration, Environment, FactSourceClass, Observation,
//!     OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutingEvidenceClass,
//!     Tick,
//! };
//! use jacquard_reference_client::{topology, ClientBuilder, SharedInMemoryNetwork};
//!
//! let topology = Observation {
//!     value: Configuration {
//!         epoch: RouteEpoch(1),
//!         nodes: BTreeMap::from([
//!             (
//!                 jacquard_core::NodeId([1; 32]),
//!                 topology::node(1).pathway().build(),
//!             ),
//!             (
//!                 jacquard_core::NodeId([2; 32]),
//!                 topology::node(2).pathway().build(),
//!             ),
//!         ]),
//!         links: BTreeMap::from([(
//!             (
//!                 jacquard_core::NodeId([1; 32]),
//!                 jacquard_core::NodeId([2; 32]),
//!             ),
//!             topology::link(2).build(),
//!         )]),
//!         environment: Environment {
//!             reachable_neighbor_count: 1,
//!             churn_permille: RatioPermille(0),
//!             contention_permille: RatioPermille(0),
//!         },
//!     },
//!     source_class: FactSourceClass::Local,
//!     evidence_class: RoutingEvidenceClass::DirectObservation,
//!     origin_authentication: OriginAuthenticationClass::Controlled,
//!     observed_at_tick: Tick(1),
//! };
//!
//! let network = SharedInMemoryNetwork::default();
//! let mut client = ClientBuilder::pathway(
//!     jacquard_core::NodeId([1; 32]),
//!     topology,
//!     network,
//!     Tick(1),
//! )
//! .build();
//! let mut bound = client.bind();
//! let _ = bound.advance_round();
//! ```
//!
//! Ownership:
//! - observational with respect to canonical route truth
//! - bridge-owned with respect to transport ingress, outbound queueing, and
//!   round advancement
//! - never publishes the canonical route table, only the router does

#![forbid(unsafe_code)]

mod bridge;
mod clients;
pub mod defaults;
pub mod topology;

pub use bridge::{
    BoundHostBridge, BridgeQueueConfig, BridgeRoundProgress, BridgeRoundReport,
    BridgeWaitState, HostBridge,
};
pub use clients::{
    build_pathway_batman_client, build_pathway_batman_client_with_profile,
    build_pathway_client, build_pathway_client_with_profile, ClientBuildOptions,
    ClientBuilder, PathwayClient, PathwayRouter,
};
pub use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport, LinkPreset,
    LinkPresetOptions, SharedInMemoryNetwork, SimulatedLinkProfile,
};
pub use jacquard_mem_node_profile::{
    NodeIdentity, NodePreset, NodePresetOptions, NodeStateSnapshot, RouteServiceBundle,
    SimulatedNodeProfile, SimulatedServiceDescriptor,
};
