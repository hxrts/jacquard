use std::collections::BTreeMap;

use jacquard_mesh::{
    DeterministicCommitteeSelector, DeterministicMeshTopologyModel, MeshEngine, MESH_ENGINE_ID,
};
use jacquard_traits::{
    effect_handler,
    jacquard_core::{
        AdaptiveRoutingProfile, Belief, Blake3Digest, BleDeviceId, BleProfileId, ByteCount,
        Configuration, ContentId, ControllerId, DeploymentProfile, DestinationId, EndpointAddress,
        Environment, Estimate, FactSourceClass, HoldFallbackPolicy, InformationSetSummary,
        InformationSummaryEncoding, Limit, Link, LinkEndpoint, LinkRuntimeState, LinkState, Node,
        NodeId, NodeProfile, NodeRelayBudget, NodeState, Observation, OrderStamp,
        OriginAuthenticationClass, PublicationId, ReachabilityState, RetentionError,
        RouteConnectivityProfile, RouteEpoch, RouteHandle, RouteLease, RouteMaintenanceOutcome,
        RouteMaintenanceTrigger, RouteMaterializationInput, RoutePartitionClass,
        RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy, RouteServiceKind,
        RoutingEngineFallbackPolicy, RoutingEvidenceClass, RoutingObjective, ServiceDescriptor,
        ServiceId, ServiceScope, Tick, TimeWindow, TransportError, TransportObservation,
        TransportProtocol,
    },
    Blake3Hashing, CommitteeSelector, MeshRoutingEngine, MeshTopologyModel, MeshTransport,
    OrderEffects, RetentionStore, RouteEventLogEffects, RoutingEngine, RoutingEnginePlanner,
    StorageEffects, TimeEffects,
};

