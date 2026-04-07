//! Mesh-owned route and runtime data types.
//!
//! These types stay inside `jacquard-mesh` even when they are assembled
//! from shared world inputs and shared route lifecycle objects.

use std::collections::BTreeSet;

use jacquard_core::{
    Blake3Digest, CommitteeSelection, ContentId, DestinationId, DeterministicOrderKey, HealthScore,
    LinkEndpoint, NodeId, PenaltyPoints, RouteCost, RouteId, RouteLifecycleEvent, RouteSummary,
    Tick, TimeWindow,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshRouteClass {
    Direct,
    MultiHop,
    Gateway,
    DeferredDelivery,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshRouteSegment {
    pub node_id: NodeId,
    pub endpoint: LinkEndpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshPath {
    pub route_id: RouteId,
    pub epoch: jacquard_core::RouteEpoch,
    pub source: NodeId,
    pub destination: DestinationId,
    pub segments: Vec<MeshRouteSegment>,
    pub valid_for: TimeWindow,
    pub route_class: MeshRouteClass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveMeshRoute {
    pub path: MeshPath,
    pub committee: Option<CommitteeSelection>,
    pub current_epoch: jacquard_core::RouteEpoch,
    pub current_owner_node_id: NodeId,
    pub next_hop_index: u8,
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub in_flight_frames: u32,
    pub last_ack_at_tick: Option<Tick>,
    pub repair_steps_remaining: u32,
    pub route_cost: RouteCost,
    pub partition_mode: bool,
    pub retained_objects: BTreeSet<ContentId<Blake3Digest>>,
    pub ordering_key: DeterministicOrderKey<RouteId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshTransportObservationSummary {
    pub last_observed_at_tick: Option<Tick>,
    pub payload_event_count: u16,
    pub observed_link_count: u16,
    pub reachable_remote_count: u16,
    pub stability_score: HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
}

#[derive(Clone, Debug)]
pub(super) struct CachedCandidate {
    pub(super) route_id: RouteId,
    pub(super) summary: RouteSummary,
    pub(super) estimate: jacquard_core::Estimate<jacquard_core::RouteEstimate>,
    pub(super) admission_check: jacquard_core::RouteAdmissionCheck,
    pub(super) witness: jacquard_core::RouteWitness,
    pub(super) ordering_key: DeterministicOrderKey<RouteId>,
}
