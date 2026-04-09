//! Proactive routing-engine contract coverage for engines that maintain a
//! private table on `engine_tick` before serving advisory candidates.

use std::collections::BTreeMap;

use jacquard_traits::{
    jacquard_core::{
        AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteId,
        BackendRouteRef, Belief, ByteCount, Configuration, ConnectivityPosture,
        ConnectivityRegime, DestinationId, Estimate, FailureModelClass,
        MessageFlowAssumptionClass, NodeDensityClass, NodeId, ObjectiveVsDelivered,
        Observation, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost,
        RouteDegradation, RouteEpoch, RouteEstimate, RouteMaintenanceOutcome,
        RouteMaintenanceResult, RouteMaintenanceTrigger, RoutePartitionClass,
        RouteProtectionClass, RouteRepairClass, RouteRuntimeState,
        RouteShapeVisibility, RouteSummary, RouteWitness, RoutingEngineCapabilities,
        RoutingEngineId, RoutingObjective, RoutingTickChange, RoutingTickContext,
        RoutingTickHint, RoutingTickOutcome, RuntimeEnvelopeClass,
        SelectedRoutingParameters, Tick, TimeWindow, TransportKind,
    },
    RoutingEngine, RoutingEnginePlanner,
};

struct ProactiveContractEngine {
    table: BTreeMap<NodeId, Tick>,
}

impl ProactiveContractEngine {
    fn new() -> Self {
        Self { table: BTreeMap::new() }
    }

    fn engine_id() -> RoutingEngineId {
        RoutingEngineId::from_contract_bytes(*b"traits.proactv.1")
    }
}

impl RoutingEnginePlanner for ProactiveContractEngine {
    fn engine_id(&self) -> RoutingEngineId {
        Self::engine_id()
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine: Self::engine_id(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_traits::jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_traits::jacquard_core::HoldSupport::Unsupported,
            decidable_admission:
                jacquard_traits::jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_traits::jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support:
                jacquard_traits::jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: RouteShapeVisibility::NextHopOnly,
        }
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        let DestinationId::Node(destination) = objective.destination else {
            return Vec::new();
        };
        self.table
            .get(&destination)
            .map(|updated_at_tick| {
                vec![RouteCandidate {
                    route_id: jacquard_traits::jacquard_core::RouteId([7; 16]),
                    summary: RouteSummary {
                        engine: Self::engine_id(),
                        protection: objective.target_protection,
                        connectivity: objective.target_connectivity,
                        protocol_mix: vec![TransportKind::WifiAware],
                        hop_count_hint: Belief::certain(2, *updated_at_tick),
                        valid_for: TimeWindow::new(
                            *updated_at_tick,
                            Tick(updated_at_tick.0.saturating_add(4)),
                        )
                        .expect("valid window"),
                    },
                    estimate: Estimate::certain(
                        RouteEstimate {
                            estimated_protection: objective.target_protection,
                            estimated_connectivity: objective.target_connectivity,
                            topology_epoch: RouteEpoch(1),
                            degradation: RouteDegradation::None,
                        },
                        *updated_at_tick,
                    ),
                    backend_ref: BackendRouteRef {
                        engine: Self::engine_id(),
                        backend_route_id: BackendRouteId(vec![1, 2, 3]),
                    },
                }]
            })
            .unwrap_or_default()
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, jacquard_traits::jacquard_core::RouteError> {
        self.admit_route(objective, profile, candidate.clone(), topology)
            .map(|admission| admission.admission_check)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, jacquard_traits::jacquard_core::RouteError> {
        Ok(RouteAdmission {
            backend_ref: candidate.backend_ref,
            objective: objective.clone(),
            profile: profile.clone(),
            admission_check: RouteAdmissionCheck {
                decision: AdmissionDecision::Admissible,
                profile: AdmissionAssumptions {
                    message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
                    failure_model: FailureModelClass::Benign,
                    runtime_envelope: RuntimeEnvelopeClass::Canonical,
                    node_density_class: NodeDensityClass::Sparse,
                    connectivity_regime: ConnectivityRegime::Stable,
                    adversary_regime: AdversaryRegime::Cooperative,
                    claim_strength: jacquard_traits::jacquard_core::ClaimStrength::ConservativeUnderProfile,
                },
                productive_step_bound: jacquard_traits::jacquard_core::Limit::Bounded(1),
                total_step_bound: jacquard_traits::jacquard_core::Limit::Bounded(1),
                route_cost: RouteCost {
                    message_count_max: jacquard_traits::jacquard_core::Limit::Bounded(1),
                    byte_count_max: jacquard_traits::jacquard_core::Limit::Bounded(ByteCount(64)),
                    hop_count: 2,
                    repair_attempt_count_max: jacquard_traits::jacquard_core::Limit::Bounded(0),
                    hold_bytes_reserved: jacquard_traits::jacquard_core::Limit::Bounded(ByteCount(0)),
                    work_step_count_max: jacquard_traits::jacquard_core::Limit::Bounded(1),
                },
            },
            summary: candidate.summary.clone(),
            witness: RouteWitness {
                protection: ObjectiveVsDelivered {
                    objective: objective.target_protection,
                    delivered: objective.target_protection,
                },
                connectivity: ObjectiveVsDelivered {
                    objective: objective.target_connectivity,
                    delivered: objective.target_connectivity,
                },
                admission_profile: AdmissionAssumptions {
                    message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
                    failure_model: FailureModelClass::Benign,
                    runtime_envelope: RuntimeEnvelopeClass::Canonical,
                    node_density_class: NodeDensityClass::Sparse,
                    connectivity_regime: ConnectivityRegime::Stable,
                    adversary_regime: AdversaryRegime::Cooperative,
                    claim_strength: jacquard_traits::jacquard_core::ClaimStrength::ConservativeUnderProfile,
                },
                topology_epoch: RouteEpoch(1),
                degradation: RouteDegradation::None,
            },
        })
    }
}

impl RoutingEngine for ProactiveContractEngine {
    fn materialize_route(
        &mut self,
        _input: jacquard_traits::jacquard_core::RouteMaterializationInput,
    ) -> Result<
        jacquard_traits::jacquard_core::RouteInstallation,
        jacquard_traits::jacquard_core::RouteError,
    > {
        unreachable!("this contract test only exercises planning and tick shape")
    }

    fn route_commitments(
        &self,
        _route: &jacquard_traits::jacquard_core::MaterializedRoute,
    ) -> Vec<jacquard_traits::jacquard_core::RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, jacquard_traits::jacquard_core::RouteError> {
        let next_table = tick
            .topology
            .value
            .nodes
            .keys()
            .copied()
            .filter(|node_id| *node_id != NodeId([1; 32]))
            .map(|node_id| (node_id, tick.topology.observed_at_tick))
            .collect::<BTreeMap<_, _>>();
        let changed = self.table != next_table;
        self.table = next_table;
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: if changed {
                RoutingTickChange::PrivateStateUpdated
            } else {
                RoutingTickChange::NoChange
            },
            next_tick_hint: if changed {
                RoutingTickHint::Immediate
            } else {
                RoutingTickHint::WithinTicks(Tick(3))
            },
        })
    }

