//! Public pathway-specific contract surfaces.
//!
//! These traits stay in `jacquard-pathway`, not `jacquard-traits`, because they
//! describe first-party explicit-path semantics rather than engine-neutral
//! routing behavior. External code may still use them to swap read-only
//! topology models or observe explicit pathway subcomponents, but that coupling
//! is now honest and local to the pathway crate.

use jacquard_core::{Configuration, Link, LinkEndpoint, Node, NodeId, Tick};
use jacquard_traits::{RetentionStore, RoutingEngine};

#[jacquard_traits::purity(read_only)]
/// Deterministic, read-only topology queries used by the pathway
/// planner/runtime.
///
/// Pathway-specific peer and neighborhood estimates belong behind this trait
/// boundary rather than in `jacquard-core`. The associated estimate types let
/// the pathway implementation expose novelty, reach, bridge, or flow heuristics
/// to its own planner/runtime without turning them into shared cross-engine
/// schema.
pub trait PathwayTopologyModel {
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
/// Narrow pathway-specialized routing-engine boundary for read-only pathway
/// subcomponent access.
///
/// Planning purity stays in `RoutingEnginePlanner` plus `PathwayTopologyModel`.
/// This trait binds the effectful routing-engine runtime only to the pathway
/// subcomponents that remain engine-specific after transport send stays on the
/// shared capability surface and ingress moves onto explicit router rounds.
pub trait PathwayRoutingEngine: RoutingEngine {
    type TopologyModel: PathwayTopologyModel;
    type Retention: RetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel;

    fn retention_store(&self) -> &Self::Retention;
}
