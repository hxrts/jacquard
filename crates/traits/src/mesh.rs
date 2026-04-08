//! Mesh-facing subcomponent interfaces.
//!
//! These traits are the high-level contract boundaries for mesh internals that
//! need to remain swappable across crates and runtimes.
//!
//! Effect boundary:
//! - `MeshTopologyModel` is read-only. It should be deterministic with respect
//!   to its inputs and must not mutate canonical route state.
//! - `RetentionStore` is effectful. It stores opaque deferred-delivery
//!   payloads, but it must not interpret higher-level routing semantics.

use jacquard_core::{
    Blake3Digest, Configuration, ContentId, HealthScore, Link, LinkEndpoint, Node,
    NodeId, RetentionError, Tick,
};
use jacquard_macros::purity;

use crate::RoutingEngine;

#[purity(read_only)]
/// Deterministic, read-only topology queries used by the mesh planner/runtime.
///
/// Read-only deterministic boundary.
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

#[purity(read_only)]
/// Score components mesh consumes from a peer-local estimate.
pub trait MeshPeerEstimateAccess {
    fn relay_value_score(&self) -> Option<HealthScore>;
    fn retention_value_score(&self) -> Option<HealthScore>;
    fn stability_score(&self) -> Option<HealthScore>;
    fn service_score(&self) -> Option<HealthScore>;
}

#[purity(read_only)]
/// Score components mesh consumes from a neighborhood-local estimate.
pub trait MeshNeighborhoodEstimateAccess {
    fn density_score(&self) -> Option<HealthScore>;
    fn repair_pressure_score(&self) -> Option<HealthScore>;
    fn partition_risk_score(&self) -> Option<HealthScore>;
    fn service_stability_score(&self) -> Option<HealthScore>;
}

#[purity(effectful)]
/// Effectful deferred-delivery retention boundary.
///
/// Effectful runtime boundary.
pub trait RetentionStore {
    #[must_use = "unchecked retain_payload result silently discards retention failures"]
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError>;

    #[must_use = "unread take_retained_payload result silently discards the held payload"]
    fn take_retained_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RetentionError>;

    #[must_use = "unread contains_retained_payload result silently discards storage errors"]
    fn contains_retained_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, RetentionError>;
}

#[purity(effectful)]
/// Mesh-specialized routing-engine boundary for mesh-private deterministic
/// semantics.
///
/// Planning purity stays in `RoutingEnginePlanner` plus `MeshTopologyModel`.
/// This trait binds the effectful routing-engine runtime only to the mesh
/// subcomponents that remain mesh-specific after transport is moved onto the
/// shared `TransportEffects` boundary.
///
/// Effectful runtime boundary with read-only subcomponent accessors.
pub trait MeshRoutingEngine: RoutingEngine {
    type TopologyModel: MeshTopologyModel;
    type Retention: RetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel;

    fn retention_store(&self) -> &Self::Retention;

    fn retention_store_mut(&mut self) -> &mut Self::Retention;
}
