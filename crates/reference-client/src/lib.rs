//! Reference client wiring for Jacquard integration tests and examples.
//!
//! Control flow: a reference client owns only local host composition.
//! It assembles shared topology observations, a router instance, and in-memory
//! transport/runtime adapters, then submits typed router operations. It does
//! not mint canonical route truth on its own.
//! Reusable reference topology builders live in `topology`, so other crates can
//! compose the same in-memory node/link shapes without copying fixture logic.
//!
//! Ownership:
//! - narrow local `ActorOwned` host loop for composition only
//! - observational with respect to canonical route truth

#![forbid(unsafe_code)]

mod clients;
pub mod topology;

use jacquard_core::{Configuration, Observation};
pub use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
    SharedInMemoryNetwork, SimulatedLinkProfile,
};
pub use jacquard_mem_node_profile::{
    NodeStateSnapshot, SimulatedNodeProfile, SimulatedServiceDescriptor,
};
pub use clients::{
    build_mesh_batman_client, build_mesh_batman_client_with_profile, build_mesh_client,
    build_mesh_client_with_profile, MeshClient, MeshRouter,
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

    pub fn replace_topology(&mut self, topology: Observation<Configuration>) {
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
