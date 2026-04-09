//! Deterministic domain data for the mesh integration tests.
//!
//! Every helper here is pure: same inputs always produce the same
//! `Node`, `Link`, or `Configuration`. The fixtures are tuned so the
//! standard `sample_configuration` produces a four-node graph with
//! three reachable peers and predictable candidate output.

use std::collections::BTreeMap;

use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_traits::jacquard_core::{
    ble_endpoint, Belief, ByteCount, Configuration, ControllerId, DiscoveryScopeId,
    DurationMs, Environment, Estimate, FactSourceClass, HoldItemCount,
    InformationSetSummary, InformationSummaryEncoding, Link, LinkRuntimeState,
    LinkState, MaintenanceWorkBudget, Node, NodeId, NodeProfile, NodeRelayBudget,
    NodeState, Observation, OriginAuthenticationClass, RatioPermille, RelayWorkBudget,
    RepairCapacitySlots, RouteEpoch, RouteServiceKind, RoutingEvidenceClass,
    ServiceDescriptor, ServiceScope, Tick, TimeWindow,
};

pub fn route_capable_services(
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
            routing_engines: vec![PATHWAY_ENGINE_ID],
            scope: ServiceScope::Discovery(DiscoveryScopeId([7; 16])),
            valid_for,
            capacity: Belief::Estimated(Estimate {
                value: jacquard_traits::jacquard_core::CapacityHint {
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

// long-block-exception: canonical route-capable node fixture.
pub fn node(node_byte: u8) -> Node {
    let node_id = NodeId([node_byte; 32]);
    let controller_id = ControllerId([node_byte; 32]);
    Node {
        controller_id,
        profile: NodeProfile {
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
        },
        state: NodeState {
            relay_budget: Belief::Estimated(Estimate {
                value: NodeRelayBudget {
                    relay_work_budget: Belief::Estimated(Estimate {
                        value: RelayWorkBudget(8),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    utilization_permille: RatioPermille(100),
                    retention_horizon_ms: Belief::Estimated(Estimate {
                        value: DurationMs(500),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            available_connection_count: Belief::Estimated(Estimate {
                value: 4,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            hold_capacity_available_bytes: Belief::Estimated(Estimate {
                value: ByteCount(4096),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            information_summary: Belief::Estimated(Estimate {
                value: InformationSetSummary {
                    summary_encoding: InformationSummaryEncoding::BloomFilter,
                    item_count: Belief::Estimated(Estimate {
                        value: HoldItemCount(4),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    byte_count: Belief::Estimated(Estimate {
                        value: ByteCount(2048),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    false_positive_permille: Belief::Estimated(Estimate {
                        value: RatioPermille(10),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
        },
    }
}

pub fn link(device_byte: u8, confidence: u16) -> Link {
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

pub fn sample_configuration() -> Observation<Configuration> {
    let local_node_id = NodeId([1; 32]);
    let node_two_id = NodeId([2; 32]);
    let node_three_id = NodeId([3; 32]);
    let node_four_id = NodeId([4; 32]);

    Observation {
        value: Configuration {
            epoch: RouteEpoch(2),
            nodes: BTreeMap::from([
                (local_node_id, node(1)),
                (node_two_id, node(2)),
                (node_three_id, node(3)),
                (node_four_id, node(4)),
            ]),
            links: BTreeMap::from([
                ((local_node_id, node_two_id), link(2, 950)),
                ((node_two_id, node_three_id), link(3, 875)),
                ((local_node_id, node_four_id), link(4, 925)),
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
