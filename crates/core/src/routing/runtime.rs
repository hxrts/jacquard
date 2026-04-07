//! Router-owned route identity, engine installation results, and runtime
//! lifecycle objects.

use jacquard_macros::{must_use_handle, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    AdmissionDecision, ByteCount, Configuration, Fact, HealthScore, Limit, NodeId,
    Observation, OrderStamp, PenaltyPoints, PriorityPoints, PublicationId, ReceiptId,
    RouteAdmission, RouteCommitmentId, RouteEpoch, RouteId, RouteRuntimeError,
    RouteWitness, Tick, TimeWindow, TimeoutPolicy,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeterministicOrderKey<K> {
    pub stable_key: K,
    pub tie_break:  OrderStamp,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOrderingKey {
    pub priority:       PriorityPoints,
    pub topology_epoch: RouteEpoch,
    pub tie_break:      OrderStamp,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Shared router-owned cadence input for one engine progress step.
///
/// The router or host decides when a tick happens and which merged world
/// observation is authoritative for that step. Engines consume this context
/// but do not invent their own ambient topology input.
pub struct RoutingTickContext {
    pub topology: Observation<Configuration>,
}

impl RoutingTickContext {
    #[must_use]
    pub fn new(topology: Observation<Configuration>) -> Self {
        Self { topology }
    }
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RoutingTickChange {
    NoChange,
    PrivateStateUpdated,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Shared report shape returned by one engine progress step.
///
/// This gives the router a minimal, engine-neutral summary of whether the
/// tick changed engine-private state while keeping engine internals private.
pub struct RoutingTickOutcome {
    pub topology_epoch: RouteEpoch,
    pub change:         RoutingTickChange,
}

// Lexicographic: priority first, then epoch, then deterministic tie-break.
impl Ord for RouteOrderingKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.priority, self.topology_epoch, self.tie_break).cmp(&(
            other.priority,
            other.topology_epoch,
            other.tie_break,
        ))
    }
}

impl PartialOrd for RouteOrderingKey {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[must_use_handle]
#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Lease-based route ownership. Prevents ambient mutable access.
/// A bare RouteId is a weak reference; a lease is the strong one.
pub struct RouteLease {
    pub owner_node_id: NodeId,
    pub lease_epoch:   RouteEpoch,
    pub valid_for:     TimeWindow,
}

impl RouteLease {
    #[must_use]
    pub fn is_valid_at(&self, tick: Tick) -> bool {
        self.valid_for.contains(tick)
    }

    pub fn ensure_valid_at(&self, tick: Tick) -> Result<(), RouteRuntimeError> {
        if self.is_valid_at(tick) {
            return Ok(());
        }

        Err(RouteRuntimeError::LeaseExpired)
    }
}

#[must_use_handle]
#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Canonical handle issued only after route installation has materially
/// succeeded.
pub struct RouteHandle {
    pub route_id:             RouteId,
    pub topology_epoch:       RouteEpoch,
    pub materialized_at_tick: Tick,
    pub publication_id:       PublicationId,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteMaterializationProof {
    pub route_id:             RouteId,
    pub topology_epoch:       RouteEpoch,
    pub materialized_at_tick: Tick,
    pub publication_id:       PublicationId,
    pub witness:              Fact<RouteWitness>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Router-owned canonical route identity passed to a routing engine during
/// realization.
///
/// Routing engines must realize admitted routes under this canonical identity
/// instead of minting a competing handle or lease of their own.
pub struct RouteMaterializationInput {
    pub handle:    RouteHandle,
    pub admission: RouteAdmission,
    pub lease:     RouteLease,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Engine-owned installation result returned after realizing a canonical route.
///
/// This keeps router-owned identity separate from engine-owned runtime state
/// and proof artifacts.
pub struct RouteInstallation {
    pub materialization_proof: RouteMaterializationProof,
    pub last_lifecycle_event:  RouteLifecycleEvent,
    pub health:                RouteHealth,
    pub progress:              RouteProgressContract,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Explicit route budget envelope. Absence is not used to encode bound
/// semantics.
pub struct RouteCost {
    pub message_count_max:        Limit<u32>,
    pub byte_count_max:           Limit<ByteCount>,
    pub hop_count:                u8,
    pub repair_attempt_count_max: Limit<u32>,
    pub hold_bytes_reserved:      Limit<ByteCount>,
    /// Upper bound in deterministic abstract work steps, not host CPU time.
    pub work_step_count_max:      Limit<u32>,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteLifecycleEvent {
    Activated,
    Repaired,
    Replaced,
    HandedOff,
    EnteredPartitionMode,
    RecoveredFromPartition,
    Expired,
    Teardown,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOperationInstance {
    pub operation_id:   crate::RouteOperationId,
    pub route_binding:  RouteBinding,
    pub service_kind:   crate::RouteServiceKind,
    pub issued_at_tick: Tick,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteBinding {
    Unbound,
    Bound(RouteId),
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCommitment {
    pub commitment_id: RouteCommitmentId,
    pub operation_id:  crate::RouteOperationId,
    pub route_binding: RouteBinding,
    pub owner_node_id: NodeId,
    pub deadline_tick: Tick,
    pub retry_policy:  TimeoutPolicy,
    pub resolution:    RouteCommitmentResolution,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteCommitmentResolution {
    Pending,
    Blocked,
    Succeeded,
    Failed(RouteCommitmentFailure),
    TimedOut,
    Cancelled,
    Invalidated(RouteInvalidationReason),
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteCommitmentFailure {
    CapabilityRejected,
    BackendUnavailable,
    BudgetExceeded,
    InvalidInput,
    TransportRejected,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteInvalidationReason {
    OwnershipTransferred,
    TopologySuperseded,
    LeaseExpired,
    EvidenceWithdrawn,
}

#[must_use_handle]
#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Explicit transfer of route ownership. Modeled as a move, not shared access.
pub struct RouteSemanticHandoff {
    pub route_id:      RouteId,
    pub from_node_id:  NodeId,
    pub to_node_id:    NodeId,
    pub handoff_epoch: RouteEpoch,
    pub receipt_id:    ReceiptId,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteProgressContract {
    pub productive_step_count_max: Limit<u32>,
    pub total_step_count_max:      Limit<u32>,
    pub last_progress_at_tick:     Tick,
    pub state:                     RouteProgressState,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteProgressState {
    Pending,
    Blocked,
    Degraded,
    TimedOut,
    Satisfied,
    Failed,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Router-owned canonical identity and admission record for one live route.
pub struct MaterializedRouteIdentity {
    pub handle:                RouteHandle,
    pub materialization_proof: RouteMaterializationProof,
    pub admission:             RouteAdmission,
    pub lease:                 RouteLease,
}

impl MaterializedRouteIdentity {
    pub fn ensure_lease_valid_at(&self, tick: Tick) -> Result<(), RouteRuntimeError> {
        self.lease.ensure_valid_at(tick)
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Engine-observed mutable runtime state for one live route.
pub struct RouteRuntimeState {
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub health:               RouteHealth,
    pub progress:             RouteProgressContract,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedRoute {
    pub identity: MaterializedRouteIdentity,
    pub runtime:  RouteRuntimeState,
}

impl MaterializedRoute {
    #[must_use]
    /// Build the canonical route view from router-owned identity plus the
    /// engine-owned installation result.
    pub fn from_installation(
        input: RouteMaterializationInput,
        installation: RouteInstallation,
    ) -> Self {
        assert_eq!(
            input.admission.admission_check.decision,
            AdmissionDecision::Admissible,
            "route installation requires an admissible control-plane decision"
        );
        assert!(
            input.admission.summary.protection
                >= input.admission.objective.protection_floor,
            "route installation must satisfy the objective protection floor"
        );
        assert_eq!(
            input.handle.route_id, input.admission.route_id,
            "route materialization input must use one canonical route id"
        );
        assert_eq!(
            input.handle.route_id, installation.materialization_proof.route_id,
            "route installation proof must match the canonical route id"
        );
        assert_eq!(
            input.handle.topology_epoch,
            installation.materialization_proof.topology_epoch,
            "route installation proof must match the canonical topology epoch"
        );
        assert_eq!(
            input.handle.publication_id,
            installation.materialization_proof.publication_id,
            "route installation proof must match the canonical publication id"
        );
        Self {
            identity: MaterializedRouteIdentity {
                handle:                input.handle,
                materialization_proof: installation.materialization_proof,
                admission:             input.admission,
                lease:                 input.lease,
            },
            runtime:  RouteRuntimeState {
                last_lifecycle_event: installation.last_lifecycle_event,
                health:               installation.health,
                progress:             installation.progress,
            },
        }
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteHealth {
    pub reachability_state:        ReachabilityState,
    pub stability_score:           HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
    pub last_validated_at_tick:    Tick,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum ReachabilityState {
    Unknown,
    Reachable,
    Unreachable,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteMaintenanceTrigger {
    LinkDegraded,
    CapacityExceeded,
    LeaseExpiring,
    EpochAdvanced,
    PolicyShift,
    RouteExpired,
    PartitionDetected,
    AntiEntropyRequired,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Routing engine returns this from maintenance so the control plane can
/// preserve the semantic payload of the decision rather than collapsing it to a
/// single enum.
pub struct RouteMaintenanceResult {
    pub event:   RouteLifecycleEvent,
    pub outcome: RouteMaintenanceOutcome,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteMaintenanceOutcome {
    Continued,
    Repaired,
    ReplacementRequired {
        trigger: RouteMaintenanceTrigger,
    },
    HandedOff(RouteSemanticHandoff),
    HoldFallback {
        trigger:               RouteMaintenanceTrigger,
        retained_object_count: u32,
    },
    Failed(RouteMaintenanceFailure),
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteMaintenanceFailure {
    LostReachability,
    CapacityExceeded,
    LeaseExpired,
    BackendUnavailable,
    InvalidEvidence,
    PolicyRejected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_ordering_key_is_total() {
        let low = RouteOrderingKey {
            priority:       crate::PriorityPoints(1),
            topology_epoch: crate::RouteEpoch(2),
            tie_break:      crate::OrderStamp(3),
        };
        let high = RouteOrderingKey {
            priority:       crate::PriorityPoints(2),
            topology_epoch: crate::RouteEpoch(2),
            tie_break:      crate::OrderStamp(3),
        };

        assert!(low < high);
    }
}
