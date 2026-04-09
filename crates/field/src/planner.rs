//! `RoutingEnginePlanner` implementation: candidate generation and route
//! admission.
//!
//! Translates the private attractor view and destination belief state into
//! public routing decisions satisfying the shared framework planning contract.
//! `candidate_routes` returns a single corridor candidate built from the
//! highest-scoring frontier entry for the requested destination. `admit_route`
//! verifies the candidate against the routing objective and returns a
//! `RouteAdmission` with a full witness.
//!
//! Admission is rejected when delivery support is below 300 permille, posterior
//! entropy exceeds 850 permille, the protection floor is unsatisfied, or the
//! connectivity posture is incompatible with the objective.
//! `route_degradation_for` classifies the degradation reason
//! (LinkInstability, CapacityConstrained, or None) from field belief state.
//! Backend tokens are encoded by `route::encode_backend_token` and embedded in
//! the returned `BackendRouteRef`.

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteRef, Belief,
    ByteCount, ClaimStrength, Configuration, ConnectivityPosture, ConnectivityRegime,
    DestinationId, Estimate, FailureModelClass, Limit, MessageFlowAssumptionClass,
    NodeDensityClass, ObjectiveVsDelivered, Observation, RouteAdmission,
    RouteAdmissionCheck, RouteAdmissionRejection, RouteCandidate, RouteCost,
    RouteDegradation, RouteError, RouteEstimate, RouteProtectionClass,
    RouteSelectionError, RouteSummary, RouteWitness, RoutingEngineCapabilities,
    RoutingEngineId, RuntimeEnvelopeClass, SelectedRoutingParameters,
};
use jacquard_traits::RoutingEnginePlanner;

use crate::{
    attractor::rank_frontier_by_attractor,
    route::{
        encode_backend_token, route_id_for_backend, FieldBackendToken,
        FieldWitnessDetail,
    },
    state::{
        CorridorBeliefEnvelope, DestinationFieldState, DestinationKey,
        ObservationClass, OperatingRegime, RoutingPosture, MAX_ALTERNATE_COUNT,
    },
    summary::{
        derive_degradation_class, EvidenceContributionClass, FieldSummary,
        SummaryDestinationKey, SummaryUncertaintyClass,
    },
    FieldEngine, FIELD_CAPABILITIES, FIELD_ENGINE_ID,
};

struct PlanningArtifacts {
    candidate: RouteCandidate,
    admission_check: RouteAdmissionCheck,
    witness: RouteWitness,
}

struct AdmissionInputs<'a> {
    objective: &'a jacquard_core::RoutingObjective,
    profile: &'a SelectedRoutingParameters,
    summary: &'a RouteSummary,
    destination_state: &'a DestinationFieldState,
    delivered_protection: RouteProtectionClass,
    delivered_connectivity: ConnectivityPosture,
    assumptions: AdmissionAssumptions,
    route_cost: RouteCost,
}

impl<Transport, Effects> RoutingEnginePlanner for FieldEngine<Transport, Effects> {
    fn engine_id(&self) -> RoutingEngineId {
        FIELD_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        FIELD_CAPABILITIES
    }

    fn candidate_routes(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        self.planning_artifacts(objective, profile, topology)
            .map(|artifacts| vec![artifacts.candidate])
            .unwrap_or_default()
    }

    fn check_candidate(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        let Ok(artifacts) = self.planning_artifacts(objective, profile, topology)
        else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        if artifacts.candidate.backend_ref != candidate.backend_ref {
            return Ok(rejected_check(
                artifacts.admission_check.profile,
                RouteAdmissionRejection::BackendUnavailable,
            ));
        }
        Ok(artifacts.admission_check)
    }

