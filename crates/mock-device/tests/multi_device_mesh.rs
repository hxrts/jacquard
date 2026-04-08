use std::collections::BTreeMap;

use jacquard_core::{
    AdaptiveRoutingProfile, Belief, BleDeviceId, BleProfileId, ByteCount,
    Configuration, ControllerId, DeploymentProfile, DestinationId, DiscoveryScopeId,
    DurationMs, EndpointAddress, Environment, Estimate, FactSourceClass,
    InformationSetSummary, InformationSummaryEncoding, Link, LinkEndpoint,
    LinkRuntimeState, LinkState, Node, NodeId, NodeProfile, NodeRelayBudget, NodeState,
    Observation, OriginAuthenticationClass, PriorityPoints, RatioPermille,
    RouteConnectivityProfile, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteReplacementPolicy, RouteServiceKind,
    RoutingEngineFallbackPolicy, RoutingEvidenceClass, RoutingObjective,
    ServiceDescriptor, ServiceScope, Tick, TimeWindow, TransportProtocol,
};
use jacquard_mock_device::{
    build_mock_mesh_device, build_mock_mesh_device_with_profile,
};
use jacquard_mock_transport::SharedInMemoryMeshNetwork;
use jacquard_traits::{Router, RoutingControlPlane, RoutingDataPlane};

const NODE_A: NodeId = NodeId([1; 32]);
const NODE_B: NodeId = NodeId([2; 32]);
const NODE_C: NodeId = NodeId([3; 32]);
const NODE_D: NodeId = NodeId([4; 32]);

#[test]
fn multi_device_mesh_routing_uses_shared_router_transport_and_device_boundaries() {
    let topology = sample_configuration();
    let network = SharedInMemoryMeshNetwork::default();
    let (mut device_a, mut device_b, mut device_c) =
        build_device_triplet(&topology, network);

    let route_a_to_c = Router::activate_route(
        device_a.router_mut(),
        objective(DestinationId::Node(NODE_C)),
    )
    .expect("device A route activation");
    let route_b_to_c = Router::activate_route(
        device_b.router_mut(),
        objective(DestinationId::Node(NODE_C)),
    )
    .expect("device B route activation");

    let payload = b"mesh-e2e";
    forward_and_assert_ingress(
        &mut device_a,
        &route_a_to_c.identity.handle.route_id,
        &mut device_b,
        payload,
        topology.value.epoch,
        "device A forwards toward B",
        "device B ingress tick",
        "device B transport summary",
    );
    forward_and_assert_ingress(
        &mut device_b,
        &route_b_to_c.identity.handle.route_id,
        &mut device_c,
        payload,
        topology.value.epoch,
        "device B forwards toward C",
        "device C ingress tick",
        "device C transport summary",
    );
}

fn build_device_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryMeshNetwork,
) -> (
    jacquard_mock_device::MockMeshDevice,
    jacquard_mock_device::MockMeshDevice,
    jacquard_mock_device::MockMeshDevice,
) {
    let device_a =
        build_mock_mesh_device(NODE_A, topology.clone(), network.clone(), Tick(2));
    let device_b = build_mock_mesh_device_with_profile(
        NODE_B,
        topology.clone(),
        network.clone(),
        Tick(2),
        relay_profile(),
    );
    let device_c = build_mock_mesh_device(NODE_C, topology.clone(), network, Tick(2));
    (device_a, device_b, device_c)
}

fn forward_and_assert_ingress(
    sender: &mut jacquard_mock_device::MockMeshDevice,
    route_id: &jacquard_core::RouteId,
    receiver: &mut jacquard_mock_device::MockMeshDevice,
    payload: &[u8],
    expected_epoch: jacquard_core::RouteEpoch,
    forward_context: &str,
    tick_context: &str,
    summary_context: &str,
) {
    sender
        .router_mut()
        .forward_payload(route_id, payload)
        .expect(forward_context);
    let outcome = receiver
        .router_mut()
        .anti_entropy_tick()
        .expect(tick_context);

    assert_eq!(outcome.topology_epoch, expected_epoch);
    assert!(
        receiver
            .router()
            .engine()
            .transport_observation_summary()
            .expect(summary_context)
            .payload_event_count
            > 0
    );
}

