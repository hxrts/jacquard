//! Admission-test fixtures for route objectives, profiles, summaries, and
//! costs.

use jacquard_core::{
    SelectedRoutingParameters, AdmissionAssumptions, AdversaryRegime, Belief,
    ClaimStrength, ConnectivityRegime, DestinationId, Estimate, FailureModelClass,
    HoldFallbackPolicy, Limit, MessageFlowAssumptionClass, NodeDensityClass, NodeId,
    RatioPermille, ConnectivityPosture, RouteCost, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteServiceKind, RouteSummary,
    RoutingObjective, RuntimeEnvelopeClass, Tick, TimeWindow,
};
use jacquard_mesh::MESH_ENGINE_ID;

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
        diversity_floor: 1,
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
        engine: MESH_ENGINE_ID,
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
