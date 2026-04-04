//! Installed route state, ordering, leases, transitions, and maintenance triggers.

use contour_macros::{must_use_handle, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    ByteCount, Fact, HealthScore, Limit, NodeId, OrderStamp, PenaltyPoints, PriorityPoints,
    PublicationId, ReceiptId, RouteAdmission, RouteCommitmentId, RouteEpoch, RouteId, RouteWitness,
    Tick, TimeWindow, TimeoutPolicy,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeterministicOrderKey<K> {
    pub stable_key: K,
    pub tie_break: OrderStamp,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOrderingKey {
    pub priority: PriorityPoints,
    pub topology_epoch: RouteEpoch,
    pub tie_break: OrderStamp,
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
    pub lease_epoch: RouteEpoch,
    pub valid_for: TimeWindow,
}

#[must_use_handle]
#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Canonical handle issued only after route installation has materially succeeded.
pub struct RouteHandle {
    pub route_id: RouteId,
    pub topology_epoch: RouteEpoch,
    pub materialized_at_tick: Tick,
    pub publication_id: PublicationId,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteMaterializationProof {
    pub route_id: RouteId,
    pub topology_epoch: RouteEpoch,
    pub materialized_at_tick: Tick,
    pub publication_id: PublicationId,
    pub witness: Fact<RouteWitness>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCost {
    pub message_count_max: Limit<u32>,
    pub byte_count_max: Limit<ByteCount>,
    pub hop_count: u8,
    pub repair_attempt_count_max: Limit<u32>,
    pub hold_bytes_reserved: Limit<ByteCount>,
    /// Upper bound in deterministic abstract work steps, not host CPU time.
    pub work_step_count_max: Limit<u32>,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteLifecycleEvent {
    Established,
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
    pub operation_id: crate::RouteOperationId,
    pub route_binding: RouteBinding,
    pub service_kind: crate::RouteServiceKind,
    pub issued_at_tick: Tick,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteBinding {
    Unbound,
    Bound(RouteId),
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCommitment {
    pub commitment_id: RouteCommitmentId,
    pub operation_id: crate::RouteOperationId,
    pub route_binding: RouteBinding,
    pub owner_node_id: NodeId,
    pub deadline_tick: Tick,
    pub retry_policy: TimeoutPolicy,
    pub resolution: RouteCommitmentResolution,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteCommitmentFailure {
    CapabilityRejected,
    BackendUnavailable,
    BudgetExceeded,
    InvalidInput,
    TransportRejected,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
    pub route_id: RouteId,
    pub from_node_id: NodeId,
    pub to_node_id: NodeId,
    pub handoff_epoch: RouteEpoch,
    pub receipt_id: ReceiptId,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteProgressContract {
    pub productive_step_count_max: Limit<u32>,
    pub total_step_count_max: Limit<u32>,
    pub last_progress_at_tick: Tick,
    pub state: RouteProgressState,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
pub struct InstalledRoute {
    pub handle: RouteHandle,
    pub materialization_proof: RouteMaterializationProof,
    pub admission: RouteAdmission,
    pub lease: RouteLease,
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub health: RouteHealth,
    pub progress: RouteProgressContract,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteHealth {
    pub reachability_state: ReachabilityState,
    pub stability_score: HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
    pub last_validated_at_tick: Tick,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReachabilityState {
    Reachable,
    Unreachable,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
/// Family returns this from maintenance so the control plane can preserve the
/// semantic payload of the decision rather than collapsing it to a single enum.
pub struct RouteMaintenanceResult {
    pub event: RouteLifecycleEvent,
    pub outcome: RouteMaintenanceOutcome,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteMaintenanceOutcome {
    Continued,
    Repaired,
    ReplacementRequired { trigger: RouteMaintenanceTrigger },
    HandedOff(RouteSemanticHandoff),
    HoldFallback { trigger: RouteMaintenanceTrigger },
    Failed(RouteMaintenanceFailure),
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
            priority: crate::PriorityPoints(1),
            topology_epoch: crate::RouteEpoch(2),
            tie_break: crate::OrderStamp(3),
        };
        let high = RouteOrderingKey {
            priority: crate::PriorityPoints(2),
            topology_epoch: crate::RouteEpoch(2),
            tie_break: crate::OrderStamp(3),
        };

        assert!(low < high);
    }
}
