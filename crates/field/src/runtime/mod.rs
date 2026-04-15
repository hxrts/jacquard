//! `RoutingEngine` and `RouterManagedEngine` implementations for `FieldEngine`.
//!
//! `materialize_route` decodes the backend token, validates the destination
//! state, and records an `ActiveFieldRoute` keyed by route ID. `engine_tick`
//! drives the per-tick control loop: seeding destinations from topology,
//! advancing the PI control plane, refreshing destination observers, stepping
//! protocol sessions, and deriving the attractor view. `maintain_route`
//! evaluates the installed route on each maintenance trigger, checking
//! corridor support, congestion price, corridor-envelope realization drift,
//! and frontier freshness to decide between hold fallback, replacement, or
//! continuation. Routes expire when delivery support falls below 250 permille
//! or the frontier has been stale for more than four ticks.

use jacquard_core::{
    Configuration, DestinationId, Fact, FactBasis, HealthScore, Limit, NodeId,
    PublishedRouteRecord, ReachabilityState, RouteBinding, RouteCommitment, RouteCommitmentFailure,
    RouteCommitmentId, RouteCommitmentResolution, RouteError, RouteHealth, RouteId,
    RouteInstallation, RouteInvalidationReason, RouteLifecycleEvent, RouteMaintenanceFailure,
    RouteMaintenanceOutcome, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteMaterializationInput, RouteMaterializationProof, RouteOperationId, RouteProgressContract,
    RouteProgressState, RouteRuntimeError, RouteRuntimeState, RouteSelectionError,
    RoutingTickChange, RoutingTickContext, RoutingTickHint, RoutingTickOutcome, Tick,
    TimeoutPolicy, TransportObservation,
};
use jacquard_traits::{Blake3Hashing, Hashing, RouterManagedEngine, RoutingEngine};

use crate::{
    attractor::{derive_local_attractor_view_with_policy, rank_frontier_by_attractor_with_policy},
    choreography::{
        FieldChoreographyAdvance, FieldHostWaitStatus, FieldProtocolCheckpoint, FieldProtocolKind,
        FieldProtocolReconfigurationCause, FieldProtocolSessionKey, QueuedProtocolSend,
        FIELD_PROTOCOL_SESSION_MAX,
    },
    control::{advance_control_plane_with_policy, ControlMeasurements},
    engine::FieldRuntimeRoundArtifact,
    observer::{update_destination_observer, ObserverInputs},
    operational::{FieldDestinationDecisionContext, FieldRuntimeDecisionContext},
    planner::{
        admission::continuity_band_for_state_with_config,
        promotion::{promotion_assessment_for_route_with_policy, FieldBootstrapDecision},
    },
    recovery::{
        FieldPromotionBlocker, FieldRouteRecoveryOutcome, FieldRouteRecoveryTrigger,
        StoredFieldRouteRecovery,
    },
    route::{
        decode_backend_token, ActiveFieldRoute, FieldBootstrapClass, FieldContinuityBand,
        FieldWitnessDetail,
    },
    state::{
        CorridorBeliefEnvelope, HopBand, NeighborContinuation, ObserverInputSignature,
        SupportBucket, SUMMARY_HEARTBEAT_TICKS,
    },
    summary::{
        summary_divergence, DirectEvidence, EvidenceContributionClass, FieldSummary,
        LocalOriginTrace, SummaryDestinationKey, SummaryUncertaintyClass,
        FIELD_SUMMARY_ENCODING_BYTES,
    },
    FieldEngine, FieldPolicyEvent, FieldPolicyGate, FieldPolicyReason,
};

const FIELD_COMMITMENT_ATTEMPT_COUNT_MAX: u32 = 2;
const FIELD_COMMITMENT_INITIAL_BACKOFF_MS: u32 = 25;
const FIELD_COMMITMENT_BACKOFF_MS_MAX: u32 = 25;
const FIELD_COMMITMENT_OVERALL_TIMEOUT_MS: u32 = 50;
const FIELD_COMMITMENT_ID_DOMAIN: &[u8] = b"field-route-commitment";
pub(crate) const FIELD_ROUTE_WEAK_SUPPORT_FLOOR: u16 = 220;
const FIELD_DEGRADED_STEADY_STALE_TICKS_MAX: u64 = 5;
const FIELD_COMMITMENT_BOOTSTRAP_SUPPORT_FLOOR: u16 = 140;
const FIELD_COMMITMENT_BOOTSTRAP_SERVICE_RELIEF_PERMILLE: u16 = 20;
const FIELD_COMMITMENT_DEGRADED_SUPPORT_FLOOR: u16 = 160;
const FIELD_COMMITMENT_DEGRADED_SERVICE_RELIEF_PERMILLE: u16 = 20;
const FIELD_COMMITMENT_DISCOVERY_SUPPORT_FLOOR: u16 = 220;
const FIELD_COMMITMENT_STEADY_SUPPORT_FLOOR: u16 = 250;
const FIELD_FAILURE_POST_SHIFT_GRACE_RELIEF_PERMILLE: u16 = 30;
const FIELD_FAILURE_DISCOVERY_RELIEF_PERMILLE: u16 = 40;
const FIELD_STALE_SERVICE_RELIEF_TICKS: u64 = 2;
const FIELD_STALE_DISCOVERY_RELIEF_TICKS: u64 = 5;
const FIELD_STALE_STEADY_DISCOVERY_TICKS_MAX: u64 = 8;
const FIELD_STALE_STEADY_TICKS_MAX: u64 = 4;
const FIELD_STALE_POST_SHIFT_GRACE_TICKS: u64 = 2;
const FIELD_WEAK_SUPPORT_DISCOVERY_RELIEF_PERMILLE: u16 = 50;

mod continuation;
mod control;
mod observer;
mod routing;
mod sessions;

use continuation::{
    continuation_shift_grace_active, field_commitment_id_for_route, node_corridor_viable,
    node_runtime_continuation_neighbors, observer_input_signature,
    pending_forward_continuations_for_maintenance, preferred_node_shift_neighbor,
    preferred_service_shift_neighbor, route_health_for, service_corridor_viable,
    service_runtime_continuation_neighbors, should_transmit_summary,
    synthesized_node_carry_forward_ranked,
};
use observer::{
    anti_entropy_summary_for_destination_with_policy, direct_evidence_for_destination,
    forward_evidence_for_observer_with_policy, merge_pending_forward_continuations,
    refresh_frontier_from_evidence, summary_for_destination,
    synthesized_node_forward_evidence_from_active_routes_with_policy,
    updated_promotion_window_score, ForwardEvidenceInput,
};
use sessions::destination_objective_class;
