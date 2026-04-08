//! Topology, node, link, and service fixtures shared across router tests.

use std::collections::BTreeMap;

use jacquard_core::{
    Belief, BleDeviceId, BleProfileId, ByteCount, ConnectivityPosture, ControllerId,
    DestinationId, DiscoveryScopeId, DiversityFloor, DurationMs, EndpointAddress,
    Environment, Estimate, FactSourceClass, HealthScore, HoldItemCount,
    IdentityAssuranceClass, InformationSetSummary, InformationSummaryEncoding, Link,
    LinkEndpoint, LinkRuntimeState, LinkState, MaintenanceWorkBudget, Node, NodeId,
    NodeProfile, NodeRelayBudget, NodeState, Observation, OriginAuthenticationClass,
    PriorityPoints, RatioPermille, RelayWorkBudget, RepairCapacitySlots,
    RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy, RouteServiceKind,
    RoutingEngineFallbackPolicy, RoutingEvidenceClass, RoutingObjective,
    RoutingPolicyInputs, SelectedRoutingParameters, ServiceDescriptor, ServiceScope,
    Tick, TimeWindow, TransportProtocol,
};
use jacquard_mesh::MESH_ENGINE_ID;

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
                ((LOCAL_NODE_ID, PEER_NODE_ID), link(2, 950)),
                ((PEER_NODE_ID, FAR_NODE_ID), link(3, 875)),
                ((LOCAL_NODE_ID, BRIDGE_NODE_ID), link(4, 925)),
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

fn route_capable_node(node_byte: u8) -> Node {
    let node_id = NodeId([node_byte; 32]);
    let controller_id = ControllerId([node_byte; 32]);

    Node {
        controller_id,
        profile: route_capable_profile(node_byte, node_id, controller_id),
        state: node_state(),
    }
}

fn route_capable_profile(
    node_byte: u8,
    node_id: NodeId,
    controller_id: ControllerId,
) -> NodeProfile {
    NodeProfile {
        services: route_capable_services(node_id, controller_id),
        endpoints: vec![ble_endpoint(node_byte)],
        connection_count_max: 8,
        neighbor_state_count_max: 8,
        simultaneous_transfer_count_max: 4,
        active_route_count_max: 4,
        relay_work_budget_max: RelayWorkBudget(10),
        maintenance_work_budget_max: MaintenanceWorkBudget(10),
        hold_item_count_max: HoldItemCount(8),
        hold_capacity_bytes_max: ByteCount(8192),
    }
}

fn node_state() -> NodeState {
    NodeState {
        relay_budget: relay_budget(),
        available_connection_count: estimate(4),
        hold_capacity_available_bytes: estimate(ByteCount(4096)),
        information_summary: estimate(InformationSetSummary {
            summary_encoding: InformationSummaryEncoding::BloomFilter,
            item_count: estimate(HoldItemCount(4)),
            byte_count: estimate(ByteCount(2048)),
            false_positive_permille: estimate(RatioPermille(10)),
        }),
    }
}

fn relay_budget() -> Belief<NodeRelayBudget> {
    Belief::Estimated(Estimate {
        value: NodeRelayBudget {
            relay_work_budget: estimate(RelayWorkBudget(8)),
            utilization_permille: RatioPermille(100),
            retention_horizon_ms: estimate(DurationMs(500)),
        },
        confidence_permille: RatioPermille(1000),
        updated_at_tick: Tick(1),
    })
}

fn estimate<T>(value: T) -> Belief<T> {
    Belief::Estimated(Estimate {
        value,
        confidence_permille: RatioPermille(1000),
        updated_at_tick: Tick(1),
    })
}

fn route_capable_services(
    node_id: NodeId,
    controller_id: ControllerId,
) -> Vec<ServiceDescriptor> {
    let valid_for = TimeWindow::new(Tick(1), Tick(20)).expect("valid service window");
    [RouteServiceKind::Discover, RouteServiceKind::Move, RouteServiceKind::Hold]
        .into_iter()
        .map(|service_kind| ServiceDescriptor {
            provider_node_id: node_id,
            controller_id,
            service_kind,
            endpoints: vec![ble_endpoint(node_id.0[0])],
            routing_engines: vec![MESH_ENGINE_ID],
            scope: ServiceScope::Discovery(DiscoveryScopeId([7; 16])),
            valid_for,
            capacity: Belief::Estimated(Estimate {
                value: jacquard_core::CapacityHint {
                    saturation_permille: RatioPermille(100),
                    repair_capacity_slots: Belief::Estimated(Estimate {
                        value: RepairCapacitySlots(4),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    hold_capacity_bytes: Belief::Estimated(Estimate {
                        value: ByteCount(4096),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
        })
        .collect()
}

fn ble_endpoint(device_byte: u8) -> LinkEndpoint {
    LinkEndpoint {
        protocol: TransportProtocol::BleGatt,
        address: EndpointAddress::Ble {
            device_id: BleDeviceId(vec![device_byte]),
            profile_id: BleProfileId([device_byte; 16]),
        },
        mtu_bytes: ByteCount(256),
    }
}

fn link(device_byte: u8, confidence: u16) -> Link {
    Link {
        endpoint: ble_endpoint(device_byte),
        profile: jacquard_core::LinkProfile {
            latency_floor_ms: DurationMs(8),
            repair_capability: jacquard_core::RepairCapability::TransportRetransmit,
            partition_recovery: jacquard_core::PartitionRecoveryClass::LocalReconnect,
        },
        state: LinkState {
            state: LinkRuntimeState::Active,
            median_rtt_ms: Belief::Estimated(Estimate {
                value: DurationMs(40),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            transfer_rate_bytes_per_sec: Belief::Estimated(Estimate {
                value: 2048,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            stability_horizon_ms: Belief::Estimated(Estimate {
                value: DurationMs(500),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            loss_permille: RatioPermille(50),
            delivery_confidence_permille: Belief::Estimated(Estimate {
                value: RatioPermille(confidence),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            symmetry_permille: Belief::Estimated(Estimate {
                value: RatioPermille(900),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
        },
    }
}
