//! Shared fixtures for the mesh integration tests.
//!
//! Each `tests/*.rs` file is its own crate, so this module is included via
//! `mod common;` in the test files that need a configured `MeshEngine`.

#![allow(dead_code)]
#![allow(unreachable_pub)]

use std::collections::BTreeMap;

use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine, MESH_ENGINE_ID};
use jacquard_traits::{
    effect_handler,
    jacquard_core::{
        AdaptiveRoutingProfile, Belief, Blake3Digest, BleDeviceId, BleProfileId, ByteCount,
        Configuration, ContentId, ControllerId, DeploymentProfile, DestinationId, EndpointAddress,
        Environment, Estimate, FactSourceClass, HoldFallbackPolicy, InformationSetSummary,
        InformationSummaryEncoding, Limit, Link, LinkEndpoint, LinkRuntimeState, LinkState, Node,
        NodeId, NodeProfile, NodeRelayBudget, NodeState, Observation, OrderStamp,
        OriginAuthenticationClass, RetentionError, RouteConnectivityProfile, RouteEpoch,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
        RouteServiceKind, RoutingEngineFallbackPolicy, RoutingEvidenceClass, RoutingObjective,
        ServiceDescriptor, ServiceScope, Tick, TimeWindow, TransportError, TransportObservation,
        TransportProtocol,
    },
    Blake3Hashing, MeshTransport, OrderEffects, RetentionStore, RouteEventLogEffects,
    StorageEffects, TimeEffects,
};

#[derive(Default)]
pub struct TestRuntimeEffects {
    pub now: Tick,
    pub next_order: u64,
    pub storage: BTreeMap<Vec<u8>, Vec<u8>>,
    pub events: Vec<jacquard_traits::jacquard_core::RouteEventStamped>,
}

#[effect_handler]
impl TimeEffects for TestRuntimeEffects {
    fn now_tick(&self) -> Tick {
        self.now
    }
}

#[effect_handler]
impl OrderEffects for TestRuntimeEffects {
    fn next_order_stamp(&mut self) -> OrderStamp {
        self.next_order += 1;
        OrderStamp(self.next_order)
    }
}

#[effect_handler]
impl StorageEffects for TestRuntimeEffects {
    fn load_bytes(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, jacquard_traits::jacquard_core::StorageError> {
        Ok(self.storage.get(key).cloned())
    }

    fn store_bytes(
        &mut self,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), jacquard_traits::jacquard_core::StorageError> {
        self.storage.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn remove_bytes(
        &mut self,
        key: &[u8],
    ) -> Result<(), jacquard_traits::jacquard_core::StorageError> {
        self.storage.remove(key);
        Ok(())
    }
}

#[effect_handler]
impl RouteEventLogEffects for TestRuntimeEffects {
    fn record_route_event(
        &mut self,
        event: jacquard_traits::jacquard_core::RouteEventStamped,
    ) -> Result<(), jacquard_traits::jacquard_core::RouteEventLogError> {
        self.events.push(event);
        Ok(())
    }
}

#[derive(Default)]
pub struct TestTransport {
    pub sent_frames: Vec<(LinkEndpoint, Vec<u8>)>,
    pub observations: Vec<TransportObservation>,
}

impl MeshTransport for TestTransport {
    fn transport_id(&self) -> TransportProtocol {
        TransportProtocol::BleGatt
    }

    fn send_frame(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.sent_frames.push((endpoint.clone(), payload.to_vec()));
        Ok(())
    }

    fn poll_observations(&mut self) -> Result<Vec<TransportObservation>, TransportError> {
        Ok(std::mem::take(&mut self.observations))
    }
}

#[derive(Default)]
pub struct TestRetentionStore {
    pub payloads: BTreeMap<ContentId<Blake3Digest>, Vec<u8>>,
}

impl RetentionStore for TestRetentionStore {
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError> {
        self.payloads.insert(object_id, payload);
        Ok(())
    }

    fn take_retained_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RetentionError> {
        Ok(self.payloads.remove(object_id))
    }

    fn contains_retained_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, RetentionError> {
        Ok(self.payloads.contains_key(object_id))
    }
}

pub fn mesh_connectivity(partition: RoutePartitionClass) -> RouteConnectivityProfile {
    RouteConnectivityProfile {
        repair: RouteRepairClass::Repairable,
        partition,
    }
}

pub fn objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: mesh_connectivity(RoutePartitionClass::PartitionTolerant),
        hold_fallback_policy: HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Bounded(jacquard_traits::jacquard_core::DurationMs(250)),
        protection_priority: jacquard_traits::jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_traits::jacquard_core::PriorityPoints(20),
    }
}

pub fn objective_with_floor(
    destination: DestinationId,
    target: RouteProtectionClass,
    floor: RouteProtectionClass,
) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: target,
        protection_floor: floor,
        target_connectivity: mesh_connectivity(RoutePartitionClass::PartitionTolerant),
        hold_fallback_policy: HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Bounded(jacquard_traits::jacquard_core::DurationMs(250)),
        protection_priority: jacquard_traits::jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_traits::jacquard_core::PriorityPoints(20),
    }
}

pub fn profile() -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: mesh_connectivity(RoutePartitionClass::PartitionTolerant),
        deployment_profile: DeploymentProfile::FieldPartitionTolerant,
        diversity_floor: 1,
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

