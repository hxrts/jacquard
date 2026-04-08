//! Mesh-facing subcomponent interfaces.
//!
//! These traits stay shared only for narrow read-only mesh boundaries that an
//! external host, test harness, or alternate first-party mesh implementation
//! may legitimately swap or observe without reaching into mesh-private
//! planner/runtime state.
//!
//! Effect boundary:
//! - `MeshTopologyModel` is read-only. It should be deterministic with respect
//!   to its inputs and must not mutate canonical route state.
//! - `RetentionStore` remains the opaque deferred-delivery storage boundary,
//!   but it now lives on the neutral shared effect surface rather than in this
//!   mesh-named module.

use jacquard_core::{Configuration, Link, LinkEndpoint, Node, NodeId, Tick};
use jacquard_macros::purity;

use crate::{RetentionStore, RoutingEngine};

#[purity(read_only)]
/// Deterministic, read-only topology queries used by the mesh planner/runtime.
///
/// Read-only deterministic boundary.
///
/// Mesh-specific peer and neighborhood estimates belong behind this trait
/// boundary rather than in `jacquard-core`. The associated estimate types let
/// one mesh implementation expose novelty, reach, bridge, or flow heuristics to
/// its own planner/runtime without turning them into shared cross-engine
/// schema. This trait remains shared because substituting the read-only
/// topology view is a legitimate extension point; the estimate-access traits
/// themselves stay mesh-owned.
pub trait MeshTopologyModel {
    type PeerEstimate;
    type NeighborhoodEstimate;

    #[must_use]
    fn local_node(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Option<Node>;

    #[must_use]
    fn neighboring_nodes(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<(NodeId, Node)>;

    #[must_use]
    fn reachable_endpoints(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<LinkEndpoint>;

    #[must_use]
    fn adjacent_links(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<Link>;

    #[must_use]
    fn peer_estimate(
        &self,
        local_node_id: &NodeId,
        peer_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<Self::PeerEstimate>;

    #[must_use]
    fn neighborhood_estimate(
        &self,
        local_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<Self::NeighborhoodEstimate>;
}

#[purity(read_only)]
/// Narrow mesh-specialized routing-engine boundary for mesh-private
/// deterministic semantics.
///
/// Planning purity stays in `RoutingEnginePlanner` plus `MeshTopologyModel`.
/// This trait binds the effectful routing-engine runtime only to the mesh
/// subcomponents that remain mesh-specific after transport is moved onto the
/// shared `TransportEffects` boundary. It intentionally exposes only
/// read-only subcomponent access; mutation of retained payload state remains
/// engine-private.
///
/// Effectful runtime boundary with read-only subcomponent accessors.
pub trait MeshRoutingEngine: RoutingEngine {
    type TopologyModel: MeshTopologyModel;
    type Retention: RetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel;

    fn retention_store(&self) -> &Self::Retention;
}
