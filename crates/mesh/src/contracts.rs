//! Public mesh-specific contract surfaces.
//!
//! These traits stay in `jacquard-mesh`, not `jacquard-traits`, because they
//! describe first-party mesh semantics rather than engine-neutral routing
//! behavior. External code may still use them to swap read-only topology models
//! or observe explicit mesh subcomponents, but that coupling is now honest and
//! local to the mesh crate.

use jacquard_core::{Configuration, Link, LinkEndpoint, Node, NodeId, Tick};
use jacquard_traits::{RetentionStore, RoutingEngine};

#[jacquard_traits::purity(read_only)]
/// Deterministic, read-only topology queries used by the mesh planner/runtime.
///
/// Mesh-specific peer and neighborhood estimates belong behind this trait
/// boundary rather than in `jacquard-core`. The associated estimate types let
/// one mesh implementation expose novelty, reach, bridge, or flow heuristics to
/// its own planner/runtime without turning them into shared cross-engine
/// schema.
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

#[jacquard_traits::purity(read_only)]
/// Narrow mesh-specialized routing-engine boundary for read-only mesh
/// subcomponent access.
///
/// Planning purity stays in `RoutingEnginePlanner` plus `MeshTopologyModel`.
/// This trait binds the effectful routing-engine runtime only to the mesh
/// subcomponents that remain mesh-specific after transport is moved onto the
/// shared `TransportEffects` boundary.
pub trait MeshRoutingEngine: RoutingEngine {
    type TopologyModel: MeshTopologyModel;
    type Retention: RetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel;

    fn retention_store(&self) -> &Self::Retention;
}
