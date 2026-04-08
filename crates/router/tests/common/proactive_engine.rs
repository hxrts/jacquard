//! `ProactiveTableTestEngine` — a synthetic proactive routing engine used to
//! prove the router does not require explicit-path visibility.
//!
//! Control flow:
//! - `engine_tick` rebuilds one engine-private next-hop table from observed
//!   topology
//! - `candidate_routes` serves advisory candidates from that private table
//! - `materialize_route` binds router-owned canonical identity to one
//!   engine-private forwarding record

use std::collections::BTreeMap;

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteId,
    BackendRouteRef, Belief, ByteCount, ClaimStrength, Configuration,
    ConnectivityPosture, ConnectivityRegime, DegradationReason, Estimate, Fact,
    FactBasis, FailureModelClass, HealthScore, Limit, LinkRuntimeState,
    MessageFlowAssumptionClass, NodeDensityClass, NodeId, ObjectiveVsDelivered,
    Observation, RatioPermille, ReachabilityState, RouteAdmission, RouteAdmissionCheck,
    RouteAdmissionRejection, RouteCandidate, RouteCost, RouteDegradation, RouteEpoch,
    RouteEstimate, RouteHealth, RouteInstallation, RouteLifecycleEvent,
    RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceResult,
    RouteMaintenanceTrigger, RouteMaterializationInput, RouteMaterializationProof,
    RoutePartitionClass, RouteProgressContract, RouteProgressState,
    RouteProtectionClass, RouteRepairClass, RouteRuntimeError, RouteRuntimeState,
    RouteSelectionError, RouteShapeVisibility, RouteSummary, RouteWitness,
    RoutingEngineCapabilities, RoutingEngineId, RoutingObjective, RoutingTickChange,
    RoutingTickContext, RoutingTickHint, RoutingTickOutcome, RuntimeEnvelopeClass,
    SelectedRoutingParameters, Tick, TimeWindow, TransportProtocol,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProactiveTableEntry {
    destination: NodeId,
    next_hop: NodeId,
    tq: RatioPermille,
    hop_count: u8,
    topology_epoch: RouteEpoch,
    updated_at_tick: Tick,
    degradation: RouteDegradation,
    protocol: TransportProtocol,
}

pub(crate) struct ProactiveTableTestEngine {
    local_node_id: NodeId,
    engine_id: RoutingEngineId,
    visibility: RouteShapeVisibility,
    now: Tick,
    table: BTreeMap<NodeId, ProactiveTableEntry>,
    active_routes: BTreeMap<jacquard_core::RouteId, NodeId>,
}

impl ProactiveTableTestEngine {
    pub(crate) fn new(
        local_node_id: NodeId,
        engine_id: RoutingEngineId,
        visibility: RouteShapeVisibility,
        now: Tick,
    ) -> Self {
        Self {
            local_node_id,
            engine_id,
            visibility,
            now,
            table: BTreeMap::new(),
            active_routes: BTreeMap::new(),
        }
    }

    fn route_id_for(&self, destination: NodeId) -> jacquard_core::RouteId {
        let mut bytes = [0_u8; 16];
        bytes[..8].copy_from_slice(&self.engine_id.contract_id.0[..8]);
        bytes[8..].copy_from_slice(&destination.0[..8]);
        jacquard_core::RouteId(bytes)
    }

    fn backend_route_id_for(
        &self,
        destination: NodeId,
        next_hop: NodeId,
    ) -> BackendRouteId {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(&destination.0);
        bytes.extend_from_slice(&next_hop.0);
        BackendRouteId(bytes)
    }

    fn build_table(
        &self,
        topology: &Observation<Configuration>,
        now: Tick,
    ) -> BTreeMap<NodeId, ProactiveTableEntry> {
        topology
            .value
            .nodes
            .keys()
            .copied()
            .filter(|destination| *destination != self.local_node_id)
            .filter_map(|destination| self.best_entry_for(destination, topology, now))
            .map(|entry| (entry.destination, entry))
            .collect()
    }