#[derive(Default)]
struct TestRuntimeEffects {
    now: Tick,
    next_order: u64,
    storage: BTreeMap<Vec<u8>, Vec<u8>>,
    events: Vec<jacquard_traits::jacquard_core::RouteEventStamped>,
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
struct TestTransport {
    sent_frames: Vec<(LinkEndpoint, Vec<u8>)>,
    observations: Vec<TransportObservation>,
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
struct TestRetentionStore {
    payloads: BTreeMap<ContentId<Blake3Digest>, Vec<u8>>,
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

fn mesh_connectivity(partition: RoutePartitionClass) -> RouteConnectivityProfile {
    RouteConnectivityProfile {
        repair: RouteRepairClass::Repairable,
        partition,
    }
}

fn objective(destination: DestinationId) -> RoutingObjective {
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

fn profile() -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: mesh_connectivity(RoutePartitionClass::PartitionTolerant),
        deployment_profile: DeploymentProfile::FieldPartitionTolerant,
        diversity_floor: 1,
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
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

fn route_capable_services(node_id: NodeId, controller_id: ControllerId) -> Vec<ServiceDescriptor> {
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

fn node(node_byte: u8) -> Node {
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

fn link(device_byte: u8, confidence: u16) -> Link {
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

fn sample_configuration() -> Observation<Configuration> {
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

fn build_engine() -> MeshEngine<
    DeterministicMeshTopologyModel,
    TestTransport,
    TestRetentionStore,
    TestRuntimeEffects,
    Blake3Hashing,
> {
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

// Two planning passes over the same topology snapshot must return identical
// candidate lists in the same order, with the expected candidate count.
#[test]
fn candidate_ordering_is_deterministic_for_the_same_topology_snapshot() {
    let engine = build_engine();
    let topology = sample_configuration();
    let objective = objective(DestinationId::Service(ServiceId(vec![1, 2, 3])));
    let profile = profile();

    let first = engine.candidate_routes(&objective, &profile, &topology);
    let second = engine.candidate_routes(&objective, &profile, &topology);

    assert_eq!(first, second);
    assert_eq!(first.len(), 3);
}

// Repeated admission checks on the same candidate must agree, and the resulting
// admission must carry the topology epoch and the mesh engine id in its witness
// and summary.
#[test]
fn mesh_admission_emits_stable_check_and_witness_values() {
    let engine = build_engine();
    let topology = sample_configuration();
    let objective = objective(DestinationId::Node(NodeId([3; 32])));
    let profile = profile();

    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("node destination should yield a candidate");
    let first_check = engine
        .check_candidate(&objective, &profile, &candidate, &topology)
        .expect("candidate check");
    let second_check = engine
        .check_candidate(&objective, &profile, &candidate, &topology)
        .expect("candidate check");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("route admission");

    assert_eq!(first_check, second_check);
    assert_eq!(admission.admission_check, first_check);
    assert_eq!(admission.witness.topology_epoch, topology.value.epoch);
    assert_eq!(admission.summary.engine, MESH_ENGINE_ID);
}

// The deterministic committee selector must produce the same Some/None result
// across calls on the same inputs, confirming both determinism and the optional
// return shape required by `CommitteeSelector`.
#[test]
fn committee_selection_is_optional_and_deterministic() {
    let selector = DeterministicCommitteeSelector::new(NodeId([1; 32]));
    let topology = sample_configuration();
    let objective = objective(DestinationId::Service(ServiceId(vec![9, 9])));
    let profile = profile();

    let first = selector
        .select_committee(&objective, &profile, &topology)
        .expect("selector result");
    let second = selector
        .select_committee(&objective, &profile, &topology)
        .expect("selector result");

    assert_eq!(first, second);
    assert!(first.is_some());
}

// The deterministic topology model must surface mesh-private intrinsic node
// state, per-protocol medium counts, and a non-trivial neighborhood density
// estimate from a shared `Configuration`.
#[test]
fn topology_model_exposes_medium_and_node_intrinsic_support() {
    let topology = sample_configuration();
    let model = DeterministicMeshTopologyModel::new();

    let intrinsic = model
        .node_intrinsic_state(&NodeId([1; 32]), &topology.value)
        .expect("local node intrinsic state");
    let medium = model.medium_state(&NodeId([1; 32]), &topology.value);
    let neighborhood = model
        .neighborhood_estimate(&NodeId([1; 32]), &topology.value)
        .expect("neighborhood estimate");

    assert_eq!(intrinsic.available_connection_count, 4);
    assert_eq!(
        medium.protocol_counts.get(&TransportProtocol::BleGatt),
        Some(&2)
    );
    assert!(neighborhood.density_score.0 > 0);
}

// End-to-end check that an admitted route forwards payloads, retains and
// recovers deferred-delivery payloads through the retention store, repairs on
// `LinkDegraded`, falls back to hold on `PartitionDetected`, and reports a
// typed lease-expiry failure once the lease window has elapsed.
#[test]
fn active_routes_respect_repairs_partitions_and_retention_boundaries() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let objective = objective(DestinationId::Node(NodeId([3; 32])));
    let profile = profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("admission");
    let input = RouteMaterializationInput {
        handle: RouteHandle {
            route_id: admission.route_id,
            topology_epoch: topology.value.epoch,
            materialized_at_tick: Tick(2),
            publication_id: PublicationId([7; 16]),
        },
        admission: admission.clone(),
        lease: RouteLease {
            owner_node_id: NodeId([1; 32]),
            lease_epoch: topology.value.epoch,
            valid_for: TimeWindow::new(Tick(2), Tick(10)).expect("valid lease"),
        },
    };
    let installation = engine
        .materialize_route(input.clone())
        .expect("materialization");
    let mut runtime = jacquard_traits::jacquard_core::RouteRuntimeState {
        last_lifecycle_event: installation.last_lifecycle_event,
        health: installation.health,
        progress: installation.progress,
    };

    engine
        .forward_payload(&admission.route_id, b"mesh-payload")
        .expect("forwarding");
    assert_eq!(engine.transport().sent_frames.len(), 1);

    let retained = engine
        .retain_for_route(&admission.route_id, b"partition-buffer")
        .expect("retain payload");
    assert!(engine
        .retention_store()
        .contains_retained_payload(&retained)
        .expect("retention lookup"));
    assert_eq!(
        engine
            .recover_retained_payload(&admission.route_id, &retained)
            .expect("recover payload"),
        Some(b"partition-buffer".to_vec())
    );

    let repaired = engine
        .maintain_route(
            &jacquard_traits::jacquard_core::MaterializedRouteIdentity {
                handle: input.handle.clone(),
                materialization_proof: installation.materialization_proof.clone(),
                admission: input.admission.clone(),
                lease: input.lease.clone(),
            },
            &mut runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("repair");
    assert_eq!(repaired.outcome, RouteMaintenanceOutcome::Repaired);

    let hold_fallback = engine
        .maintain_route(
            &jacquard_traits::jacquard_core::MaterializedRouteIdentity {
                handle: input.handle.clone(),
                materialization_proof: installation.materialization_proof.clone(),
                admission: input.admission.clone(),
                lease: input.lease.clone(),
            },
            &mut runtime,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("partition maintenance");
    assert_eq!(
        hold_fallback.outcome,
        RouteMaintenanceOutcome::HoldFallback {
            trigger: RouteMaintenanceTrigger::PartitionDetected,
        }
    );

    engine.runtime_effects_mut().now = Tick(12);
    let expired = engine
        .maintain_route(
            &jacquard_traits::jacquard_core::MaterializedRouteIdentity {
                handle: input.handle,
                materialization_proof: installation.materialization_proof,
                admission: input.admission,
                lease: input.lease,
            },
            &mut runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("lease expiry maintenance");
    assert_eq!(
        expired.outcome,
        RouteMaintenanceOutcome::Failed(
            jacquard_traits::jacquard_core::RouteMaintenanceFailure::LeaseExpired,
        )
    );
    assert_eq!(engine.runtime_effects().events.len(), 4);
    assert!(matches!(
        runtime.health.reachability_state,
        ReachabilityState::Reachable
    ));
}
