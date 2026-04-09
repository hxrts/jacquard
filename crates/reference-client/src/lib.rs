//! Reference client for Jacquard integration tests and examples.
//!
//! Provides a thin `Client<Router>` wrapper that pairs one shared
//! topology observation with one router instance. Concrete router-plus-
//! engine builders live in the `clients` module. Reusable `Node` and
//! `Link` fixtures live in the `topology` module. In-memory profile types
//! from `mem-link-profile` and `mem-node-profile` are re-exported so
//! downstream test crates only depend on this crate.
//!
//! Ownership:
//! - observational with respect to canonical route truth
//! - never publishes the canonical route table, only the router does

#![forbid(unsafe_code)]

mod clients;
pub mod topology;

pub use clients::{
    build_pathway_batman_client, build_pathway_batman_client_with_profile,
    build_pathway_client, build_pathway_client_with_profile, PathwayClient,
    PathwayRouter,
};
use jacquard_core::{Configuration, Observation};
pub use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
    SharedInMemoryNetwork, SimulatedLinkProfile,
};
pub use jacquard_mem_node_profile::{
    NodeStateSnapshot, SimulatedNodeProfile, SimulatedServiceDescriptor,
};

/// Minimal client wrapper that demonstrates host-side composition.
pub struct Client<Router> {
    topology: Observation<Configuration>,
    router: Router,
}

impl<Router> Client<Router> {
    #[must_use]
    pub fn new(topology: Observation<Configuration>, router: Router) -> Self {
        Self { topology, router }
    }

    #[must_use]
    pub fn topology(&self) -> &Observation<Configuration> {
        &self.topology
    }

    pub fn ingest_topology_observation(
        &mut self,
        topology: Observation<Configuration>,
    ) {
        self.topology = topology;
    }

    #[must_use]
    pub fn router(&self) -> &Router {
        &self.router
    }

    pub fn router_mut(&mut self) -> &mut Router {
        &mut self.router
    }
}
