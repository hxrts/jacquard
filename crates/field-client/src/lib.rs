//! Standalone field-engine client harness for Jacquard.
//!
//! `jacquard-field` is a corridor-envelope routing engine that derives routes
//! from direct link observations rather than a propagated OGM table. This crate
//! wraps the engine in a minimal host harness scoped to field-only scenarios.
//! It is narrower than `reference-client`: it composes only `jacquard-field`,
//! uses only the in-memory profile crates for fixtures, and does not pull in
//! the bridge or choreography layers.
//!
//! Three surfaces are exposed: [`FieldClientBuilder`] and [`FieldClient`] for
//! driving the engine through the full route lifecycle; the [`topology`] module
//! for preset node and link fixtures wired to `FIELD_ENGINE_ID`; and
//! [`default_objective`] and [`default_profile`] for a canonical
//! `PartitionTolerant` routing configuration used across tests.
//!
//! Starter path:
//! 1. Build a topology with [`topology::node`] and [`topology::link`].
//! 2. Construct a [`FieldClient`] through [`FieldClientBuilder`].
//! 3. Call [`FieldClient::advance_round`] to seed local field state.
//! 4. Activate a route, forward a payload, and drain peer ingress.

#![forbid(unsafe_code)]

mod client;
pub mod topology;

pub use client::{default_objective, default_profile, FieldClient, FieldClientBuilder};
pub use jacquard_field::{FieldEngine, FIELD_CAPABILITIES, FIELD_ENGINE_ID};
pub use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
    SharedInMemoryNetwork, SimulatedLinkProfile,
};
pub use jacquard_mem_node_profile::{
    NodeIdentity, NodePreset, NodePresetOptions, NodeStateSnapshot, RouteServiceBundle,
    SimulatedNodeProfile, SimulatedServiceDescriptor,
};
