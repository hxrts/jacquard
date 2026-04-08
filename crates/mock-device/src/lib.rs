//! Mock device-side wiring for Jacquard integration tests and examples.
//!
//! Control flow intuition: a mock device owns only local host composition. It
//! assembles shared topology observations, a router instance, and in-memory
//! transport/runtime adapters, then submits typed router operations. It does
//! not mint canonical route truth on its own.
//!
//! Ownership:
//! - narrow local `ActorOwned` host loop for composition only
//! - observational with respect to canonical route truth

#![forbid(unsafe_code)]

mod mesh;

use jacquard_core::{Configuration, Observation};
pub use mesh::{
    build_mock_mesh_device, build_mock_mesh_device_with_profile, MockMeshDevice,
    MockMeshRouter,
};

/// Minimal device wrapper that demonstrates host-side composition.
pub struct MockDevice<Router> {
    topology: Observation<Configuration>,
    router:   Router,
}

impl<Router> MockDevice<Router> {
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
