//! Mesh-owned route and runtime data types.
//!
//! These types stay inside `jacquard-mesh` even when they are assembled
//! from shared world inputs and shared route lifecycle objects.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    Blake3Digest, CommitteeSelection, ContentId, DestinationId, DeterministicOrderKey,
    HealthScore, LinkEndpoint, NodeId, PenaltyPoints, ReceiptId, RouteCost, RouteId,
    RouteLifecycleEvent, RouteSummary, Tick, TimeWindow,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshRouteClass {
    Direct,
    MultiHop,
    Gateway,
    DeferredDelivery,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshRouteSegment {
    pub node_id: NodeId,
    pub endpoint: LinkEndpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum MeshCommitteeStatus {
    NotApplicable,
    Selected(CommitteeSelection),
    SelectorFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshPath {
    pub route_id: RouteId,
    pub epoch: jacquard_core::RouteEpoch,
    pub source: NodeId,
    pub destination: DestinationId,
    pub segments: Vec<MeshRouteSegment>,
    pub valid_for: TimeWindow,
    pub route_class: MeshRouteClass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshForwardingState {
    pub current_owner_node_id: NodeId,
    pub next_hop_index: u8,
    pub in_flight_frames: u32,
    /// `None` means this event has never occurred.
    pub last_ack_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshRepairState {
    pub steps_remaining: u32,
    /// `None` means this event has never occurred.
    pub last_repaired_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshHandoffState {
    /// `None` means this event has never occurred.
    pub last_receipt_id: Option<ReceiptId>,
    /// `None` means this event has never occurred.
    pub last_handoff_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshRouteAntiEntropyState {
    pub partition_mode: bool,
    pub retained_objects: BTreeSet<ContentId<Blake3Digest>>,
    /// `None` means this event has never occurred.
    pub last_refresh_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActiveMeshRoute {
    pub path: MeshPath,
    pub committee: Option<CommitteeSelection>,
    pub current_epoch: jacquard_core::RouteEpoch,
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub route_cost: RouteCost,
    pub ordering_key: DeterministicOrderKey<RouteId>,
    pub forwarding: MeshForwardingState,
    pub repair: MeshRepairState,
    pub handoff: MeshHandoffState,
    pub anti_entropy: MeshRouteAntiEntropyState,
}

impl ActiveMeshRoute {
    pub(crate) fn is_in_partition_mode(&self) -> bool {
        self.anti_entropy.partition_mode
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshObservedRemoteLink {
    pub last_observed_at_tick: Tick,
    pub stability_score: HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshTransportObservationSummary {
    /// `None` means this event has never occurred.
    pub last_observed_at_tick: Option<Tick>,
    pub payload_event_count: u16,
    pub observed_link_count: u16,
    pub reachable_remote_count: u16,
    pub freshness: MeshTransportFreshness,
    pub stability_score: HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
    pub remote_links: BTreeMap<NodeId, MeshObservedRemoteLink>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshTransportFreshness {
    Fresh,
    Quiet,
    Stale,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshAntiEntropyState {
    pub pressure_score: HealthScore,
    /// `None` means this event has never occurred.
    pub last_refreshed_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshControlState {
    pub last_updated_at_tick: Tick,
    pub transport_stability_score: HealthScore,
    pub repair_pressure_score: HealthScore,
    pub anti_entropy: MeshAntiEntropyState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshForwardingCursor {
    pub current_owner_node_id: NodeId,
    pub next_hop_index: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshRouteRetentionView {
    pub partition_mode: bool,
    pub retained_object_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshActiveRouteView {
    pub route_class: MeshRouteClass,
    pub first_hop_node_id: Option<NodeId>,
    pub segment_count: usize,
    pub has_committee: bool,
    pub forwarding: MeshForwardingCursor,
    pub retention: MeshRouteRetentionView,
    pub repair_steps_remaining: u32,
}

impl From<&ActiveMeshRoute> for MeshActiveRouteView {
    fn from(active_route: &ActiveMeshRoute) -> Self {
        Self {
            route_class: active_route.path.route_class,
            first_hop_node_id: active_route
                .path
                .segments
                .first()
                .map(|segment| segment.node_id),
            segment_count: active_route.path.segments.len(),
            has_committee: active_route.committee.is_some(),
            forwarding: MeshForwardingCursor {
                current_owner_node_id: active_route.forwarding.current_owner_node_id,
                next_hop_index: active_route.forwarding.next_hop_index,
            },
            retention: MeshRouteRetentionView {
                partition_mode: active_route.anti_entropy.partition_mode,
                retained_object_count: active_route.anti_entropy.retained_objects.len(),
            },
            repair_steps_remaining: active_route.repair.steps_remaining,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct CachedCandidate {
    pub(super) route_id: RouteId,
    pub(super) path_metric_score: u32,
    pub(super) summary: RouteSummary,
    pub(super) estimate: jacquard_core::Estimate<jacquard_core::RouteEstimate>,
    pub(super) admission_check: jacquard_core::RouteAdmissionCheck,
    pub(super) witness: jacquard_core::RouteWitness,
    pub(super) ordering_key: DeterministicOrderKey<RouteId>,
}