    fn best_entry_for(
        &self,
        destination: NodeId,
        topology: &Observation<Configuration>,
        now: Tick,
    ) -> Option<ProactiveTableEntry> {
        let direct = self
            .link_score(topology, self.local_node_id, destination)
            .map(|(score, protocol, degradation)| ProactiveTableEntry {
                destination,
                next_hop: destination,
                tq: score,
                hop_count: 1,
                topology_epoch: topology.value.epoch,
                updated_at_tick: now,
                degradation,
                protocol,
            });

        let via_neighbor = topology
            .value
            .links
            .keys()
            .filter(|(from, to)| *from == self.local_node_id && *to != destination)
            .filter_map(|(_, neighbor)| {
                let (first_hop_score, protocol, first_degradation) =
                    self.link_score(topology, self.local_node_id, *neighbor)?;
                let (second_hop_score, _, second_degradation) =
                    self.link_score(topology, *neighbor, destination)?;
                let combined = tq_product(first_hop_score, second_hop_score);
                Some(ProactiveTableEntry {
                    destination,
                    next_hop: *neighbor,
                    tq: combined,
                    hop_count: 2,
                    topology_epoch: topology.value.epoch,
                    updated_at_tick: now,
                    degradation: max_degradation(first_degradation, second_degradation),
                    protocol,
                })
            })
            .max_by_key(|entry| (entry.tq, std::cmp::Reverse(entry.next_hop)));

        direct.into_iter().chain(via_neighbor).max_by_key(|entry| {
            (
                entry.tq,
                std::cmp::Reverse(entry.hop_count),
                std::cmp::Reverse(entry.next_hop),
            )
        })
    }

    fn link_score(
        &self,
        topology: &Observation<Configuration>,
        from: NodeId,
        to: NodeId,
    ) -> Option<(RatioPermille, TransportProtocol, RouteDegradation)> {
        let link = topology.value.links.get(&(from, to))?;
        if matches!(
            link.state.state,
            LinkRuntimeState::Suspended | LinkRuntimeState::Faulted
        ) {
            return None;
        }
        let delivery =
            ratio_belief_or_default(&link.state.delivery_confidence_permille, 850);
        let symmetry = ratio_belief_or_default(&link.state.symmetry_permille, 850);
        let loss = u32::from(link.state.loss_permille.0);
        let weighted = (u32::from(delivery.0) * 5
            + u32::from(symmetry.0) * 3
            + (1000_u32.saturating_sub(loss)) * 2)
            / 10;
        let score = RatioPermille(u16::try_from(weighted).expect("permille score"));
        let degradation = if score.0 < 700 {
            RouteDegradation::Degraded(DegradationReason::LinkInstability)
        } else {
            RouteDegradation::None
        };
        Some((score, link.endpoint.protocol.clone(), degradation))
    }

    fn candidate_for(
        &self,
        objective: &RoutingObjective,
        entry: &ProactiveTableEntry,
    ) -> RouteCandidate {
        RouteCandidate {
            route_id: self.route_id_for(entry.destination),
            summary: RouteSummary {
                engine: self.engine_id.clone(),
                protection: objective.target_protection,
                connectivity: objective.target_connectivity,
                protocol_mix: vec![entry.protocol.clone()],
                hop_count_hint: Belief::certain(entry.hop_count, entry.updated_at_tick),
                valid_for: TimeWindow::new(
                    entry.updated_at_tick,
                    Tick(entry.updated_at_tick.0.saturating_add(8)),
                )
                .expect("valid candidate window"),
            },
            estimate: Estimate::certain(
                RouteEstimate {
                    estimated_protection: objective.target_protection,
                    estimated_connectivity: objective.target_connectivity,
                    topology_epoch: entry.topology_epoch,
                    degradation: entry.degradation,
                },
                entry.updated_at_tick,
            ),
            backend_ref: BackendRouteRef {
                engine: self.engine_id.clone(),
                backend_route_id: self
                    .backend_route_id_for(entry.destination, entry.next_hop),
            },
        }
    }

