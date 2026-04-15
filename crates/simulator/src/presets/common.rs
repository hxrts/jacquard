use super::*;

pub(super) fn line_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(NODE_A, node_a), (NODE_B, node_b), (NODE_C, node_c)]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_C), topology::link(3).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

pub(super) fn fanout_service_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(NODE_A, node_a), (NODE_B, node_b), (NODE_C, node_c)]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_A), topology::link(1).build()),
                ((NODE_A, NODE_C), topology::link(3).build()),
                ((NODE_C, NODE_A), topology::link(1).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

pub(super) fn bidirectional_line_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(NODE_A, node_a), (NODE_B, node_b), (NODE_C, node_c)]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_A), topology::link(1).build()),
                ((NODE_B, NODE_C), topology::link(3).build()),
                ((NODE_C, NODE_B), topology::link(2).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

pub(super) fn dual_pair_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
    node_d: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([
                (NODE_A, node_a),
                (NODE_B, node_b),
                (NODE_C, node_c),
                (NODE_D, node_d),
            ]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_A), topology::link(1).build()),
                ((NODE_C, NODE_D), topology::link(4).build()),
                ((NODE_D, NODE_C), topology::link(3).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

pub(super) fn ring_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
    node_d: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([
                (NODE_A, node_a),
                (NODE_B, node_b),
                (NODE_C, node_c),
                (NODE_D, node_d),
            ]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_A), topology::link(1).build()),
                ((NODE_B, NODE_C), topology::link(3).build()),
                ((NODE_C, NODE_B), topology::link(2).build()),
                ((NODE_C, NODE_D), topology::link(4).build()),
                ((NODE_D, NODE_C), topology::link(3).build()),
                ((NODE_D, NODE_A), topology::link(1).build()),
                ((NODE_A, NODE_D), topology::link(4).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

pub(super) fn connected_objective(destination: jacquard_core::NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub(super) fn service_objective(service_id: ServiceId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Service(service_id),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub(super) fn best_effort_connected_profile() -> jacquard_core::SelectedRoutingParameters {
    jacquard_core::SelectedRoutingParameters {
        selected_protection: jacquard_core::RouteProtectionClass::LinkProtected,
        selected_connectivity: jacquard_core::ConnectivityPosture {
            repair: jacquard_core::RouteRepairClass::BestEffort,
            partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: jacquard_core::OperatingMode::DenseInteractive,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}
