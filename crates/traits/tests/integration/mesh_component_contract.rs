use std::collections::BTreeMap;

use jacquard_traits::{
    jacquard_core::{
        Belief, Blake3Digest, ByteCount, Configuration, ContentId, ControllerId, DurationMs,
        Environment, Fact, InformationSetSummary, Link, LinkEndpoint, LinkRuntimeState, LinkState,
        MaterializedRouteIdentity, Node, NodeId, NodeProfile, NodeRelayBudget, NodeState,
        PublicationId, RatioPermille, RetentionError, RouteAdmission, RouteAdmissionCheck,
        RouteBinding, RouteCommitment, RouteCommitmentId, RouteCommitmentResolution,
        RouteConnectivityProfile, RouteCost, RouteEpoch, RouteHealth, RouteId, RouteInstallation,
        RouteLifecycleEvent, RouteMaintenanceOutcome, RouteMaintenanceResult,
        RouteMaintenanceTrigger, RouteMaterializationInput, RouteMaterializationProof,
        RouteProtectionClass, RouteRuntimeState, RouteSummary, RouteWitness,
        RoutingEngineCapabilities, RoutingEngineId, ServiceDescriptor, TransportError,
        TransportObservation, TransportProtocol,
    },
    EffectHandler, MeshRoutingEngine, MeshTopologyModel, MeshTransport, RetentionStore,
    RoutingEngine, RoutingEnginePlanner, TransportEffects,
};

struct StubTopologyModel;

impl MeshTopologyModel for StubTopologyModel {
    type PeerEstimate = ();
    type NeighborhoodEstimate = ();

    fn local_node(&self, local_node_id: &NodeId, configuration: &Configuration) -> Option<Node> {
        configuration.nodes.get(local_node_id).cloned()
    }

