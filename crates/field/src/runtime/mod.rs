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
    attractor::{derive_local_attractor_view, rank_frontier_by_attractor},
    choreography::{
        FieldChoreographyAdvance, FieldHostWaitStatus, FieldProtocolCheckpoint, FieldProtocolKind,
        FieldProtocolReconfigurationCause, FieldProtocolSessionKey, QueuedProtocolSend,
        FIELD_PROTOCOL_SESSION_MAX,
    },
    control::{advance_control_plane, ControlMeasurements},
    engine::FieldRuntimeRoundArtifact,
    observer::{update_destination_observer, ObserverInputs},
    planner::{
        admission::continuity_band_for_state_with_config,
        promotion::{promotion_assessment_for_route, FieldBootstrapDecision},
    },
    recovery::{
        FieldPromotionBlocker, FieldRouteRecoveryOutcome, FieldRouteRecoveryTrigger,
        StoredFieldRouteRecovery,
    },
    route::{decode_backend_token, ActiveFieldRoute, FieldBootstrapClass, FieldContinuityBand},
    state::{
        HopBand, NeighborContinuation, ObserverInputSignature, SupportBucket,
        SUMMARY_HEARTBEAT_TICKS,
    },
    summary::{
        summary_divergence, DirectEvidence, EvidenceContributionClass, FieldSummary,
        LocalOriginTrace, SummaryDestinationKey, SummaryUncertaintyClass,
        FIELD_SUMMARY_ENCODING_BYTES,
    },
    FieldEngine,
};

const FIELD_COMMITMENT_ATTEMPT_COUNT_MAX: u32 = 2;
const FIELD_COMMITMENT_INITIAL_BACKOFF_MS: u32 = 25;
const FIELD_COMMITMENT_BACKOFF_MS_MAX: u32 = 25;
const FIELD_COMMITMENT_OVERALL_TIMEOUT_MS: u32 = 50;
const FIELD_COMMITMENT_ID_DOMAIN: &[u8] = b"field-route-commitment";
pub(crate) const FIELD_ROUTE_FAILURE_SUPPORT_FLOOR: u16 = 180;
pub(crate) const FIELD_ROUTE_WEAK_SUPPORT_FLOOR: u16 = 220;
const FIELD_BOOTSTRAP_FAILURE_SUPPORT_FLOOR: u16 = 140;
const FIELD_DEGRADED_STEADY_FAILURE_SUPPORT_FLOOR: u16 = 160;
const FIELD_BOOTSTRAP_STALE_TICKS_MAX: u64 = 6;
const FIELD_DEGRADED_STEADY_STALE_TICKS_MAX: u64 = 5;
const FIELD_ENVELOPE_SHIFT_SUPPORT_DELTA_MAX: u16 = 180;

mod continuation;
mod control;
mod observer;
mod routing;
mod sessions;
#[cfg(test)]
#[path = "../../tests/runtime/mod.rs"]
mod tests;

use continuation::*;
use observer::*;
use sessions::*;
