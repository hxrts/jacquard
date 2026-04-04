//! Installed route state, ordering, leases, transitions, and maintenance triggers.

use serde::{Deserialize, Serialize};

use crate::{
    HealthScore, Limit, NodeId, OrderStamp, PenaltyPoints, PriorityPoints, RouteAdmission,
    RouteEpoch, RouteId, Tick, TimeoutPolicy,
};

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Lease-based route ownership. Prevents ambient mutable access.
/// A bare RouteId is a weak reference; a lease is the strong one.
pub struct RouteLease {
    pub owner_node_id: NodeId,
    pub lease_epoch: RouteEpoch,
    pub leased_at: Tick,
    pub expires_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCost {
    pub message_count_max: Limit<u32>,
    pub byte_count_max: Limit<u64>,
    pub hop_count: u8,
    pub repair_attempt_count_max: Limit<u32>,
    pub hold_bytes_reserved: Limit<u64>,
    pub cpu_work_units_max: Limit<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteTransition {
    Established,
    Repaired,
    Replaced,
    HandedOff,
    EnteredPartitionMode,
    RecoveredFromPartition,
    Expired,
    Teardown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOperationInstance {
    pub operation_id: crate::RouteOperationId,
    pub route_binding: RouteBinding,
    pub service_family: crate::ServiceFamily,
    pub issued_at: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteBinding {
    Unbound,
    Bound(RouteId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOutstandingEffect {
    pub operation_id: crate::RouteOperationId,
    pub owner_node_id: NodeId,
    pub deadline: Tick,
    pub retry_policy: TimeoutPolicy,
    pub state: RouteEffectState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteEffectState {
    Pending,
    Blocked,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Explicit transfer of route ownership. Modeled as a move, not shared access.
pub struct RouteSemanticHandoff {
    pub route_id: RouteId,
    pub from_node_id: NodeId,
    pub to_node_id: NodeId,
    pub handoff_epoch: RouteEpoch,
    pub receipt_id: [u8; 16],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteProgressContract {
    pub productive_step_count_max: Limit<u32>,
    pub total_step_count_max: Limit<u32>,
    pub last_progress_at: Tick,
    pub state: RouteProgressState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteProgressState {
    Pending,
    Blocked,
    Degraded,
    TimedOut,
    Satisfied,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledRoute {
    pub admission: RouteAdmission,
    pub lease: RouteLease,
    pub current_transition: RouteTransition,
    pub health: RouteHealth,
    pub progress: RouteProgressContract,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteHealth {
    pub reachability_state: ReachabilityState,
    pub stability_score: HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
    pub last_validated_at: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReachabilityState {
    Reachable,
    Unreachable,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Family returns this from maintenance. ReplaceRoute escalates to the top-level router.
pub enum RouteMaintenanceDisposition {
    Continue,
    Repaired,
    ReplaceRoute,
    HoldFallback,
    Fail,
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
