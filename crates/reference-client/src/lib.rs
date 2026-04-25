//! Reference host bridge for Jacquard integration tests and examples.
//!
//! This crate demonstrates the intended host-side composition:
//! - construct a small `Observation<Configuration>`
//! - build one router plus one or more engines through [`ClientBuilder`]
//! - attach one bridge-owned transport driver
//! - queue outbound transport commands during synchronous routing work
//! - stamp ingress with Jacquard logical time at the bridge boundary
//! - advance the router through explicit synchronous rounds
//!
//! `clients` contains client builders. `bridge` contains the host-bridge
//! surface. In-memory profile types from `mem-link-profile` and
//! `mem-node-profile` are re-exported so downstream examples can compose the
//! reference bridge without reaching into the lower-level profile crates.
//!
//! Starter path:
//! 1. Build a small `Observation<Configuration>`.
//! 2. Use [`ClientBuilder`] to choose engines and queue configuration.
//! 3. Bind the resulting [`HostBridge`] and drive explicit rounds.
//! 4. Optionally project host-readable topology and route state with
//!    [`TopologyProjector`].
//!
//! ```rust
//! use std::collections::BTreeMap;
//!
//! use jacquard_core::{
//!     Configuration, Environment, FactSourceClass, Observation,
//!     OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutingEvidenceClass,
//!     Tick,
//! };
//! use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
//! use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
//! use jacquard_pathway::PATHWAY_ENGINE_ID;
//! use jacquard_reference_client::{ClientBuilder, SharedInMemoryNetwork};
//!
//! let topology = Observation {
//!     value: Configuration {
//!         epoch: RouteEpoch(1),
//!         nodes: BTreeMap::from([
//!             (
//!                 jacquard_core::NodeId([1; 32]),
//!                 NodePreset::route_capable(
//!                     NodePresetOptions::new(
//!                         NodeIdentity::new(
//!                             jacquard_core::NodeId([1; 32]),
//!                             jacquard_core::ControllerId([1; 32]),
//!                         ),
//!                         jacquard_host_support::opaque_endpoint(
//!                             jacquard_core::TransportKind::WifiAware,
//!                             vec![1],
//!                             jacquard_core::ByteCount(256),
//!                         ),
//!                         Tick(1),
//!                     ),
//!                     &PATHWAY_ENGINE_ID,
//!                 )
//!                 .build(),
//!             ),
//!             (
//!                 jacquard_core::NodeId([2; 32]),
//!                 NodePreset::route_capable(
//!                     NodePresetOptions::new(
//!                         NodeIdentity::new(
//!                             jacquard_core::NodeId([2; 32]),
//!                             jacquard_core::ControllerId([2; 32]),
//!                         ),
//!                         jacquard_host_support::opaque_endpoint(
//!                             jacquard_core::TransportKind::WifiAware,
//!                             vec![2],
//!                             jacquard_core::ByteCount(256),
//!                         ),
//!                         Tick(1),
//!                     ),
//!                     &PATHWAY_ENGINE_ID,
//!                 )
//!                 .build(),
//!             ),
//!         ]),
//!         links: BTreeMap::from([(
//!             (
//!                 jacquard_core::NodeId([1; 32]),
//!                 jacquard_core::NodeId([2; 32]),
//!             ),
//!             LinkPreset::lossy(LinkPresetOptions::new(
//!                 jacquard_host_support::opaque_endpoint(
//!                     jacquard_core::TransportKind::WifiAware,
//!                     vec![2],
//!                     jacquard_core::ByteCount(256),
//!                 ),
//!                 Tick(1),
//!             ))
//!             .build(),
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
//! .build()
//! .expect("build reference client");
//! let mut bound = client.bind();
//! bound.advance_round().expect("advance round");
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

pub use bridge::{
    BoundHostBridge, BridgeQueueConfig, BridgeRoundProgress, BridgeRoundReport, BridgeWaitState,
    HostBridge,
};
pub use clients::{
    ClientBuilder, EngineKind, ReferenceClient, ReferenceClientBuildError, ReferenceRouter,
};
pub use jacquard_host_support::{
    ObservedLink, ObservedNode, ObservedRoute, ObservedRouteShape, TopologyProjector,
    TopologySnapshot,
};
pub use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport, LinkPreset,
    LinkPresetOptions, SharedInMemoryNetwork, SimulatedLinkProfile,
};
pub use jacquard_mem_node_profile::{
    NodeIdentity, NodePreset, NodePresetOptions, NodeStateSnapshot, RouteServiceBundle,
    SimulatedNodeProfile, SimulatedServiceDescriptor,
};