    fn neighboring_nodes(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<(NodeId, Node)> {
        configuration
            .links
            .iter()
            .filter_map(|((left, right), _)| {
                if left == local_node_id {
                    configuration
                        .nodes
                        .get(right)
                        .cloned()
                        .map(|node| (*right, node))
                } else if right == local_node_id {
                    configuration
                        .nodes
                        .get(left)
                        .cloned()
                        .map(|node| (*left, node))
                } else {
                    None
                }
            })
            .collect()
    }

    fn reachable_endpoints(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<LinkEndpoint> {
        self.neighboring_nodes(local_node_id, configuration)
            .into_iter()
            .flat_map(|(_, node)| node.profile.endpoints)
            .collect()
    }

    fn adjacent_links(&self, local_node_id: &NodeId, configuration: &Configuration) -> Vec<Link> {
        configuration
            .links
            .iter()
            .filter_map(|((left, right), link)| {
                if left == local_node_id || right == local_node_id {
                    Some(link.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn peer_estimate(
        &self,
        _local_node_id: &NodeId,
        _peer_node_id: &NodeId,
        _configuration: &Configuration,
    ) -> Option<Self::PeerEstimate> {
        Some(())
    }

    fn neighborhood_estimate(
        &self,
        _local_node_id: &NodeId,
        _configuration: &Configuration,
    ) -> Option<Self::NeighborhoodEstimate> {
        Some(())
    }
}

struct StubTransport {
    observations: Vec<TransportObservation>,
    sent_frames: Vec<Vec<u8>>,
}

impl MeshTransport for StubTransport {
    fn transport_id(&self) -> TransportProtocol {
        TransportProtocol::BleGatt
    }

    fn send_frame(
        &mut self,
        _endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.sent_frames.push(payload.to_vec());
        Ok(())
    }

    fn poll_observations(&mut self) -> Result<Vec<TransportObservation>, TransportError> {
        Ok(std::mem::take(&mut self.observations))
    }
}

struct StubRetentionStore {
    payloads: BTreeMap<ContentId<Blake3Digest>, Vec<u8>>,
}

impl RetentionStore for StubRetentionStore {
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

struct StubMeshFamily {
    topology: StubTopologyModel,
    transport: StubTransport,
    retention: StubRetentionStore,
    route: Option<jacquard_traits::jacquard_core::MaterializedRoute>,
}

impl RoutingEnginePlanner for StubMeshFamily {
    fn engine_id(&self) -> RoutingEngineId {
        RoutingEngineId::Mesh
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine: RoutingEngineId::Mesh,
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: RouteConnectivityProfile {
                repair: jacquard_traits::jacquard_core::RouteRepairClass::Repairable,
                partition: jacquard_traits::jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_traits::jacquard_core::RepairSupport::Supported,
            hold_support: jacquard_traits::jacquard_core::HoldSupport::Supported,
            decidable_admission: jacquard_traits::jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_traits::jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support:
                jacquard_traits::jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: jacquard_traits::jacquard_core::RouteShapeVisibility::Explicit,
        }
    }

    fn candidate_routes(
        &self,
        _objective: &jacquard_traits::jacquard_core::RoutingObjective,
        _profile: &jacquard_traits::jacquard_core::AdaptiveRoutingProfile,
        _topology: &jacquard_traits::jacquard_core::Observation<Configuration>,
    ) -> Vec<jacquard_traits::jacquard_core::RouteCandidate> {
        Vec::new()
    }

    fn check_candidate(
        &self,
        _objective: &jacquard_traits::jacquard_core::RoutingObjective,
        _profile: &jacquard_traits::jacquard_core::AdaptiveRoutingProfile,
        _candidate: &jacquard_traits::jacquard_core::RouteCandidate,
        _topology: &jacquard_traits::jacquard_core::Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, jacquard_traits::jacquard_core::RouteError> {
        Ok(RouteAdmissionCheck {
            decision: jacquard_traits::jacquard_core::AdmissionDecision::Admissible,
            profile: jacquard_traits::jacquard_core::AdmissionAssumptions {
                message_flow_assumption:
                    jacquard_traits::jacquard_core::MessageFlowAssumptionClass::BestEffort,
                failure_model: jacquard_traits::jacquard_core::FailureModelClass::Benign,
                runtime_envelope: jacquard_traits::jacquard_core::RuntimeEnvelopeClass::Canonical,
                node_density_class: jacquard_traits::jacquard_core::NodeDensityClass::Sparse,
                connectivity_regime: jacquard_traits::jacquard_core::ConnectivityRegime::Stable,
                adversary_regime: jacquard_traits::jacquard_core::AdversaryRegime::Cooperative,
                claim_strength: jacquard_traits::jacquard_core::ClaimStrength::InterfaceOnly,
            },
            productive_step_bound: jacquard_traits::jacquard_core::Limit::Bounded(1),
            total_step_bound: jacquard_traits::jacquard_core::Limit::Bounded(1),
            route_cost: RouteCost {
                message_count_max: jacquard_traits::jacquard_core::Limit::Bounded(1),
                byte_count_max: jacquard_traits::jacquard_core::Limit::Bounded(ByteCount(1)),
                hop_count: 1,
                repair_attempt_count_max: jacquard_traits::jacquard_core::Limit::Bounded(0),
                hold_bytes_reserved: jacquard_traits::jacquard_core::Limit::Bounded(ByteCount(0)),
                work_step_count_max: jacquard_traits::jacquard_core::Limit::Bounded(1),
            },
        })
    }

    fn admit_route(
        &self,
        objective: &jacquard_traits::jacquard_core::RoutingObjective,
        profile: &jacquard_traits::jacquard_core::AdaptiveRoutingProfile,
        _candidate: jacquard_traits::jacquard_core::RouteCandidate,
        _topology: &jacquard_traits::jacquard_core::Observation<Configuration>,
    ) -> Result<RouteAdmission, jacquard_traits::jacquard_core::RouteError> {
        Ok(sample_route_admission(objective.clone(), profile.clone()))
    }
}

impl RoutingEngine for StubMeshFamily {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, jacquard_traits::jacquard_core::RouteError> {
        let route = sample_materialized_route(&input);
        self.route = Some(route.clone());
        Ok(RouteInstallation {
            materialization_proof: route.identity.materialization_proof,
            last_lifecycle_event: route.runtime.last_lifecycle_event,
            health: route.runtime.health,
            progress: route.runtime.progress,
        })
    }

    fn route_commitments(
        &self,
        _route: &jacquard_traits::jacquard_core::MaterializedRoute,
    ) -> Vec<RouteCommitment> {
        vec![RouteCommitment {
            commitment_id: RouteCommitmentId([4; 16]),
            operation_id: jacquard_traits::jacquard_core::RouteOperationId([5; 16]),
            route_binding: RouteBinding::Bound(RouteId([6; 16])),
            owner_node_id: NodeId([1; 32]),
            deadline_tick: jacquard_traits::jacquard_core::Tick(4),
            retry_policy: jacquard_traits::jacquard_core::TimeoutPolicy {
                attempt_count_max: 1,
                initial_backoff_ms: DurationMs(1),
                backoff_multiplier_permille: RatioPermille(1000),
                backoff_ms_max: DurationMs(1),
                overall_timeout_ms: DurationMs(1),
            },
            resolution: RouteCommitmentResolution::Pending,
        }]
    }

    fn maintain_route(
        &mut self,
        _identity: &MaterializedRouteIdentity,
        runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, jacquard_traits::jacquard_core::RouteError> {
        runtime.last_lifecycle_event = RouteLifecycleEvent::Repaired;
        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Repaired,
            outcome: RouteMaintenanceOutcome::Repaired,
        })
    }

    fn teardown(&mut self, _route_id: &RouteId) {}
}

impl MeshRoutingEngine for StubMeshFamily {
    type TopologyModel = StubTopologyModel;
    type Transport = StubTransport;
    type Retention = StubRetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel {
        &self.topology
    }

    fn transport(&self) -> &Self::Transport {
        &self.transport
    }

    fn transport_mut(&mut self) -> &mut Self::Transport {
        &mut self.transport
    }

    fn retention_store(&self) -> &Self::Retention {
        &self.retention
    }

    fn retention_store_mut(&mut self) -> &mut Self::Retention {
        &mut self.retention
    }
}

fn sample_endpoint() -> LinkEndpoint {
    LinkEndpoint {
        protocol: TransportProtocol::BleGatt,
        address: jacquard_traits::jacquard_core::EndpointAddress::Ble {
            device_id: jacquard_traits::jacquard_core::BleDeviceId(vec![1]),
            profile_id: jacquard_traits::jacquard_core::BleProfileId([2; 16]),
        },
        mtu_bytes: ByteCount(512),
    }
}

fn sample_node(controller_seed: u8) -> Node {
    Node {
        controller_id: ControllerId([controller_seed; 32]),
        profile: NodeProfile {
            services: Vec::<ServiceDescriptor>::new(),
            endpoints: vec![sample_endpoint()],
            connection_count_max: 4,
            neighbor_state_count_max: 8,
            simultaneous_transfer_count_max: 2,
            active_route_count_max: 4,
            relay_work_budget_max: 16,
            maintenance_work_budget_max: 8,
            hold_item_count_max: 8,
            hold_capacity_bytes_max: ByteCount(1024),
        },
        state: NodeState {
            relay_budget: Belief::Estimated(jacquard_traits::jacquard_core::Estimate {
                value: NodeRelayBudget {
                    relay_work_budget: Belief::Absent,
                    utilization_permille: RatioPermille(0),
                    retention_horizon_ms: Belief::Absent,
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: jacquard_traits::jacquard_core::Tick(1),
            }),
            available_connection_count: Belief::Absent,
            hold_capacity_available_bytes: Belief::Absent,
            information_summary: Belief::Estimated(jacquard_traits::jacquard_core::Estimate {
                value: InformationSetSummary {
                    summary_encoding:
                        jacquard_traits::jacquard_core::InformationSummaryEncoding::BloomFilter,
                    item_count: Belief::Absent,
                    byte_count: Belief::Absent,
                    false_positive_permille: Belief::Absent,
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: jacquard_traits::jacquard_core::Tick(1),
            }),
        },
    }
}

fn sample_link() -> Link {
    Link {
        endpoint: sample_endpoint(),
        state: LinkState {
            state: LinkRuntimeState::Active,
            median_rtt_ms: DurationMs(5),
            transfer_rate_bytes_per_sec: Belief::Absent,
            stability_horizon_ms: Belief::Absent,
            loss_permille: RatioPermille(0),
            delivery_confidence_permille: Belief::Absent,
            symmetry_permille: Belief::Absent,
        },
    }
}

fn sample_configuration() -> Configuration {
    let local = NodeId([1; 32]);
    let remote = NodeId([2; 32]);

    let mut nodes = BTreeMap::new();
    nodes.insert(local, sample_node(9));
    nodes.insert(remote, sample_node(8));

    let mut links = BTreeMap::new();
    links.insert((local, remote), sample_link());

    Configuration {
        epoch: RouteEpoch(1),
        nodes,
        links,
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    }
}

fn sample_route_admission(
    objective: jacquard_traits::jacquard_core::RoutingObjective,
    profile: jacquard_traits::jacquard_core::AdaptiveRoutingProfile,
) -> RouteAdmission {
    RouteAdmission {
        route_id: RouteId([3; 16]),
        objective,
        profile,
        admission_check: RouteAdmissionCheck {
            decision: jacquard_traits::jacquard_core::AdmissionDecision::Admissible,
            profile: jacquard_traits::jacquard_core::AdmissionAssumptions {
                message_flow_assumption:
                    jacquard_traits::jacquard_core::MessageFlowAssumptionClass::BestEffort,
                failure_model: jacquard_traits::jacquard_core::FailureModelClass::Benign,
                runtime_envelope: jacquard_traits::jacquard_core::RuntimeEnvelopeClass::Canonical,
                node_density_class: jacquard_traits::jacquard_core::NodeDensityClass::Sparse,
                connectivity_regime: jacquard_traits::jacquard_core::ConnectivityRegime::Stable,
                adversary_regime: jacquard_traits::jacquard_core::AdversaryRegime::Cooperative,
                claim_strength: jacquard_traits::jacquard_core::ClaimStrength::InterfaceOnly,
            },
            productive_step_bound: jacquard_traits::jacquard_core::Limit::Bounded(1),
            total_step_bound: jacquard_traits::jacquard_core::Limit::Bounded(1),
            route_cost: RouteCost {
                message_count_max: jacquard_traits::jacquard_core::Limit::Bounded(1),
                byte_count_max: jacquard_traits::jacquard_core::Limit::Bounded(ByteCount(64)),
                hop_count: 1,
                repair_attempt_count_max: jacquard_traits::jacquard_core::Limit::Bounded(0),
                hold_bytes_reserved: jacquard_traits::jacquard_core::Limit::Bounded(ByteCount(0)),
                work_step_count_max: jacquard_traits::jacquard_core::Limit::Bounded(1),
            },
        },
        summary: RouteSummary {
            engine: RoutingEngineId::Mesh,
            protection: RouteProtectionClass::LinkProtected,
            connectivity: RouteConnectivityProfile {
                repair: jacquard_traits::jacquard_core::RouteRepairClass::Repairable,
                partition: jacquard_traits::jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            protocol_mix: vec![TransportProtocol::BleGatt],
            hop_count_hint: Belief::Absent,
            valid_for: jacquard_traits::jacquard_core::TimeWindow::new(
                jacquard_traits::jacquard_core::Tick(1),
                jacquard_traits::jacquard_core::Tick(2),
            )
            .expect("valid route summary window"),
        },
        witness: RouteWitness {
            objective_protection: RouteProtectionClass::LinkProtected,
            delivered_protection: RouteProtectionClass::LinkProtected,
            objective_connectivity: RouteConnectivityProfile {
                repair: jacquard_traits::jacquard_core::RouteRepairClass::Repairable,
                partition: jacquard_traits::jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            delivered_connectivity: RouteConnectivityProfile {
                repair: jacquard_traits::jacquard_core::RouteRepairClass::Repairable,
                partition: jacquard_traits::jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            admission_profile: jacquard_traits::jacquard_core::AdmissionAssumptions {
                message_flow_assumption:
                    jacquard_traits::jacquard_core::MessageFlowAssumptionClass::BestEffort,
                failure_model: jacquard_traits::jacquard_core::FailureModelClass::Benign,
                runtime_envelope: jacquard_traits::jacquard_core::RuntimeEnvelopeClass::Canonical,
                node_density_class: jacquard_traits::jacquard_core::NodeDensityClass::Sparse,
                connectivity_regime: jacquard_traits::jacquard_core::ConnectivityRegime::Stable,
                adversary_regime: jacquard_traits::jacquard_core::AdversaryRegime::Cooperative,
                claim_strength: jacquard_traits::jacquard_core::ClaimStrength::InterfaceOnly,
            },
            topology_epoch: RouteEpoch(1),
            degradation: jacquard_traits::jacquard_core::RouteDegradation::None,
        },
    }
}

fn sample_materialized_route(
    input: &RouteMaterializationInput,
) -> jacquard_traits::jacquard_core::MaterializedRoute {
    jacquard_traits::jacquard_core::MaterializedRoute::from_installation(
        input.clone(),
        RouteInstallation {
            materialization_proof: RouteMaterializationProof {
                route_id: input.admission.route_id,
                topology_epoch: RouteEpoch(1),
                materialized_at_tick: jacquard_traits::jacquard_core::Tick(1),
                publication_id: PublicationId([9; 16]),
                witness: Fact {
                    value: input.admission.witness.clone(),
                    basis: jacquard_traits::jacquard_core::FactBasis::Published,
                    established_at_tick: jacquard_traits::jacquard_core::Tick(1),
                },
            },
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: jacquard_traits::jacquard_core::ReachabilityState::Reachable,
                stability_score: jacquard_traits::jacquard_core::HealthScore(1000),
                congestion_penalty_points: jacquard_traits::jacquard_core::PenaltyPoints(0),
                last_validated_at_tick: jacquard_traits::jacquard_core::Tick(1),
            },
            progress: jacquard_traits::jacquard_core::RouteProgressContract {
                productive_step_count_max: jacquard_traits::jacquard_core::Limit::Bounded(1),
                total_step_count_max: jacquard_traits::jacquard_core::Limit::Bounded(1),
                last_progress_at_tick: jacquard_traits::jacquard_core::Tick(1),
                state: jacquard_traits::jacquard_core::RouteProgressState::Satisfied,
            },
        },
    )
}

#[test]
fn mesh_topology_model_is_read_only_over_configuration_inputs() {
    let local = NodeId([1; 32]);
    let configuration = sample_configuration();
    let model = StubTopologyModel;

    assert!(model.local_node(&local, &configuration).is_some());
    assert_eq!(model.neighboring_nodes(&local, &configuration).len(), 1);
    assert_eq!(model.reachable_endpoints(&local, &configuration).len(), 1);
    assert_eq!(model.adjacent_links(&local, &configuration).len(), 1);
}

#[test]
fn mesh_transport_carries_frames_without_interpreting_them() {
    let endpoint = sample_endpoint();
    let mut transport = StubTransport {
        observations: Vec::new(),
        sent_frames: Vec::new(),
    };

    transport
        .send_frame(&endpoint, b"frame")
        .expect("send frame");
    let observations = transport
        .poll_observations()
        .expect("poll transport observations");

    assert_eq!(transport.transport_id(), TransportProtocol::BleGatt);
    assert!(observations.is_empty());
    assert_eq!(transport.sent_frames, vec![b"frame".to_vec()]);
}

#[test]
fn retention_store_retains_and_releases_opaque_payloads() {
    let object_id = ContentId {
        digest: Blake3Digest([7; 32]),
    };
    let mut retention = StubRetentionStore {
        payloads: BTreeMap::new(),
    };

    retention
        .retain_payload(object_id, b"payload".to_vec())
        .expect("put payload");
    assert!(retention
        .contains_retained_payload(&object_id)
        .expect("contains payload"));

    let payload = retention
        .take_retained_payload(&object_id)
        .expect("take payload");
    assert_eq!(payload, Some(b"payload".to_vec()));
    assert!(!retention
        .contains_retained_payload(&object_id)
        .expect("payload removed"));
}

#[test]
fn mesh_transport_is_also_a_transport_effect_handler() {
    fn assert_transport_handler<T>()
    where
        T: MeshTransport + TransportEffects + EffectHandler<dyn TransportEffects>,
    {
    }

    assert_transport_handler::<StubTransport>();
}

#[test]
fn mesh_routing_engine_exposes_explicit_subcomponent_boundaries() {
    let mut family = StubMeshFamily {
        topology: StubTopologyModel,
        transport: StubTransport {
            observations: Vec::new(),
            sent_frames: Vec::new(),
        },
        retention: StubRetentionStore {
            payloads: BTreeMap::new(),
        },
        route: None,
    };

    assert_eq!(
        family
            .topology_model()
            .adjacent_links(&NodeId([1; 32]), &sample_configuration())
            .len(),
        1
    );
    family
        .transport_mut()
        .send_frame(&sample_endpoint(), b"frame")
        .expect("send frame");
    assert_eq!(
        family.transport().transport_id(),
        TransportProtocol::BleGatt
    );
    family
        .retention_store_mut()
        .retain_payload(
            ContentId {
                digest: Blake3Digest([8; 32]),
            },
            b"payload".to_vec(),
        )
        .expect("store payload");
    assert!(family
        .retention_store()
        .contains_retained_payload(&ContentId {
            digest: Blake3Digest([8; 32]),
        })
        .expect("payload present"));
}
