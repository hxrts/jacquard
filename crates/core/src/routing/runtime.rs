//! Router-owned route identity, installation results, and runtime lifecycle
//! objects for the full route control plane.
//!
//! This module defines the most comprehensive portion of the shared routing
//! boundary: everything from how the router canonically identifies a route
//! through how it tracks maintenance, commitments, leases, and round outcomes.
//!
//! Identity and proof: [`RouteIdentityStamp`] (the four-field canonical key),
//! [`RouteHandle`] (the must-use-handle capability token), [`RouteLease`]
//! (lease-based ownership with validity checking),
//! [`RouteMaterializationProof`] (engine echo-back of the stamp plus a witness
//! fact), [`RouteMaterializationInput`] (the canonical identity the router
//! passes to an engine during realization), and [`RouteInstallation`] (the
//! engine's installation result).
//!
//! Lifecycle and maintenance: [`MaterializedRoute`], [`PublishedRouteRecord`],
//! [`RouteRuntimeState`], [`RouteLifecycleEvent`], [`RouteMaintenanceResult`],
//! [`RouteMaintenanceOutcome`], [`RouteMaintenanceFailure`], and
//! [`RouteSemanticHandoff`]. Commitments: [`RouteCommitment`],
//! [`RouteCommitmentResolution`], [`RouteCommitmentFailure`],
//! [`RouteInvalidationReason`]. Tick machinery: [`RoutingTickContext`],
//! [`RoutingTickOutcome`], [`RoutingTickHint`], [`RouterRoundOutcome`].