    fn admission_for(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
    ) -> RouteAdmission {
        RouteAdmission {
            backend_ref: candidate.backend_ref,
            objective: objective.clone(),
            profile: profile.clone(),
            admission_check: RouteAdmissionCheck {
                decision: AdmissionDecision::Admissible,
                profile: AdmissionAssumptions {
                    message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
                    failure_model: FailureModelClass::Benign,
                    runtime_envelope: RuntimeEnvelopeClass::Canonical,
                    node_density_class: NodeDensityClass::Moderate,
                    connectivity_regime: ConnectivityRegime::Stable,
                    adversary_regime: AdversaryRegime::Cooperative,
                    claim_strength: ClaimStrength::ConservativeUnderProfile,
                },
                productive_step_bound: Limit::Bounded(1),
                total_step_bound: Limit::Bounded(1),
                route_cost: RouteCost {
                    message_count_max: Limit::Bounded(1),
                    byte_count_max: Limit::Bounded(ByteCount(128)),
                    hop_count: 1,
                    repair_attempt_count_max: Limit::Bounded(0),
                    hold_bytes_reserved: Limit::Bounded(ByteCount(0)),
                    work_step_count_max: Limit::Bounded(1),
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
                    node_density_class: NodeDensityClass::Moderate,
                    connectivity_regime: ConnectivityRegime::Stable,
                    adversary_regime: AdversaryRegime::Cooperative,
                    claim_strength: ClaimStrength::ConservativeUnderProfile,
                },
                topology_epoch: candidate.estimate.value.topology_epoch,
                degradation: candidate.estimate.value.degradation,
            },
        }
    }

    fn entry_for_objective(
        &self,
        objective: &RoutingObjective,
    ) -> Option<&ProactiveTableEntry> {
        match objective.destination {
            | jacquard_core::DestinationId::Node(destination) => {
                self.table.get(&destination)
            },
            | _ => None,
        }
    }
}