pub fn profile_with_connectivity(
    repair: RouteRepairClass,
    partition: RoutePartitionClass,
) -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: RouteConnectivityProfile { repair, partition },
        deployment_profile: DeploymentProfile::FieldPartitionTolerant,
        diversity_floor: 1,
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

pub fn ble_endpoint(device_byte: u8) -> LinkEndpoint {
    LinkEndpoint {
        protocol: TransportProtocol::BleGatt,
        address: EndpointAddress::Ble {
            device_id: BleDeviceId(vec![device_byte]),
            profile_id: BleProfileId([device_byte; 16]),
        },
        mtu_bytes: ByteCount(256),
    }
}

pub fn route_capable_services(
    node_id: NodeId,
    controller_id: ControllerId,
) -> Vec<ServiceDescriptor> {
    let valid_for = TimeWindow::new(Tick(1), Tick(20)).expect("valid service window");
    [
        RouteServiceKind::Discover,
        RouteServiceKind::Move,
        RouteServiceKind::Hold,
    ]
    .into_iter()
    .map(|service_kind| ServiceDescriptor {
        provider_node_id: node_id,
        controller_id,
        service_kind,
        endpoints: vec![ble_endpoint(node_id.0[0])],
        routing_engines: vec![MESH_ENGINE_ID],
        scope: ServiceScope::Discovery(jacquard_traits::jacquard_core::DiscoveryScopeId([7; 16])),
        valid_for,
        capacity: Belief::Estimated(Estimate {
            value: jacquard_traits::jacquard_core::CapacityHint {
                saturation_permille: jacquard_traits::jacquard_core::RatioPermille(100),
                repair_capacity: Belief::Estimated(Estimate {
                    value: 4,
                    confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                    updated_at_tick: Tick(1),
                }),
                hold_capacity_bytes: Belief::Estimated(Estimate {
                    value: ByteCount(4096),
                    confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                    updated_at_tick: Tick(1),
                }),
            },
            confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
            updated_at_tick: Tick(1),
        }),
    })
    .collect()
}

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
            relay_work_budget_max: 10,
            maintenance_work_budget_max: 10,
            hold_item_count_max: 8,
            hold_capacity_bytes_max: ByteCount(8192),
        },
        state: NodeState {
            relay_budget: Belief::Estimated(Estimate {
                value: NodeRelayBudget {
                    relay_work_budget: Belief::Estimated(Estimate {
                        value: 8,
                        confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    utilization_permille: jacquard_traits::jacquard_core::RatioPermille(100),
                    retention_horizon_ms: Belief::Estimated(Estimate {
                        value: jacquard_traits::jacquard_core::DurationMs(500),
                        confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                },
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            available_connection_count: Belief::Estimated(Estimate {
                value: 4,
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            hold_capacity_available_bytes: Belief::Estimated(Estimate {
                value: ByteCount(4096),
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            information_summary: Belief::Estimated(Estimate {
                value: InformationSetSummary {
                    summary_encoding: InformationSummaryEncoding::BloomFilter,
                    item_count: Belief::Estimated(Estimate {
                        value: 4,
                        confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    byte_count: Belief::Estimated(Estimate {
                        value: ByteCount(2048),
                        confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    false_positive_permille: Belief::Estimated(Estimate {
                        value: jacquard_traits::jacquard_core::RatioPermille(10),
                        confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                },
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
        },
    }
}

pub fn link(device_byte: u8, confidence: u16) -> Link {
    Link {
        endpoint: ble_endpoint(device_byte),
        state: LinkState {
            state: LinkRuntimeState::Active,
            median_rtt_ms: jacquard_traits::jacquard_core::DurationMs(40),
            transfer_rate_bytes_per_sec: Belief::Estimated(Estimate {
                value: 2048,
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            stability_horizon_ms: Belief::Estimated(Estimate {
                value: jacquard_traits::jacquard_core::DurationMs(500),
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            loss_permille: jacquard_traits::jacquard_core::RatioPermille(50),
            delivery_confidence_permille: Belief::Estimated(Estimate {
                value: jacquard_traits::jacquard_core::RatioPermille(confidence),
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            symmetry_permille: Belief::Estimated(Estimate {
                value: jacquard_traits::jacquard_core::RatioPermille(900),
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
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
                churn_permille: jacquard_traits::jacquard_core::RatioPermille(150),
                contention_permille: jacquard_traits::jacquard_core::RatioPermille(120),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

pub type TestEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    TestTransport,
    TestRetentionStore,
    TestRuntimeEffects,
    Blake3Hashing,
>;

pub fn build_engine() -> TestEngine {
    MeshEngine::without_committee_selector(
        NodeId([1; 32]),
        DeterministicMeshTopologyModel::new(),
        TestTransport::default(),
        TestRetentionStore::default(),
        TestRuntimeEffects {
            now: Tick(2),
            ..Default::default()
        },
        Blake3Hashing,
    )
}

pub fn build_engine_at_tick(now: Tick) -> TestEngine {
    MeshEngine::without_committee_selector(
        NodeId([1; 32]),
        DeterministicMeshTopologyModel::new(),
        TestTransport::default(),
        TestRetentionStore::default(),
        TestRuntimeEffects {
            now,
            ..Default::default()
        },
        Blake3Hashing,
    )
}