    fn maintain_route(
        &mut self,
        _identity: &jacquard_traits::jacquard_core::PublishedRouteRecord,
        _runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, jacquard_traits::jacquard_core::RouteError>
    {
        Ok(RouteMaintenanceResult {
            event: jacquard_traits::jacquard_core::RouteLifecycleEvent::Activated,
            outcome: RouteMaintenanceOutcome::Continued,
        })
    }

    fn teardown(&mut self, _route_id: &jacquard_traits::jacquard_core::RouteId) {}
}

#[test]
fn routing_engine_supports_private_table_proactive_ticks() {
    let topology = common::sample_configuration();
    let mut engine = ProactiveContractEngine::new();

    let first = engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("first proactive tick");
    assert_eq!(first.change, RoutingTickChange::PrivateStateUpdated);
    assert_eq!(first.next_tick_hint, RoutingTickHint::Immediate);

    let second = engine
        .engine_tick(&RoutingTickContext::new(topology))
        .expect("second proactive tick");
    assert_eq!(second.change, RoutingTickChange::NoChange);
    assert_eq!(second.next_tick_hint, RoutingTickHint::WithinTicks(Tick(3)));
}

#[test]
fn proactive_candidates_can_be_served_without_explicit_path_visibility() {
    let topology = common::sample_configuration();
    let mut engine = ProactiveContractEngine::new();
    let objective = common::sample_objective();
    let profile = common::sample_profile();

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("populate proactive table");
    let candidates = engine.candidate_routes(&objective, &profile, &topology);

    assert_eq!(
        engine.capabilities().route_shape_visibility,
        RouteShapeVisibility::NextHopOnly
    );
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].summary.hop_count_hint,
        Belief::certain(2, Tick(1))
    );
}

mod common {
    use std::collections::BTreeMap;

