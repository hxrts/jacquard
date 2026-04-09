//! Reference host bridge for Jacquard integration tests and examples.
//!
//! This crate demonstrates the intended host-side composition:
//! - build one router plus one or more engines
//! - attach one bridge-owned transport driver
//! - queue outbound transport commands during synchronous routing work
//! - stamp ingress with Jacquard logical time at the bridge boundary
//! - advance the router through explicit synchronous rounds
//!
//! `clients` contains concrete bridge builders. `bridge` contains the
//! host-bridge surface. `topology` contains reusable route-capable `Node`
//! and `Link` builders for tests and examples. In-memory profile types from
//! `mem-link-profile` and `mem-node-profile` are re-exported so downstream
//! test crates only depend on this crate.
//!
//! Ownership:
//! - observational with respect to canonical route truth
//! - bridge-owned with respect to transport ingress, outbound queueing, and
//!   round advancement
//! - never publishes the canonical route table, only the router does

#![forbid(unsafe_code)]

mod bridge;
mod clients;
pub mod topology;

pub use bridge::{
    BoundHostBridge, BridgeRoundProgress, BridgeRoundReport, BridgeWaitState,
    HostBridge,
};
pub use clients::{
    build_pathway_batman_client, build_pathway_batman_client_with_profile,
    build_pathway_client, build_pathway_client_with_profile, PathwayClient,
    PathwayRouter,
};
pub use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
    SharedInMemoryNetwork, SimulatedLinkProfile,
};
pub use jacquard_mem_node_profile::{
    NodeStateSnapshot, SimulatedNodeProfile, SimulatedServiceDescriptor,
};