    fn admit_route(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        let artifacts = self.planning_artifacts(objective, profile, topology)?;
        if artifacts.candidate.backend_ref != candidate.backend_ref {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        if let AdmissionDecision::Rejected(reason) = artifacts.admission_check.decision
        {
            return Err(RouteSelectionError::Inadmissible(reason).into());
        }
        Ok(RouteAdmission {
            backend_ref: artifacts.candidate.backend_ref,
            objective: objective.clone(),
            profile: profile.clone(),
            admission_check: artifacts.admission_check,
            summary: artifacts.candidate.summary,
            witness: artifacts.witness,
        })
    }
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    pub(crate) fn witness_detail_for_destination(
        &self,
        destination: &DestinationKey,
    ) -> Option<FieldWitnessDetail> {
        self.state
            .destinations
            .get(destination)
            .map(|state| self.witness_detail_from_state(state))
    }

    // long-block-exception: candidate, admission, witness, and route-id
    // derivation form one coherent planning pipeline with shared intermediates.
    fn planning_artifacts(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Result<PlanningArtifacts, RouteError> {
        if !self.destination_supports_objective(topology, objective) {
            return Err(RouteSelectionError::NoCandidate.into());
        }

        let destination_key = DestinationKey::from(&objective.destination);
        let Some(destination_state) = self.state.destinations.get(&destination_key)
        else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let ranked = rank_frontier_by_attractor(
            destination_state,
            &self.state.mean_field,
            self.state.regime.current,
            self.state.posture.current,
            &self.state.controller,
        );
        let Some((primary, _)) = ranked.first() else {
            return Err(RouteSelectionError::NoCandidate.into());
        };

        let witness_detail = self.witness_detail_from_state(destination_state);
        let backend_token = FieldBackendToken {
            destination: destination_key,
            primary_neighbor: primary.neighbor_id,
            alternates: ranked
                .iter()
                .skip(1)
                .take(MAX_ALTERNATE_COUNT)
                .map(|(entry, _)| entry.neighbor_id)
                .collect(),
            topology_epoch: topology.value.epoch,
            regime: self.state.regime.current,
            posture: self.state.posture.current,
        };
        let backend_route_id = encode_backend_token(&backend_token);
        let route_id = route_id_for_backend(&backend_route_id);
        let route_summary =
            self.route_summary_for(destination_state, primary.neighbor_id, topology);
        let degradation =
            self.route_degradation_for(destination_state, topology.value.epoch);
        let delivered_protection = delivered_protection(destination_state);
        let delivered_connectivity =
            delivered_connectivity(self.state.posture.current, destination_state);
        let admission_profile =
            admission_assumptions(&witness_detail, self.state.regime.current);
        let route_cost = route_cost_for(
            &destination_state.corridor_belief,
            ranked.len().saturating_sub(1),
            self.state.posture.current,
        );
        let admission_check = admission_check_for(AdmissionInputs {
            objective,
            profile,
            summary: &route_summary,
            destination_state,
            delivered_protection,
            delivered_connectivity,
            assumptions: admission_profile.clone(),
            route_cost,
        });
        let witness = RouteWitness {
            protection: ObjectiveVsDelivered {
                objective: objective.target_protection,
                delivered: delivered_protection,
            },
            connectivity: ObjectiveVsDelivered {
                objective: objective.target_connectivity,
                delivered: delivered_connectivity,
            },
            admission_profile: admission_profile.clone(),
            topology_epoch: topology.value.epoch,
            degradation,
        };

        Ok(PlanningArtifacts {
            candidate: RouteCandidate {
                route_id,
                summary: route_summary.clone(),
                estimate: Estimate::new(
                    RouteEstimate {
                        estimated_protection: delivered_protection,
                        estimated_connectivity: delivered_connectivity,
                        topology_epoch: topology.value.epoch,
                        degradation,
                    },
                    jacquard_core::RatioPermille(
                        destination_state.posterior.top_corridor_mass.value(),
                    ),
                    topology.observed_at_tick,
                ),
                backend_ref: BackendRouteRef {
                    engine: FIELD_ENGINE_ID,
                    backend_route_id,
                },
            },
            admission_check,
            witness,
        })
    }

    fn witness_detail_from_state(
        &self,
        destination_state: &DestinationFieldState,
    ) -> FieldWitnessDetail {
        FieldWitnessDetail {
            evidence_class: evidence_class_from_state(destination_state),
            uncertainty_class: uncertainty_class_for(
                destination_state.posterior.usability_entropy.value(),
            ),
            regime: self.state.regime.current,
            posture: self.state.posture.current,
            degradation: self
                .route_degradation_for(destination_state, jacquard_core::RouteEpoch(0)),
        }
    }

    fn route_summary_for(
        &self,
        destination_state: &DestinationFieldState,
        primary_neighbor: jacquard_core::NodeId,
        topology: &Observation<Configuration>,
    ) -> RouteSummary {
        let hop_midpoint = destination_state
            .corridor_belief
            .expected_hop_band
            .min_hops
            .saturating_add(
                destination_state
                    .corridor_belief
                    .expected_hop_band
                    .max_hops
                    .saturating_sub(
                        destination_state.corridor_belief.expected_hop_band.min_hops,
                    )
                    / 2,
            );
        let protocol_mix = topology
            .value
            .links
            .get(&(self.local_node_id, primary_neighbor))
            .map(|link| vec![link.endpoint.transport_kind.clone()])
            .unwrap_or_default();
        RouteSummary {
            engine: FIELD_ENGINE_ID,
            protection: delivered_protection(destination_state),
            connectivity: delivered_connectivity(
                self.state.posture.current,
                destination_state,
            ),
            protocol_mix,
            hop_count_hint: Belief::estimated(
                hop_midpoint,
                jacquard_core::RatioPermille(
                    destination_state.posterior.top_corridor_mass.value(),
                ),
                topology.observed_at_tick,
            ),
            valid_for: destination_state.corridor_belief.validity_window,
        }
    }

    fn route_degradation_for(
        &self,
        destination_state: &DestinationFieldState,
        topology_epoch: jacquard_core::RouteEpoch,
    ) -> RouteDegradation {
        let summary = FieldSummary {
            destination: SummaryDestinationKey::from(&DestinationId::from(
                &destination_state.destination,
            )),
            topology_epoch,
            freshness_tick: destination_state
                .corridor_belief
                .validity_window
                .start_tick(),
            hop_band: destination_state.corridor_belief.expected_hop_band,
            delivery_support: destination_state.corridor_belief.delivery_support,
            congestion_penalty: destination_state.corridor_belief.congestion_penalty,
            retention_support: destination_state.corridor_belief.retention_affinity,
            uncertainty_penalty: destination_state.posterior.usability_entropy,
            evidence_class: evidence_class_from_state(destination_state),
            uncertainty_class: uncertainty_class_for(
                destination_state.posterior.usability_entropy.value(),
            ),
        };
        derive_degradation_class(
            &summary,
            self.state.regime.current,
            &self.state.controller,
        )
    }

    fn destination_supports_objective(
        &self,
        topology: &Observation<Configuration>,
        objective: &jacquard_core::RoutingObjective,
    ) -> bool {
        match objective.destination {
            | DestinationId::Node(destination) => topology
                .value
                .nodes
                .get(&destination)
                .map(|node| {
                    node.profile.services.iter().any(|service| {
                        service.service_kind == objective.service_kind
                            && service.routing_engines.contains(&FIELD_ENGINE_ID)
                    })
                })
                .unwrap_or(false),
            | DestinationId::Gateway(_) | DestinationId::Service(_) => self
                .state
                .destinations
                .contains_key(&DestinationKey::from(&objective.destination)),
        }
    }
}

fn evidence_class_from_state(
    destination_state: &DestinationFieldState,
) -> EvidenceContributionClass {
    match destination_state.posterior.predicted_observation_class {
        | ObservationClass::DirectOnly => EvidenceContributionClass::Direct,
        | ObservationClass::ForwardPropagated | ObservationClass::Mixed => {
            EvidenceContributionClass::ForwardPropagated
        },
        | ObservationClass::ReverseValidated => {
            EvidenceContributionClass::ReverseFeedback
        },
    }
}

fn admission_check_for(inputs: AdmissionInputs<'_>) -> RouteAdmissionCheck {
    let AdmissionInputs {
        objective,
        profile,
        summary,
        destination_state,
        delivered_protection,
        delivered_connectivity,
        assumptions,
        route_cost,
    } = inputs;

    let decision = if destination_state.corridor_belief.delivery_support.value() < 300 {
        AdmissionDecision::Rejected(RouteAdmissionRejection::CapacityExceeded)
    } else if objective.protection_floor > FIELD_CAPABILITIES.max_protection
        || profile.selected_protection > FIELD_CAPABILITIES.max_protection
        || delivered_protection < objective.protection_floor
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::ProtectionFloorUnsatisfied)
    } else if destination_state.posterior.usability_entropy.value() > 850 {
        AdmissionDecision::Rejected(
            RouteAdmissionRejection::DeliveryAssumptionUnsupported,
        )
    } else if delivered_connectivity.repair < profile.selected_connectivity.repair
        || delivered_connectivity.partition < profile.selected_connectivity.partition
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::BranchingInfeasible)
    } else {
        AdmissionDecision::Admissible
    };

    RouteAdmissionCheck {
        decision,
        profile: assumptions,
        productive_step_bound: Limit::Bounded(u32::from(
            summary.hop_count_hint.value_or(1),
        )),
        total_step_bound: Limit::Bounded(
            u32::from(summary.hop_count_hint.value_or(1)).saturating_add(2),
        ),
        route_cost,
    }
}