use jacquard_macros::{must_use_handle, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    AdmissionDecision, ByteCount, Configuration, Fact, HealthScore, HoldItemCount, Limit, NodeId,
    Observation, OrderStamp, PenaltyPoints, PriorityPoints, PublicationId, ReceiptId,
    RouteAdmission, RouteCommitmentId, RouteEpoch, RouteId, RouteRuntimeError, RouteWitness, Tick,
    TimeWindow, TimeoutPolicy,
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

impl RoutingTickOutcome {
    #[must_use]
    pub fn no_change_for(tick: &RoutingTickContext) -> Self {
        Self {
            topology_epoch: tick.topology.value.epoch,
            change: RoutingTickChange::NoChange,
            next_tick_hint: RoutingTickHint::HostDefault,
        }
    }
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutingTickChange {
    NoChange,
    PrivateStateUpdated,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Advisory scheduling pressure reported by one routing engine tick.
///
/// The engine may indicate when more proactive work would be useful, but the
/// host/router still owns final scheduling.
pub enum RoutingTickHint {
    HostDefault,
    Immediate,
    WithinTicks(Tick),
}

impl RoutingTickHint {
    #[must_use]
    pub fn more_urgent(self, other: Self) -> Self {
        match (self, other) {
            (Self::Immediate, _) | (_, Self::Immediate) => Self::Immediate,
            (Self::WithinTicks(left), Self::WithinTicks(right)) => {
                Self::WithinTicks(std::cmp::min(left, right))
            }
            (Self::WithinTicks(left), Self::HostDefault)
            | (Self::HostDefault, Self::WithinTicks(left)) => Self::WithinTicks(left),
            (Self::HostDefault, Self::HostDefault) => Self::HostDefault,
        }
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Shared report shape returned by one engine progress step.
///
/// This gives the router a minimal, engine-neutral summary of whether the
/// tick changed engine-private state while keeping engine internals private.
/// Engines may also report scheduling pressure for their next proactive work
/// step without taking ownership of the global clock.
pub struct RoutingTickOutcome {
    pub topology_epoch: RouteEpoch,
    pub change: RoutingTickChange,
    pub next_tick_hint: RoutingTickHint,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Canonical route mutation published by the router after consuming typed
/// engine evidence.
pub enum RouterCanonicalMutation {
    None,
    RouteReplaced {
        previous_route_id: RouteId,
        route: Box<MaterializedRoute>,
    },
    LeaseTransferred {
        route_id: RouteId,
        handoff: RouteSemanticHandoff,
        lease: RouteLease,
    },
    RouteExpired {
        route_id: RouteId,
    },
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Control-plane result after one maintenance step. The engine result remains
/// intact, while any canonical mutation is surfaced explicitly at the router
/// layer.
pub struct RouterMaintenanceOutcome {
    pub engine_result: RouteMaintenanceResult,
    pub canonical_mutation: RouterCanonicalMutation,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Router-owned round outcome that keeps engine-private change reporting
/// separate from canonical route mutation.
pub struct RouterRoundOutcome {
    pub topology_epoch: RouteEpoch,
    pub engine_change: RoutingTickChange,
    pub next_round_hint: RoutingTickHint,
    pub canonical_mutation: RouterCanonicalMutation,
}

// Manual Ord (not derived) keeps the priority-first, then-epoch,
// then-tie-break lexicographic order explicit and independent of field
// declaration order.
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

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// The four fields that canonically identify one materialized route.
///
/// Shared by `RouteHandle` (the capability token) and
/// `RouteMaterializationProof` (the engine echo-back). Having a single named
/// type lets `from_installation` compare the two with one assertion instead of
/// three and makes "this belongs to the same route" explicit at the type level.
pub struct RouteIdentityStamp {
    pub route_id: RouteId,
    pub topology_epoch: RouteEpoch,
    pub materialized_at_tick: Tick,
    pub publication_id: PublicationId,
}

#[must_use_handle]
#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Canonical handle issued only after route installation has materially
/// succeeded. Wraps a `RouteIdentityStamp` to add the must-use-handle
/// capability semantics without duplicating the identity fields.
pub struct RouteHandle {
    pub stamp: RouteIdentityStamp,
}

impl RouteHandle {
    #[must_use]
    pub fn route_id(&self) -> &RouteId {
        &self.stamp.route_id
    }

    #[must_use]
    pub fn topology_epoch(&self) -> RouteEpoch {
        self.stamp.topology_epoch
    }

    #[must_use]
    pub fn materialized_at_tick(&self) -> Tick {
        self.stamp.materialized_at_tick
    }

    #[must_use]
    pub fn publication_id(&self) -> &PublicationId {
        &self.stamp.publication_id
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Engine echo-back of the canonical identity stamp plus a witness.
/// The stamp must equal the `RouteHandle` stamp that was passed to the engine
/// during `materialize_route`; `from_installation` asserts this in one
/// comparison.
pub struct RouteMaterializationProof {
    pub stamp: RouteIdentityStamp,
    pub witness: Fact<RouteWitness>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Router-owned canonical route identity passed to a routing engine during
/// realization.
///
/// Routing engines must realize admitted routes under this canonical identity
/// instead of minting a competing handle or lease of their own.
pub struct RouteMaterializationInput {
    pub handle: RouteHandle,
    pub admission: RouteAdmission,
    pub lease: RouteLease,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Engine-owned installation result returned after realizing a canonical route.
///
/// This keeps router-owned identity separate from engine-owned runtime state
/// and proof artifacts.
pub struct RouteInstallation {
    pub materialization_proof: RouteMaterializationProof,
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub health: RouteHealth,
    pub progress: RouteProgressContract,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Explicit route budget envelope. Absence is not used to encode bound
/// semantics.
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
/// Router-owned canonical identity and admission record for one live route.
///
/// `stamp` is the single canonical source of route identity. `proof` carries
/// its own stamp copy (which must equal `stamp`) plus the admission witness.
/// `admission` holds the engine's decision artifacts without restating identity
/// fields that belong to `stamp`.
pub struct PublishedRouteRecord {
    pub stamp: RouteIdentityStamp,
    pub proof: RouteMaterializationProof,
    pub admission: RouteAdmission,
    pub lease: RouteLease,
}

impl PublishedRouteRecord {
    pub fn ensure_lease_valid_at(&self, tick: Tick) -> Result<(), RouteRuntimeError> {
        self.lease.ensure_valid_at(tick)
    }

    pub fn route_id(&self) -> &RouteId {
        &self.stamp.route_id
    }

    pub fn topology_epoch(&self) -> RouteEpoch {
        self.stamp.topology_epoch
    }

    pub fn materialized_at_tick(&self) -> Tick {
        self.stamp.materialized_at_tick
    }

    pub fn publication_id(&self) -> &PublicationId {
        &self.stamp.publication_id
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Engine-observed mutable runtime state for one live route.
pub struct RouteRuntimeState {
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub health: RouteHealth,
    pub progress: RouteProgressContract,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedRoute {
    /// Router-owned canonical published record for this live route.
    pub identity: PublishedRouteRecord,
    /// Engine-observed mutable runtime state for the same published route.
    pub runtime: RouteRuntimeState,
}

impl MaterializedRoute {
    #[must_use]
    /// Build the canonical route view from router-owned identity plus the
    /// engine-owned installation result.
    pub fn from_installation(
        input: RouteMaterializationInput,
        installation: RouteInstallation,
    ) -> Self {
        // Invariant assertions: from_installation is a trusted router-internal
        // path. These are programming-error guards, not recoverable failures.
        assert_eq!(
            input.admission.admission_check.decision,
            AdmissionDecision::Admissible,
            "route installation requires an admissible control-plane decision"
        );
        assert!(
            input.admission.summary.protection >= input.admission.objective.protection_floor,
            "route installation must satisfy the objective protection floor"
        );
        assert_eq!(
            input.handle.stamp, installation.materialization_proof.stamp,
            "route installation proof stamp must match the canonical handle stamp"
        );
        Self {
            identity: PublishedRouteRecord {
                stamp: input.handle.stamp,
                proof: installation.materialization_proof,
                admission: input.admission,
                lease: input.lease,
            },
            runtime: RouteRuntimeState {
                last_lifecycle_event: installation.last_lifecycle_event,
                health: installation.health,
                progress: installation.progress,
            },
        }
    }
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
    Unknown,
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
/// Routing engine returns this from maintenance so the control plane can
/// preserve the semantic payload of the decision rather than collapsing it to a
/// single enum.
pub struct RouteMaintenanceResult {
    pub event: RouteLifecycleEvent,
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
        trigger: RouteMaintenanceTrigger,
        retained_object_count: HoldItemCount,
    },
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
    use crate::{
        Configuration, Environment, Observation, OriginAuthenticationClass, RoutingEvidenceClass,
    };

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

    #[test]
    fn no_change_tick_defaults_to_host_cadence() {
        let tick = crate::RoutingTickContext::new(Observation {
            value: Configuration {
                epoch: crate::RouteEpoch(3),
                nodes: std::collections::BTreeMap::new(),
                links: std::collections::BTreeMap::new(),
                environment: Environment {
                    reachable_neighbor_count: 0,
                    churn_permille: crate::RatioPermille(0),
                    contention_permille: crate::RatioPermille(0),
                },
            },
            source_class: crate::FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::AdmissionWitnessed,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(5),
        });

        let outcome = RoutingTickOutcome::no_change_for(&tick);
        assert_eq!(outcome.change, RoutingTickChange::NoChange);
        assert_eq!(outcome.next_tick_hint, RoutingTickHint::HostDefault);
    }

    #[test]
    fn immediate_tick_hint_dominates_merge() {
        let merged = RoutingTickHint::WithinTicks(Tick(4)).more_urgent(RoutingTickHint::Immediate);
        assert_eq!(merged, RoutingTickHint::Immediate);
    }

    #[test]
    fn within_ticks_merge_chooses_smaller_horizon() {
        let merged = RoutingTickHint::WithinTicks(Tick(9))
            .more_urgent(RoutingTickHint::WithinTicks(Tick(3)));
        assert_eq!(merged, RoutingTickHint::WithinTicks(Tick(3)));
    }
}