    use jacquard_traits::jacquard_core::{
        Configuration, ConnectivityPosture, ControllerId, Environment, Link,
        LinkEndpoint, LinkProfile, LinkRuntimeState, LinkState, Node, NodeProfile,
        NodeState, Observation, OriginAuthenticationClass, RatioPermille,
        RepairCapability, RouteEpoch, RoutePartitionClass, RouteProtectionClass,
        RouteRepairClass, RouteServiceKind, RoutingEvidenceClass, RoutingObjective,
        SelectedRoutingParameters, Tick, TransportKind,
    };

    pub(super) fn sample_configuration() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(1),
                nodes: BTreeMap::from([
                    (node(1), empty_node(1)),
                    (node(2), empty_node(2)),
                    (node(3), empty_node(3)),
                ]),
                links: BTreeMap::from([
                    ((node(1), node(2)), active_link(2)),
                    ((node(2), node(3)), active_link(3)),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 2,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    pub(super) fn sample_objective() -> RoutingObjective {
        RoutingObjective {
            destination: jacquard_traits::jacquard_core::DestinationId::Node(node(3)),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy:
                jacquard_traits::jacquard_core::HoldFallbackPolicy::Forbidden,
            latency_budget_ms: jacquard_traits::jacquard_core::Limit::Bounded(
                jacquard_traits::jacquard_core::DurationMs(100),
            ),
            protection_priority: jacquard_traits::jacquard_core::PriorityPoints(10),
            connectivity_priority: jacquard_traits::jacquard_core::PriorityPoints(10),
        }
    }

    pub(super) fn sample_profile() -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            deployment_profile:
                jacquard_traits::jacquard_core::OperatingMode::SparseLowPower,
            diversity_floor: jacquard_traits::jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy:
                jacquard_traits::jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy:
                jacquard_traits::jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn node(byte: u8) -> jacquard_traits::jacquard_core::NodeId {
        jacquard_traits::jacquard_core::NodeId([byte; 32])
    }

    fn empty_node(byte: u8) -> Node {
        Node {
            controller_id: ControllerId([byte; 32]),
            profile: NodeProfile {
                services: Vec::new(),
                endpoints: vec![LinkEndpoint::new(
                    TransportKind::WifiAware,
                    jacquard_traits::jacquard_core::EndpointLocator::Opaque(vec![byte]),
                    jacquard_traits::jacquard_core::ByteCount(64),
                )],
                connection_count_max: 4,
                neighbor_state_count_max: 4,
                simultaneous_transfer_count_max: 1,
                active_route_count_max: 4,
                relay_work_budget_max: jacquard_traits::jacquard_core::RelayWorkBudget(
                    1,
                ),
                maintenance_work_budget_max:
                    jacquard_traits::jacquard_core::MaintenanceWorkBudget(1),
                hold_item_count_max: jacquard_traits::jacquard_core::HoldItemCount(0),
                hold_capacity_bytes_max: jacquard_traits::jacquard_core::ByteCount(0),
            },
            state: NodeState {
                relay_budget: jacquard_traits::jacquard_core::Belief::Absent,
                available_connection_count:
                    jacquard_traits::jacquard_core::Belief::Absent,
                hold_capacity_available_bytes:
                    jacquard_traits::jacquard_core::Belief::Absent,
                information_summary: jacquard_traits::jacquard_core::Belief::Absent,
            },
        }
    }

    fn active_link(remote_byte: u8) -> Link {
        Link {
            endpoint: LinkEndpoint::new(
                TransportKind::WifiAware,
                jacquard_traits::jacquard_core::EndpointLocator::Opaque(vec![
                    remote_byte,
                ]),
                jacquard_traits::jacquard_core::ByteCount(64),
            ),
            profile: LinkProfile {
                latency_floor_ms: jacquard_traits::jacquard_core::DurationMs(5),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: jacquard_traits::jacquard_core::PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: jacquard_traits::jacquard_core::Belief::Absent,
                transfer_rate_bytes_per_sec: jacquard_traits::jacquard_core::Belief::Absent,
                stability_horizon_ms: jacquard_traits::jacquard_core::Belief::Absent,
                loss_permille: RatioPermille(10),
                delivery_confidence_permille: jacquard_traits::jacquard_core::Belief::certain(
                    RatioPermille(950),
                    Tick(1),
                ),
                symmetry_permille: jacquard_traits::jacquard_core::Belief::certain(
                    RatioPermille(900),
                    Tick(1),
                ),
            },
        }
    }
}