fn relay_profile() -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection:            RouteProtectionClass::LinkProtected,
        selected_connectivity:          RouteConnectivityProfile {
            repair:    RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile:             DeploymentProfile::DenseInteractive,
        diversity_floor:                1,
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy:       RouteReplacementPolicy::Allowed,
    }
}

fn objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: RouteConnectivityProfile {
            repair:    RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

fn sample_configuration() -> Observation<Configuration> {
    Observation {
        value:                 Configuration {
            epoch:       jacquard_core::RouteEpoch(2),
            nodes:       BTreeMap::from([
                (NODE_A, route_capable_node(1)),
                (NODE_B, route_capable_node(2)),
                (NODE_C, route_capable_node(3)),
                (NODE_D, route_capable_node(4)),
            ]),
            links:       BTreeMap::from([
                ((NODE_A, NODE_B), link(2, 950)),
                ((NODE_B, NODE_C), link(3, 875)),
                ((NODE_A, NODE_D), link(4, 925)),
                ((NODE_B, NODE_D), link(4, 900)),
            ]),
            environment: Environment {
                reachable_neighbor_count: 3,
                churn_permille:           RatioPermille(150),
                contention_permille:      RatioPermille(120),
            },
        },
        source_class:          FactSourceClass::Local,
        evidence_class:        RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick:      Tick(2),
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
        relay_work_budget_max: 10,
        maintenance_work_budget_max: 10,
        hold_item_count_max: 8,
        hold_capacity_bytes_max: ByteCount(8192),
    }
}

fn node_state() -> NodeState {
    NodeState {
        relay_budget:                  relay_budget(),
        available_connection_count:    estimate(4),
        hold_capacity_available_bytes: estimate(ByteCount(4096)),
        information_summary:           estimate(InformationSetSummary {
            summary_encoding:        InformationSummaryEncoding::BloomFilter,
            item_count:              estimate(4),
            byte_count:              estimate(ByteCount(2048)),
            false_positive_permille: estimate(RatioPermille(10)),
        }),
    }
}

fn relay_budget() -> Belief<NodeRelayBudget> {
    Belief::Estimated(Estimate {
        value:               NodeRelayBudget {
            relay_work_budget:    estimate(8),
            utilization_permille: RatioPermille(100),
            retention_horizon_ms: estimate(DurationMs(500)),
        },
        confidence_permille: RatioPermille(1000),
        updated_at_tick:     Tick(1),
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
            routing_engines: vec![jacquard_mesh::MESH_ENGINE_ID],
            scope: ServiceScope::Discovery(DiscoveryScopeId([7; 16])),
            valid_for,
            capacity: Belief::Estimated(Estimate {
                value:               jacquard_core::CapacityHint {
                    saturation_permille: RatioPermille(100),
                    repair_capacity:     Belief::Estimated(Estimate {
                        value:               4,
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick:     Tick(1),
                    }),
                    hold_capacity_bytes: Belief::Estimated(Estimate {
                        value:               ByteCount(4096),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick:     Tick(1),
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
        })
        .collect()
}

fn ble_endpoint(device_byte: u8) -> LinkEndpoint {
    LinkEndpoint {
        protocol:  TransportProtocol::BleGatt,
        address:   EndpointAddress::Ble {
            device_id:  BleDeviceId(vec![device_byte]),
            profile_id: BleProfileId([device_byte; 16]),
        },
        mtu_bytes: ByteCount(256),
    }
}

fn link(device_byte: u8, confidence: u16) -> Link {
    Link {
        endpoint: ble_endpoint(device_byte),
        state:    LinkState {
            state: LinkRuntimeState::Active,
            median_rtt_ms: DurationMs(40),
            transfer_rate_bytes_per_sec: Belief::Estimated(Estimate {
                value:               2048,
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
            stability_horizon_ms: Belief::Estimated(Estimate {
                value:               DurationMs(500),
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
            loss_permille: RatioPermille(50),
            delivery_confidence_permille: Belief::Estimated(Estimate {
                value:               RatioPermille(confidence),
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
            symmetry_permille: Belief::Estimated(Estimate {
                value:               RatioPermille(900),
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
        },
    }
}