fn delivered_protection(
    destination_state: &DestinationFieldState,
) -> RouteProtectionClass {
    if destination_state.corridor_belief.delivery_support.value() >= 300 {
        RouteProtectionClass::LinkProtected
    } else {
        RouteProtectionClass::None
    }
}

fn delivered_connectivity(
    posture: RoutingPosture,
    destination_state: &DestinationFieldState,
) -> ConnectivityPosture {
    let partition = if destination_state.corridor_belief.delivery_support.value() >= 400
        || posture == RoutingPosture::RetentionBiased
    {
        jacquard_core::RoutePartitionClass::PartitionTolerant
    } else {
        jacquard_core::RoutePartitionClass::ConnectedOnly
    };
    let repair = if posture == RoutingPosture::RiskSuppressed
        && destination_state.posterior.usability_entropy.value() > 700
    {
        jacquard_core::RouteRepairClass::BestEffort
    } else {
        jacquard_core::RouteRepairClass::Repairable
    };
    ConnectivityPosture { repair, partition }
}

fn admission_assumptions(
    witness_detail: &FieldWitnessDetail,
    regime: OperatingRegime,
) -> AdmissionAssumptions {
    AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
        failure_model: match regime {
            | OperatingRegime::Adversarial => FailureModelClass::ByzantineInterface,
            | OperatingRegime::Unstable => FailureModelClass::CrashStop,
            | _ => FailureModelClass::Benign,
        },
        runtime_envelope: RuntimeEnvelopeClass::EnvelopeAdmitted,
        node_density_class: match regime {
            | OperatingRegime::Sparse => NodeDensityClass::Sparse,
            | OperatingRegime::Congested => NodeDensityClass::Dense,
            | OperatingRegime::RetentionFavorable
            | OperatingRegime::Unstable
            | OperatingRegime::Adversarial => NodeDensityClass::Moderate,
        },
        connectivity_regime: match regime {
            | OperatingRegime::Sparse => ConnectivityRegime::Stable,
            | OperatingRegime::Congested | OperatingRegime::RetentionFavorable => {
                ConnectivityRegime::PartitionProne
            },
            | OperatingRegime::Unstable | OperatingRegime::Adversarial => {
                ConnectivityRegime::HighChurn
            },
        },
        adversary_regime: match regime {
            | OperatingRegime::Adversarial => AdversaryRegime::ActiveAdversarial,
            | OperatingRegime::Unstable => AdversaryRegime::BenignUntrusted,
            | _ => AdversaryRegime::Cooperative,
        },
        claim_strength: match (
            witness_detail.evidence_class,
            witness_detail.uncertainty_class,
        ) {
            | (EvidenceContributionClass::Direct, SummaryUncertaintyClass::Low) => {
                ClaimStrength::ConservativeUnderProfile
            },
            | (_, SummaryUncertaintyClass::High) => ClaimStrength::InterfaceOnly,
            | _ => ClaimStrength::ConservativeUnderProfile,
        },
    }
}

