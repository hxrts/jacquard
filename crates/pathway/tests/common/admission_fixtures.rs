//! Admission-test fixtures for route objectives, profiles, summaries, and
//! costs.
//!
//! These builders produce the typed inputs consumed by the admission tests in
//! `admission.rs`. Each function is pure and deterministic: same arguments
//! always yield the same struct value. `neutral_assumptions` provides the
//! benign baseline assumption set. `objective_with_floor` produces a
//! `RoutingObjective` with a configurable protection floor. `profile_with`
//! selects repair and partition classes independently. `summary_with`
//! constructs a `RouteSummary` keyed to the pathway engine. `unit_route_cost`
//! returns a minimal single-hop cost bound used to drive rejection branches
//! that depend on cost constraints.

use jacquard_core::{
    AdmissionAssumptions, AdversaryRegime, Belief, ClaimStrength, ConnectivityPosture,
    ConnectivityRegime, DestinationId, DiversityFloor, Estimate, FailureModelClass,
    HoldFallbackPolicy, Limit, MessageFlowAssumptionClass, NodeDensityClass, NodeId,
    RatioPermille, RouteCost, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteServiceKind, RouteSummary, RoutingObjective,
    RuntimeEnvelopeClass, SelectedRoutingParameters, Tick, TimeWindow,
};
use jacquard_pathway::PATHWAY_ENGINE_ID;

pub fn neutral_assumptions() -> AdmissionAssumptions {
    AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::PerRouteSequenced,
        failure_model: FailureModelClass::Benign,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: NodeDensityClass::Sparse,
        connectivity_regime: ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::BenignUntrusted,
        claim_strength: ClaimStrength::ConservativeUnderProfile,
    }
}

pub fn objective_with_floor(floor: RouteProtectionClass) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(NodeId([3; 32])),
        service_kind: RouteServiceKind::Move,
        target_protection: floor,
        protection_floor: floor,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Unbounded,
        protection_priority: jacquard_core::PriorityPoints(0),
        connectivity_priority: jacquard_core::PriorityPoints(0),
    }
}

pub fn profile_with(
    repair: RouteRepairClass,
    partition: RoutePartitionClass,
) -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture { repair, partition },
        deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy:
            jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

pub fn summary_with(
    protection: RouteProtectionClass,
    repair: RouteRepairClass,
    partition: RoutePartitionClass,
) -> RouteSummary {
    RouteSummary {
        engine: PATHWAY_ENGINE_ID,
        protection,
        connectivity: ConnectivityPosture { repair, partition },
        protocol_mix: Vec::new(),
        hop_count_hint: Belief::Estimated(Estimate {
            value: 1_u8,
            confidence_permille: RatioPermille(1000),
            updated_at_tick: Tick(0),
        }),
        valid_for: TimeWindow::new(Tick(0), Tick(100)).unwrap(),
    }
}

pub fn unit_route_cost() -> RouteCost {
    RouteCost {
        message_count_max: Limit::Bounded(1),
        byte_count_max: Limit::Bounded(jacquard_core::ByteCount(1024)),
        hop_count: 1,
        repair_attempt_count_max: Limit::Bounded(1),
        hold_bytes_reserved: Limit::Bounded(jacquard_core::ByteCount(0)),
        work_step_count_max: Limit::Bounded(2),
    }
}
