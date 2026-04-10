//! Pathway-owned route and runtime data types.
//!
//! These types stay inside `jacquard-pathway` even when they are assembled
//! from shared world inputs and shared route lifecycle objects. Includes the
//! active-route record (`ActivePathwayRoute`) that holds forwarding, repair,
//! handoff, and anti-entropy sub-state; per-route projection views used by
//! the router-facing API (`PathwayActiveRouteView`, `PathwayForwardingCursor`);
//! engine-wide control summaries (`PathwayControlState`,
//! `PathwayAntiEntropyState`, `PathwayTransportObservationSummary`); and the
//! round-progress discriminant (`PathwayRoundProgress`) that the host sees
//! after each `engine_tick` call. The planner cache entry (`CachedCandidate`)
//! is also defined here so it can be shared across the `planner` and
//! `runtime` sub-modules without crossing visibility boundaries.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    Blake3Digest, CommitteeSelection, ContentId, DestinationId, DeterministicOrderKey, HealthScore,
    LinkEndpoint, NodeId, PenaltyPoints, ReceiptId, RouteCost, RouteId, RouteLifecycleEvent,
    RouteSummary, RoutingTickChange, RoutingTickHint, RoutingTickOutcome, Tick, TimeWindow,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathwayRouteClass {
    Direct,
    MultiHop,
    Gateway,
    DeferredDelivery,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathwayRouteSegment {
    pub node_id: NodeId,
    pub endpoint: LinkEndpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum PathwayCommitteeStatus {
    NotApplicable,
    Selected(CommitteeSelection),
    SelectorFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathwayPath {
    pub route_id: RouteId,
    pub epoch: jacquard_core::RouteEpoch,
    pub source: NodeId,
    pub destination: DestinationId,
    pub segments: Vec<PathwayRouteSegment>,
    pub valid_for: TimeWindow,
    pub route_class: PathwayRouteClass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathwayForwardingState {
    pub current_owner_node_id: NodeId,
    pub next_hop_index: u8,
    pub in_flight_frames: u32,
    /// `None` means this event has never occurred.
    pub last_ack_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathwayRepairState {
    pub steps_remaining: u32,
    /// `None` means this event has never occurred.
    pub last_repaired_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathwayHandoffState {
    /// `None` means this event has never occurred.
    pub last_receipt_id: Option<ReceiptId>,
    /// `None` means this event has never occurred.
    pub last_handoff_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathwayRouteAntiEntropyState {
    pub partition_mode: bool,
    pub retained_objects: BTreeSet<ContentId<Blake3Digest>>,
    /// `None` means this event has never occurred.
    pub last_refresh_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActivePathwayRoute {
    pub path: PathwayPath,
    pub committee: Option<CommitteeSelection>,
    pub current_epoch: jacquard_core::RouteEpoch,
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub route_cost: RouteCost,
    pub ordering_key: DeterministicOrderKey<RouteId>,
    pub forwarding: PathwayForwardingState,
    pub repair: PathwayRepairState,
    pub handoff: PathwayHandoffState,
    pub anti_entropy: PathwayRouteAntiEntropyState,
}

impl ActivePathwayRoute {
    pub(crate) fn is_in_partition_mode(&self) -> bool {
        self.anti_entropy.partition_mode
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathwayObservedRemoteLink {
    pub last_observed_at_tick: Tick,
    pub stability_score: HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathwayTransportObservationSummary {
    /// `None` means this event has never occurred.
    pub last_observed_at_tick: Option<Tick>,
    pub payload_event_count: u16,
    pub observed_link_count: u16,
    pub reachable_remote_count: u16,
    pub freshness: PathwayTransportFreshness,
    pub stability_score: HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
    pub remote_links: BTreeMap<NodeId, PathwayObservedRemoteLink>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathwayTransportFreshness {
    Fresh,
    Quiet,
    Stale,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathwayAntiEntropyState {
    pub pressure_score: HealthScore,
    /// `None` means this event has never occurred.
    pub last_refreshed_at_tick: Option<Tick>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathwayControlState {
    pub last_updated_at_tick: Tick,
    pub transport_stability_score: HealthScore,
    pub repair_pressure_score: HealthScore,
    pub anti_entropy: PathwayAntiEntropyState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathwayRoundReport {
    pub tick_outcome: RoutingTickOutcome,
    pub ingested_transport_observation_count: usize,
    pub dropped_transport_observation_count: usize,
    pub transport_summary: Option<PathwayTransportObservationSummary>,
    pub control_state: Option<PathwayControlState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathwayRoundWaitState {
    pub next_tick_hint: RoutingTickHint,
    pub pending_transport_observation_count: usize,
    pub dropped_transport_observation_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathwayRoundProgress {
    Advanced(Box<PathwayRoundReport>),
    Waiting(PathwayRoundWaitState),
}

impl PathwayRoundProgress {
    #[must_use]
    pub fn from_tick_outcome(
        tick_outcome: RoutingTickOutcome,
        ingested_transport_observation_count: usize,
        dropped_transport_observation_count: usize,
        pending_transport_observation_count: usize,
        transport_summary: Option<PathwayTransportObservationSummary>,
        control_state: Option<PathwayControlState>,
    ) -> Self {
        if tick_outcome.change == RoutingTickChange::NoChange
            && ingested_transport_observation_count == 0
            && dropped_transport_observation_count == 0
        {
            return Self::Waiting(PathwayRoundWaitState {
                next_tick_hint: tick_outcome.next_tick_hint,
                pending_transport_observation_count,
                dropped_transport_observation_count,
            });
        }

        Self::Advanced(Box::new(PathwayRoundReport {
            tick_outcome,
            ingested_transport_observation_count,
            dropped_transport_observation_count,
            transport_summary,
            control_state,
        }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathwayForwardingCursor {
    pub current_owner_node_id: NodeId,
    pub next_hop_index: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathwayRouteRetentionView {
    pub partition_mode: bool,
    pub retained_object_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathwayActiveRouteView {
    pub route_class: PathwayRouteClass,
    pub first_hop_node_id: Option<NodeId>,
    pub segment_count: usize,
    pub has_committee: bool,
    pub forwarding: PathwayForwardingCursor,
    pub retention: PathwayRouteRetentionView,
    pub repair_steps_remaining: u32,
}

impl From<&ActivePathwayRoute> for PathwayActiveRouteView {
    fn from(active_route: &ActivePathwayRoute) -> Self {
        Self {
            route_class: active_route.path.route_class,
            first_hop_node_id: active_route
                .path
                .segments
                .first()
                .map(|segment| segment.node_id),
            segment_count: active_route.path.segments.len(),
            has_committee: active_route.committee.is_some(),
            forwarding: PathwayForwardingCursor {
                current_owner_node_id: active_route.forwarding.current_owner_node_id,
                next_hop_index: active_route.forwarding.next_hop_index,
            },
            retention: PathwayRouteRetentionView {
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