fn route_cost_for(
    corridor: &CorridorBeliefEnvelope,
    alternate_count: usize,
    posture: RoutingPosture,
) -> RouteCost {
    let hop_count = corridor.expected_hop_band.max_hops.max(1);
    let hold_bytes_reserved = if posture == RoutingPosture::RetentionBiased {
        ByteCount(256)
    } else {
        ByteCount(0)
    };
    RouteCost {
        message_count_max: Limit::Bounded(u32::from(hop_count)),
        byte_count_max: Limit::Bounded(ByteCount(u64::from(hop_count) * 256)),
        hop_count,
        repair_attempt_count_max: Limit::Bounded(
            u32::try_from(alternate_count).expect("alternate count fits u32"),
        ),
        hold_bytes_reserved: Limit::Bounded(hold_bytes_reserved),
        work_step_count_max: Limit::Bounded(
            u32::from(hop_count)
                .saturating_add(
                    u32::try_from(alternate_count).expect("alternate count fits u32"),
                )
                .saturating_add(1),
        ),
    }
}

fn rejected_check(
    assumptions: AdmissionAssumptions,
    reason: RouteAdmissionRejection,
) -> RouteAdmissionCheck {
    RouteAdmissionCheck {
        decision: AdmissionDecision::Rejected(reason),
        profile: assumptions,
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
    }
}

