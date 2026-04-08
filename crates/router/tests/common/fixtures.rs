//! Topology, node, link, and service fixtures shared across router tests.

use std::collections::BTreeMap;

use jacquard_core::{
    ConnectivityPosture, DestinationId, DiversityFloor, DurationMs, Environment,
    FactSourceClass, HealthScore, IdentityAssuranceClass, NodeId, Observation,
    OriginAuthenticationClass, PriorityPoints, RatioPermille, RouteProtectionClass,
    RouteRepairClass, RouteReplacementPolicy, RouteServiceKind,
    RoutingEngineFallbackPolicy, RoutingEvidenceClass, RoutingObjective,
    RoutingPolicyInputs, SelectedRoutingParameters, Tick,
};
use jacquard_reference_client::topology::{active_link, route_capable_node};

pub(crate) const LOCAL_NODE_ID: NodeId = NodeId([1; 32]);
pub(crate) const PEER_NODE_ID: NodeId = NodeId([2; 32]);
pub(crate) const FAR_NODE_ID: NodeId = NodeId([3; 32]);
pub(crate) const BRIDGE_NODE_ID: NodeId = NodeId([4; 32]);

pub(crate) fn sample_policy_inputs(
    topology: &Observation<jacquard_core::Configuration>,
) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: Observation {
            value: topology.value.nodes[&LOCAL_NODE_ID].clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        local_environment: Observation {
            value: topology.value.environment.clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        routing_engine_count: 1,
        median_rtt_ms: DurationMs(40),
        loss_permille: RatioPermille(50),
        partition_risk_permille: RatioPermille(150),
        adversary_pressure_permille: RatioPermille(25),
        identity_assurance: IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: HealthScore(900),
    }
}

pub(crate) fn profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: jacquard_core::RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

pub(crate) fn objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: jacquard_core::RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub(crate) fn sample_configuration() -> Observation<jacquard_core::Configuration> {
    Observation {
        value: jacquard_core::Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: BTreeMap::from([
                (LOCAL_NODE_ID, route_capable_node(1)),
                (PEER_NODE_ID, route_capable_node(2)),
                (FAR_NODE_ID, route_capable_node(3)),
                (BRIDGE_NODE_ID, route_capable_node(4)),
            ]),
            links: BTreeMap::from([
                ((LOCAL_NODE_ID, PEER_NODE_ID), active_link(2, 950)),
                ((PEER_NODE_ID, FAR_NODE_ID), active_link(3, 875)),
                ((LOCAL_NODE_ID, BRIDGE_NODE_ID), active_link(4, 925)),
            ]),
            environment: Environment {
                reachable_neighbor_count: 3,
                churn_permille: RatioPermille(150),
                contention_permille: RatioPermille(120),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}