impl jacquard_traits::RoutingEnginePlanner for ProactiveTableTestEngine {
    fn engine_id(&self) -> RoutingEngineId {
        self.engine_id.clone()
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine: self.engine_id.clone(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_core::HoldSupport::Unsupported,
            decidable_admission: jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: self.visibility,
        }
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        self.entry_for_objective(objective)
            .map(|entry| vec![self.candidate_for(objective, entry)])
            .unwrap_or_default()
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, jacquard_core::RouteError> {
        let entry = self
            .entry_for_objective(objective)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let expected = self.candidate_for(objective, entry);
        if expected.backend_ref != candidate.backend_ref {
            return Ok(RouteAdmissionCheck {
                decision: AdmissionDecision::Rejected(
                    RouteAdmissionRejection::BackendUnavailable,
                ),
                profile: self
                    .admission_for(objective, profile, expected)
                    .admission_check
                    .profile,
                productive_step_bound: Limit::Bounded(0),
                total_step_bound: Limit::Bounded(0),
                route_cost: RouteCost {
                    message_count_max: Limit::Bounded(0),
                    byte_count_max: Limit::Bounded(ByteCount(0)),
                    hop_count: 0,
                    repair_attempt_count_max: Limit::Bounded(0),
                    hold_bytes_reserved: Limit::Bounded(ByteCount(0)),
                    work_step_count_max: Limit::Bounded(0),
                },
            });
        }
        Ok(self
            .admission_for(objective, profile, expected)
            .admission_check)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, jacquard_core::RouteError> {
        let entry = self
            .entry_for_objective(objective)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let expected = self.candidate_for(objective, entry);
        if expected.backend_ref != candidate.backend_ref {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        Ok(self.admission_for(objective, profile, expected))
    }
}

impl jacquard_traits::RoutingEngine for ProactiveTableTestEngine {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, jacquard_core::RouteError> {
        let destination = match input.admission.objective.destination {
            | jacquard_core::DestinationId::Node(destination) => destination,
            | _ => return Err(RouteSelectionError::NoCandidate.into()),
        };
        let entry = self
            .table
            .get(&destination)
            .ok_or(RouteSelectionError::NoCandidate)?;
        self.active_routes
            .insert(*input.handle.route_id(), entry.next_hop);
        Ok(RouteInstallation {
            materialization_proof: RouteMaterializationProof {
                stamp: input.handle.stamp.clone(),
                witness: Fact {
                    basis: FactBasis::Admitted,
                    value: input.admission.witness.clone(),
                    established_at_tick: self.now,
                },
            },
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: HealthScore(u32::from(entry.tq.0)),
                congestion_penalty_points: jacquard_core::PenaltyPoints(0),
                last_validated_at_tick: self.now,
            },
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(1),
                total_step_count_max: Limit::Bounded(1),
                last_progress_at_tick: self.now,
                state: RouteProgressState::Pending,
            },
        })
    }

    fn route_commitments(
        &self,
        _route: &jacquard_core::MaterializedRoute,
    ) -> Vec<jacquard_core::RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, jacquard_core::RouteError> {
        self.now = tick.topology.observed_at_tick;
        let next_table = self.build_table(&tick.topology, self.now);
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
                RoutingTickHint::WithinTicks(Tick(4))
            },
        })
    }

    fn maintain_route(
        &mut self,
        identity: &jacquard_core::PublishedRouteRecord,
        runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, jacquard_core::RouteError> {
        let destination = match identity.admission.objective.destination {
            | jacquard_core::DestinationId::Node(destination) => destination,
            | _ => {
                return Ok(RouteMaintenanceResult {
                    event: RouteLifecycleEvent::Expired,
                    outcome: RouteMaintenanceOutcome::Failed(
                        RouteMaintenanceFailure::InvalidEvidence,
                    ),
                })
            },
        };
        let Some(entry) = self.table.get(&destination) else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(
                    RouteMaintenanceFailure::LostReachability,
                ),
            });
        };
        let Some(active_next_hop) = self.active_routes.get(identity.route_id()) else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        runtime.health.last_validated_at_tick = self.now;
        runtime.health.stability_score = HealthScore(u32::from(entry.tq.0));
        if active_next_hop == &entry.next_hop {
            Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Activated,
                outcome: RouteMaintenanceOutcome::Continued,
            })
        } else {
            Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::LinkDegraded,
                },
            })
        }
    }

    fn teardown(&mut self, route_id: &jacquard_core::RouteId) {
        self.active_routes.remove(route_id);
    }
}

impl jacquard_traits::RouterManagedEngine for ProactiveTableTestEngine {
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &jacquard_core::RouteId,
        _payload: &[u8],
    ) -> Result<(), jacquard_core::RouteError> {
        if self.active_routes.contains_key(route_id) {
            Ok(())
        } else {
            Err(RouteSelectionError::NoCandidate.into())
        }
    }

    fn restore_route_runtime_for_router(
        &mut self,
        route_id: &jacquard_core::RouteId,
    ) -> Result<bool, jacquard_core::RouteError> {
        Ok(self.active_routes.contains_key(route_id))
    }
}

fn ratio_belief_or_default(
    value: &Belief<RatioPermille>,
    default: u16,
) -> RatioPermille {
    match value {
        | Belief::Estimated(estimate) => estimate.value,
        | Belief::Absent => RatioPermille(default),
    }
}

fn tq_product(left: RatioPermille, right: RatioPermille) -> RatioPermille {
    let value = (u32::from(left.0) * u32::from(right.0)) / 1000;
    RatioPermille(u16::try_from(value).expect("permille product"))
}

fn max_degradation(
    left: RouteDegradation,
    right: RouteDegradation,
) -> RouteDegradation {
    match (left, right) {
        | (RouteDegradation::Degraded(reason), _)
        | (_, RouteDegradation::Degraded(reason)) => RouteDegradation::Degraded(reason),
        | (RouteDegradation::None, RouteDegradation::None) => RouteDegradation::None,
    }
}