fn uncertainty_class_for(value: u16) -> SummaryUncertaintyClass {
    match value {
        | 0..=249 => SummaryUncertaintyClass::Low,
        | 250..=599 => SummaryUncertaintyClass::Medium,
        | _ => SummaryUncertaintyClass::High,
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId,
        Environment, FactSourceClass, Observation, OriginAuthenticationClass,
        RatioPermille, RouteEpoch, RoutePartitionClass, RouteProtectionClass,
        RouteRepairClass, RouteServiceKind, RoutingEvidenceClass, RoutingObjective,
        SelectedRoutingParameters, Tick,
    };
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::RoutingEnginePlanner;

    use super::*;
    use crate::state::{
        DestinationInterestClass, HopBand, NeighborContinuation, SupportBucket,
    };

    fn node(byte: u8) -> jacquard_core::NodeId {
        jacquard_core::NodeId([byte; 32])
    }

    fn sample_objective(destination: jacquard_core::NodeId) -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(destination),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
            latency_budget_ms: Limit::Bounded(jacquard_core::DurationMs(100)),
            protection_priority: jacquard_core::PriorityPoints(10),
            connectivity_priority: jacquard_core::PriorityPoints(10),
        }
    }

    fn sample_profile() -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy:
                jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn supported_topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(4),
                nodes: BTreeMap::from([
                    (
                        node(1),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(1), ControllerId([1; 32])),
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![1],
                                    ByteCount(128),
                                ),
                                Tick(1),
                            ),
                            &FIELD_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(2),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(2), ControllerId([2; 32])),
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![2],
                                    ByteCount(128),
                                ),
                                Tick(1),
                            ),
                            &FIELD_ENGINE_ID,
                        )
                        .build(),
                    ),
                ]),
                links: BTreeMap::new(),
                environment: Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(100),
                    contention_permille: RatioPermille(100),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(4),
        }
    }

    fn seeded_engine() -> FieldEngine<(), ()> {
        let mut engine = FieldEngine::new(node(1), (), ());
        let state = engine.state.upsert_destination_interest(
            &DestinationId::Node(node(2)),
            DestinationInterestClass::Transit,
            Tick(4),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(850);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(200);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::DirectOnly;
        state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
        state.corridor_belief.delivery_support = SupportBucket::new(800);
        state.corridor_belief.retention_affinity = SupportBucket::new(300);
        state.frontier = state.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(2),
            net_value: SupportBucket::new(900),
            downstream_support: SupportBucket::new(850),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(4),
        });
        engine
    }

    #[test]
    fn candidate_routes_emit_corridor_candidate_from_frontier() {
        let engine = seeded_engine();
        let candidates = engine.candidate_routes(
            &sample_objective(node(2)),
            &sample_profile(),
            &supported_topology(),
        );
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].summary.engine, FIELD_ENGINE_ID,);
        assert_eq!(
            candidates[0].summary.protection,
            RouteProtectionClass::LinkProtected,
        );
    }

    #[test]
    fn check_candidate_rejects_low_support_corridor_envelope() {
        let mut engine = seeded_engine();
        let state = engine
            .state
            .destinations
            .get_mut(&DestinationKey::from(&DestinationId::Node(node(2))))
            .expect("seeded destination");
        state.corridor_belief.delivery_support = SupportBucket::new(200);
        let topology = supported_topology();
        let candidate = engine
            .candidate_routes(&sample_objective(node(2)), &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let check = engine
            .check_candidate(
                &sample_objective(node(2)),
                &sample_profile(),
                &candidate,
                &topology,
            )
            .expect("check");
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(RouteAdmissionRejection::CapacityExceeded),
        );
    }

    #[test]
    fn admit_route_returns_conservative_witness() {
        let engine = seeded_engine();
        let topology = supported_topology();
        let objective = sample_objective(node(2));
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let admission = engine
            .admit_route(&objective, &sample_profile(), candidate, &topology)
            .expect("admission");
        assert_eq!(admission.summary.engine, FIELD_ENGINE_ID);
        assert_eq!(
            admission.witness.admission_profile.claim_strength,
            ClaimStrength::ConservativeUnderProfile,
        );
    }

    #[test]
    fn witness_detail_tracks_regime_posture_and_uncertainty() {
        let engine = seeded_engine();
        let detail = engine
            .witness_detail_for_destination(&DestinationKey::from(
                &DestinationId::Node(node(2)),
            ))
            .expect("detail");
        assert_eq!(detail.regime, engine.state.regime.current);
        assert_eq!(detail.posture, engine.state.posture.current);
        assert_eq!(detail.uncertainty_class, SummaryUncertaintyClass::Low);
    }
}
