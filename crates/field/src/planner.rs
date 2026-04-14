//! `RoutingEnginePlanner` implementation: candidate generation and route
//! admission.
//!
//! Translates the private attractor view and destination belief state into
//! public routing decisions satisfying the shared framework planning contract.
//! `candidate_routes` returns one corridor candidate for the requested
//! objective: field stays a single private-selector engine even though it may
//! consider multiple admissible continuations internally. `admit_route`
//! verifies the candidate against the routing objective and returns a
//! `RouteAdmission` with a full witness.
//!
//! Admission is rejected when delivery support is below 300 permille, posterior
//! entropy exceeds 850 permille, the protection floor is unsatisfied, or the
//! connectivity posture is incompatible with the objective.
//! `route_degradation_for` classifies the degradation reason
//! (LinkInstability, CapacityConstrained, or None) from field belief state.
//! Backend tokens are encoded by `route::encode_backend_token` and embedded in the
//! returned `BackendRouteRef`. They carry one selected runtime realization plus
//! a bounded continuation envelope, not several planner-visible field candidates.
// long-file-exception: planner keeps candidate production, admission, and promotion assessment together because those mappings share one audited route-publication contract.

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteRef, Belief, ByteCount,
    ClaimStrength, Configuration, ConnectivityPosture, ConnectivityRegime, DestinationId, Estimate,
    FailureModelClass, Limit, MessageFlowAssumptionClass, NodeDensityClass, ObjectiveVsDelivered,
    Observation, RouteAdmission, RouteAdmissionCheck, RouteAdmissionRejection, RouteCandidate,
    RouteCost, RouteDegradation, RouteError, RouteEstimate, RouteProtectionClass,
    RouteSelectionError, RouteSummary, RouteWitness, RoutingEngineCapabilities, RoutingEngineId,
    RuntimeEnvelopeClass, SelectedRoutingParameters, Tick,
};
use jacquard_traits::RoutingEnginePlanner;

use crate::{
    attractor::rank_frontier_by_attractor,
    recovery::FieldPromotionBlocker,
    route::{
        encode_backend_token, route_id_for_backend, ActiveFieldRoute, FieldBackendToken,
        FieldBootstrapClass, FieldContinuityBand, FieldWitnessDetail,
    },
    runtime::FIELD_ROUTE_WEAK_SUPPORT_FLOOR,
    state::{
        CorridorBeliefEnvelope, DestinationFieldState, DestinationKey, ObservationClass,
        OperatingRegime, RoutingPosture, MAX_CONTINUATION_NEIGHBOR_COUNT,
    },
    summary::{
        derive_degradation_class, summary_divergence, EvidenceContributionClass, FieldSummary,
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
    search_config: &'a crate::FieldSearchConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldAdmissionClass {
    BootstrapAdmissible,
    SteadyAdmissible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldBootstrapDecision {
    Hold,
    Narrow,
    Promote,
    Withdraw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldPromotionAssessment {
    pub(crate) support_growth: bool,
    pub(crate) uncertainty_reduced: bool,
    pub(crate) anti_entropy_confirmed: bool,
    pub(crate) continuation_coherent: bool,
    pub(crate) fresh_enough: bool,
}

impl FieldPromotionAssessment {
    #[must_use]
    fn confirmed_stability(
        self,
        destination_state: &DestinationFieldState,
        confirmation_streak: u8,
        promotion_window_score: u8,
    ) -> bool {
        (confirmation_streak >= 1 || promotion_window_score >= 3)
            && self.anti_entropy_confirmed
            && self.continuation_coherent
            && self.fresh_enough
            && destination_state.corridor_belief.delivery_support.value() >= 180
            && destination_state.corridor_belief.retention_affinity.value() >= 240
            && destination_state.posterior.top_corridor_mass.value() >= 220
            && destination_state.posterior.usability_entropy.value() <= 925
    }

    #[must_use]
    pub(crate) fn can_promote(self, promotion_window_score: u8) -> bool {
        self.anti_entropy_confirmed
            && self.continuation_coherent
            && self.fresh_enough
            && ((self.support_growth && self.uncertainty_reduced) || promotion_window_score >= 4)
    }

    #[must_use]
    pub(crate) fn degraded_but_coherent(self, destination_state: &DestinationFieldState) -> bool {
        self.continuation_coherent
            && (self.fresh_enough || self.anti_entropy_confirmed)
            && destination_state.corridor_belief.retention_affinity.value() >= 260
            && destination_state.corridor_belief.delivery_support.value()
                >= FIELD_ROUTE_WEAK_SUPPORT_FLOOR.saturating_sub(40)
    }

    #[must_use]
    pub(crate) fn decision_for_bootstrap(
        self,
        destination_state: &DestinationFieldState,
        confirmation_streak: u8,
        promotion_window_score: u8,
        search_config: &crate::FieldSearchConfig,
    ) -> FieldBootstrapDecision {
        if (self.can_promote(promotion_window_score)
            || self.confirmed_stability(
                destination_state,
                confirmation_streak,
                promotion_window_score,
            ))
            && promoted_corridor_admissible_with_config(
                destination_state,
                confirmation_streak,
                promotion_window_score,
                search_config,
            )
        {
            FieldBootstrapDecision::Promote
        } else if self.degraded_but_coherent(destination_state)
            && destination_state.frontier.len() > 1
        {
            FieldBootstrapDecision::Narrow
        } else if self.degraded_but_coherent(destination_state) {
            FieldBootstrapDecision::Hold
        } else {
            FieldBootstrapDecision::Withdraw
        }
    }

    #[must_use]
    pub(crate) fn primary_blocker(self) -> FieldPromotionBlocker {
        if !self.support_growth {
            FieldPromotionBlocker::SupportTrend
        } else if !self.uncertainty_reduced {
            FieldPromotionBlocker::Uncertainty
        } else if !self.anti_entropy_confirmed {
            FieldPromotionBlocker::AntiEntropyConfirmation
        } else if !self.continuation_coherent {
            FieldPromotionBlocker::ContinuationCoherence
        } else {
            FieldPromotionBlocker::Freshness
        }
    }
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
        let Ok(artifacts) = self.planning_artifacts(objective, profile, topology) else {
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
        if let AdmissionDecision::Rejected(reason) = artifacts.admission_check.decision {
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
        let destination_key = DestinationKey::from(&objective.destination);
        let Some(destination_state) = self.state.destinations.get(&destination_key) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        if !self.destination_supports_objective(topology, objective) {
            return Err(RouteSelectionError::NoCandidate.into());
        }

        let search_record = self.search_record_for_objective(objective, topology);
        let ranked = rank_frontier_by_attractor(
            destination_state,
            &self.state.mean_field,
            self.state.regime.current,
            self.state.posture.current,
            &self.state.controller,
        );
        let Some(selected_continuation) = search_record.selected_continuation.as_ref() else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let selected_neighbor = selected_continuation.chosen_neighbor;
        if !selected_neighbor_publishable(
            destination_state,
            topology,
            self.local_node_id,
            selected_neighbor,
        ) {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        let mut continuation_neighbors = Vec::with_capacity(MAX_CONTINUATION_NEIGHBOR_COUNT + 1);
        let mut continuation_set = std::collections::BTreeSet::new();
        continuation_neighbors.push(selected_neighbor);
        continuation_set.insert(selected_neighbor);
        if matches!(objective.destination, DestinationId::Service(_)) {
            continuation_neighbors.extend(
                service_publication_neighbors(
                    destination_state,
                    selected_neighbor,
                    &self.search_config,
                )
                .into_iter()
                .filter(|neighbor_id| continuation_set.insert(*neighbor_id)),
            );
        } else if self.search_config.node_discovery_enabled() {
            continuation_neighbors.extend(
                node_publication_neighbors(
                    destination_state,
                    selected_neighbor,
                    &self.search_config,
                )
                .into_iter()
                .filter(|neighbor_id| continuation_set.insert(*neighbor_id)),
            );
        }
        continuation_neighbors.extend(
            ranked
                .iter()
                .filter(|(entry, _)| entry.neighbor_id != selected_neighbor)
                .filter(|(entry, _)| continuation_set.insert(entry.neighbor_id))
                .take(MAX_CONTINUATION_NEIGHBOR_COUNT + 1)
                .map(|(entry, _)| entry.neighbor_id),
        );
        continuation_neighbors.truncate(MAX_CONTINUATION_NEIGHBOR_COUNT + 1);

        let admission_class =
            admission_class_for_state_with_config(destination_state, &self.search_config);
        let witness_detail = self.witness_detail_from_state(destination_state);
        let backend_token = FieldBackendToken {
            destination: destination_key,
            selected_neighbor,
            continuation_neighbors: continuation_neighbors.clone(),
            topology_epoch: topology.value.epoch,
            regime: self.state.regime.current,
            posture: self.state.posture.current,
        };
        let backend_route_id = encode_backend_token(&backend_token);
        let route_id = route_id_for_backend(&backend_route_id);
        let route_summary = self.route_summary_for(destination_state, selected_neighbor, topology);
        let degradation = self.route_degradation_for(destination_state, topology.value.epoch);
        let delivered_protection = delivered_protection(destination_state, &self.search_config);
        let delivered_connectivity = delivered_connectivity(
            self.state.posture.current,
            destination_state,
            &self.search_config,
        );
        let admission_profile =
            admission_assumptions(&witness_detail, self.state.regime.current, admission_class);
        let route_cost = route_cost_for(
            &destination_state.corridor_belief,
            continuation_neighbors.len().saturating_sub(1),
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
            search_config: &self.search_config,
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

    pub(crate) fn witness_detail_from_state(
        &self,
        destination_state: &DestinationFieldState,
    ) -> FieldWitnessDetail {
        FieldWitnessDetail {
            evidence_class: evidence_class_from_state(destination_state),
            uncertainty_class: uncertainty_class_for(
                destination_state.posterior.usability_entropy.value(),
            ),
            bootstrap_class: bootstrap_class_for_state_with_config(
                destination_state,
                &self.search_config,
            ),
            continuity_band: continuity_band_for_state_with_config(
                destination_state,
                &self.search_config,
            ),
            corridor_support: destination_state.corridor_belief.delivery_support,
            retention_support: destination_state.corridor_belief.retention_affinity,
            usability_entropy: destination_state.posterior.usability_entropy,
            top_corridor_mass: destination_state.posterior.top_corridor_mass,
            frontier_width: u8::try_from(destination_state.frontier.len()).unwrap_or(u8::MAX),
            regime: self.state.regime.current,
            posture: self.state.posture.current,
            degradation: self
                .route_degradation_for(destination_state, jacquard_core::RouteEpoch(0)),
        }
    }

    fn route_summary_for(
        &self,
        destination_state: &DestinationFieldState,
        summary_neighbor: jacquard_core::NodeId,
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
                    .saturating_sub(destination_state.corridor_belief.expected_hop_band.min_hops)
                    / 2,
            );
        let protocol_mix = topology
            .value
            .links
            .get(&(self.local_node_id, summary_neighbor))
            .map(|link| vec![link.endpoint.transport_kind.clone()])
            .unwrap_or_default();
        RouteSummary {
            engine: FIELD_ENGINE_ID,
            protection: delivered_protection(destination_state, &self.search_config),
            connectivity: delivered_connectivity(
                self.state.posture.current,
                destination_state,
                &self.search_config,
            ),
            protocol_mix,
            hop_count_hint: Belief::estimated(
                hop_midpoint,
                jacquard_core::RatioPermille(publication_confidence_for(
                    destination_state,
                    &self.search_config,
                )),
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
        derive_degradation_class(&summary, self.state.regime.current, &self.state.controller)
    }

    pub(crate) fn destination_supports_objective(
        &self,
        topology: &Observation<Configuration>,
        objective: &jacquard_core::RoutingObjective,
    ) -> bool {
        match objective.destination {
            DestinationId::Node(destination) => topology
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
            DestinationId::Gateway(_) | DestinationId::Service(_) => self
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
        ObservationClass::DirectOnly => EvidenceContributionClass::Direct,
        ObservationClass::ForwardPropagated | ObservationClass::Mixed => {
            EvidenceContributionClass::ForwardPropagated
        }
        ObservationClass::ReverseValidated => EvidenceContributionClass::ReverseFeedback,
    }
}

fn selected_neighbor_publishable(
    destination_state: &DestinationFieldState,
    topology: &Observation<Configuration>,
    local_node_id: jacquard_core::NodeId,
    selected_neighbor: jacquard_core::NodeId,
) -> bool {
    destination_state
        .frontier
        .as_slice()
        .iter()
        .any(|entry| entry.neighbor_id == selected_neighbor)
        || destination_state
            .pending_forward_evidence
            .iter()
            .any(|evidence| evidence.from_neighbor == selected_neighbor)
        || topology
            .value
            .links
            .contains_key(&(local_node_id, selected_neighbor))
        || topology
            .value
            .links
            .contains_key(&(selected_neighbor, local_node_id))
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
        search_config,
    } = inputs;

    let decision = if !bootstrap_corridor_admissible_with_config(destination_state, search_config) {
        AdmissionDecision::Rejected(RouteAdmissionRejection::CapacityExceeded)
    } else if objective.protection_floor > FIELD_CAPABILITIES.max_protection
        || profile.selected_protection > FIELD_CAPABILITIES.max_protection
        || delivered_protection < objective.protection_floor
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::ProtectionFloorUnsatisfied)
    } else if !steady_corridor_admissible(destination_state)
        && destination_state.posterior.usability_entropy.value()
            > if search_config.node_discovery_enabled() {
                search_config.node_bootstrap_entropy_ceiling()
            } else {
                925
            }
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
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
        productive_step_bound: Limit::Bounded(u32::from(summary.hop_count_hint.value_or(1))),
        total_step_bound: Limit::Bounded(
            u32::from(summary.hop_count_hint.value_or(1)).saturating_add(2),
        ),
        route_cost,
    }
}

fn delivered_protection(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> RouteProtectionClass {
    if bootstrap_corridor_admissible_with_config(destination_state, search_config) {
        RouteProtectionClass::LinkProtected
    } else {
        RouteProtectionClass::None
    }
}

fn delivered_connectivity(
    posture: RoutingPosture,
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> ConnectivityPosture {
    let partition = if bootstrap_corridor_admissible_with_config(destination_state, search_config)
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

#[cfg(test)]
pub(crate) fn bootstrap_corridor_admissible(destination_state: &DestinationFieldState) -> bool {
    bootstrap_corridor_admissible_with_config(
        destination_state,
        &crate::FieldSearchConfig::default(),
    )
}

// long-block-exception: bootstrap admission keeps the node and service
// thresholds in one fail-closed gate over the same belief surface.
pub(crate) fn bootstrap_corridor_admissible_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> bool {
    let support = destination_state.corridor_belief.delivery_support.value();
    let entropy = destination_state.posterior.usability_entropy.value();
    let retention = destination_state.corridor_belief.retention_affinity.value();
    let top_mass = destination_state.posterior.top_corridor_mass.value();
    let evidence_class = evidence_class_from_state(destination_state);
    let service_bias = matches!(destination_state.destination, DestinationKey::Service(_));
    let discovery_enabled = !service_bias && search_config.node_discovery_enabled();
    let support_floor = if service_bias {
        130
    } else {
        search_config.node_bootstrap_support_floor()
    };
    let top_mass_floor = if service_bias {
        260
    } else {
        search_config.node_bootstrap_top_mass_floor()
    };
    let entropy_ceiling = if service_bias {
        950
    } else {
        search_config.node_bootstrap_entropy_ceiling()
    };
    let coherent_source_count = destination_state
        .frontier
        .len()
        .max(destination_state.pending_forward_evidence.len());
    let service_branch_count = service_corroborating_branch_count(destination_state);
    let service_support_score =
        service_corroborated_support_score(destination_state, &crate::FieldSearchConfig::default());

    if support < support_floor || entropy > entropy_ceiling {
        return false;
    }

    if service_bias
        && service_branch_count >= 2
        && support >= 130
        && retention >= 140
        && top_mass >= 140
        && entropy <= 970
        && service_support_score >= 380
    {
        return true;
    }

    match evidence_class {
        EvidenceContributionClass::Direct => {
            top_mass
                >= if discovery_enabled {
                    top_mass_floor.saturating_sub(80)
                } else {
                    top_mass_floor
                }
        }
        EvidenceContributionClass::ReverseFeedback => {
            top_mass
                >= if discovery_enabled {
                    top_mass_floor.saturating_sub(100)
                } else {
                    180
                }
                && (support >= support_floor.saturating_sub(40)
                    || retention >= if discovery_enabled { 140 } else { 180 }
                    || coherent_source_count >= if discovery_enabled { 1 } else { 2 })
        }
        EvidenceContributionClass::ForwardPropagated => {
            (top_mass >= 260 && retention >= 220 && support.saturating_add(retention) >= 520)
                || (coherent_source_count >= 2
                    && top_mass >= 180
                    && retention >= 160
                    && support.saturating_add(retention) >= 420)
                || (discovery_enabled
                    && coherent_source_count >= 1
                    && top_mass >= top_mass_floor.saturating_sub(90)
                    && retention >= 140
                    && support.saturating_add(retention) >= support_floor.saturating_add(160))
        }
    }
}

pub(crate) fn steady_corridor_admissible(destination_state: &DestinationFieldState) -> bool {
    destination_state.corridor_belief.delivery_support.value() >= 300
        && destination_state.posterior.usability_entropy.value() <= 850
}

#[cfg(test)]
pub(crate) fn promoted_corridor_admissible(
    destination_state: &DestinationFieldState,
    confirmation_streak: u8,
    promotion_window_score: u8,
) -> bool {
    promoted_corridor_admissible_with_config(
        destination_state,
        confirmation_streak,
        promotion_window_score,
        &crate::FieldSearchConfig::default(),
    )
}

pub(crate) fn promoted_corridor_admissible_with_config(
    destination_state: &DestinationFieldState,
    confirmation_streak: u8,
    promotion_window_score: u8,
    search_config: &crate::FieldSearchConfig,
) -> bool {
    if steady_corridor_admissible(destination_state) {
        return true;
    }
    let window_confirmed = confirmation_streak >= 1 || promotion_window_score >= 3;
    let service_bias = matches!(destination_state.destination, DestinationKey::Service(_));
    let service_branch_count = service_corroborating_branch_count(destination_state);
    let service_support_score =
        service_corroborated_support_score(destination_state, &crate::FieldSearchConfig::default());
    if service_bias
        && service_branch_count >= 2
        && destination_state.corridor_belief.delivery_support.value() >= 150
        && destination_state.posterior.usability_entropy.value() <= 950
        && destination_state.corridor_belief.retention_affinity.value() >= 160
        && service_support_score >= if window_confirmed { 420 } else { 460 }
    {
        return true;
    }
    destination_state.corridor_belief.delivery_support.value()
        >= if search_config.node_discovery_enabled() {
            search_config
                .node_bootstrap_support_floor()
                .saturating_sub(20)
                .max(180)
        } else {
            180
        }
        && destination_state.posterior.usability_entropy.value()
            <= if search_config.node_discovery_enabled() {
                search_config
                    .node_bootstrap_entropy_ceiling()
                    .saturating_sub(if window_confirmed { 20 } else { 35 })
                    .max(if window_confirmed { 940 } else { 925 })
            } else if window_confirmed {
                940
            } else {
                925
            }
        && destination_state.corridor_belief.retention_affinity.value()
            >= if window_confirmed { 220 } else { 240 }
        && destination_state.posterior.top_corridor_mass.value()
            >= if search_config.node_discovery_enabled() {
                search_config
                    .node_bootstrap_top_mass_floor()
                    .saturating_sub(if window_confirmed { 20 } else { 0 })
                    .max(if window_confirmed { 200 } else { 220 })
            } else if window_confirmed {
                200
            } else {
                220
            }
}

fn admission_class_for_state_with_config(
    destination_state: &DestinationFieldState,
    _search_config: &crate::FieldSearchConfig,
) -> FieldAdmissionClass {
    if steady_corridor_admissible(destination_state) {
        FieldAdmissionClass::SteadyAdmissible
    } else {
        FieldAdmissionClass::BootstrapAdmissible
    }
}

pub(crate) fn bootstrap_class_for_state(
    destination_state: &DestinationFieldState,
) -> FieldBootstrapClass {
    bootstrap_class_for_state_with_config(destination_state, &crate::FieldSearchConfig::default())
}

pub(crate) fn bootstrap_class_for_state_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> FieldBootstrapClass {
    match admission_class_for_state_with_config(destination_state, search_config) {
        FieldAdmissionClass::BootstrapAdmissible => FieldBootstrapClass::Bootstrap,
        FieldAdmissionClass::SteadyAdmissible => FieldBootstrapClass::Steady,
    }
}

fn degraded_steady_band_admissible_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> bool {
    let service_bias = matches!(destination_state.destination, DestinationKey::Service(_));
    let discovery_node_route = !service_bias && search_config.node_discovery_enabled();
    let support_floor = if service_bias || discovery_node_route {
        180
    } else {
        220
    };
    let retention_floor = if service_bias {
        240
    } else if discovery_node_route {
        180
    } else {
        220
    };
    let top_mass_floor = if service_bias || discovery_node_route {
        160
    } else {
        180
    };
    destination_state.corridor_belief.delivery_support.value() >= support_floor
        && destination_state.corridor_belief.retention_affinity.value() >= retention_floor
        && destination_state.posterior.top_corridor_mass.value() >= top_mass_floor
        && destination_state.posterior.usability_entropy.value()
            <= if discovery_node_route { 960 } else { 940 }
}

fn steady_route_softening_needed_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> bool {
    let service_bias = matches!(destination_state.destination, DestinationKey::Service(_));
    let discovery_node_route = !service_bias && search_config.node_discovery_enabled();
    let support = destination_state.corridor_belief.delivery_support.value();
    let retention = destination_state.corridor_belief.retention_affinity.value();
    let top_mass = destination_state.posterior.top_corridor_mass.value();
    let entropy = destination_state.posterior.usability_entropy.value();
    support < if discovery_node_route { 320 } else { 360 }
        || retention < if discovery_node_route { 260 } else { 320 }
        || top_mass < if discovery_node_route { 220 } else { 280 }
        || entropy > if discovery_node_route { 820 } else { 760 }
}

pub(crate) fn continuity_band_for_state(
    destination_state: &DestinationFieldState,
) -> FieldContinuityBand {
    continuity_band_for_state_with_config(destination_state, &crate::FieldSearchConfig::default())
}

pub(crate) fn continuity_band_for_state_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> FieldContinuityBand {
    if steady_corridor_admissible(destination_state)
        && !steady_route_softening_needed_with_config(destination_state, search_config)
    {
        FieldContinuityBand::Steady
    } else if degraded_steady_band_admissible_with_config(destination_state, search_config) {
        FieldContinuityBand::DegradedSteady
    } else {
        FieldContinuityBand::Bootstrap
    }
}

// long-block-exception: service publication narrowing keeps corroboration and
// freshness ordering in one deterministic neighbor-selection pass.
fn service_publication_neighbors(
    destination_state: &DestinationFieldState,
    selected_neighbor: jacquard_core::NodeId,
    search_config: &crate::FieldSearchConfig,
) -> Vec<jacquard_core::NodeId> {
    let mut scores: std::collections::BTreeMap<jacquard_core::NodeId, u32> =
        std::collections::BTreeMap::new();
    let freshest_forward_tick = destination_state
        .pending_forward_evidence
        .iter()
        .map(|evidence| evidence.observed_at_tick.0)
        .max()
        .unwrap_or(0);
    let freshest_frontier_tick = destination_state
        .frontier
        .as_slice()
        .iter()
        .map(|entry| entry.freshness.0)
        .max()
        .unwrap_or(0);
    for evidence in &destination_state.pending_forward_evidence {
        if evidence.summary.retention_support.value() >= 120
            && evidence.summary.delivery_support.value() >= 80
            && evidence.summary.uncertainty_penalty.value() <= 900
        {
            let freshness_gap = freshest_forward_tick.saturating_sub(evidence.observed_at_tick.0);
            let freshness_penalty =
                u32::try_from(freshness_gap.min(4)).expect("bounded freshness gap fits u32");
            let freshness_weight =
                u32::from(search_config.service_freshness_weight().clamp(25, 200));
            let score = u32::from(evidence.summary.retention_support.value())
                .saturating_add(u32::from(evidence.summary.delivery_support.value()))
                .saturating_sub(u32::from(evidence.summary.uncertainty_penalty.value()) / 2);
            let score = score
                .saturating_add(160)
                .saturating_sub((freshness_penalty * freshness_weight) / 4);
            scores
                .entry(evidence.from_neighbor)
                .and_modify(|current| *current = (*current).max(score))
                .or_insert(score);
        }
    }
    for entry in destination_state.frontier.as_slice() {
        let freshness_gap = freshest_frontier_tick.saturating_sub(entry.freshness.0);
        let freshness_penalty =
            u32::try_from(freshness_gap.min(4)).expect("bounded freshness gap fits u32");
        let freshness_weight = u32::from(search_config.service_freshness_weight().clamp(25, 200));
        let score = u32::from(entry.downstream_support.value())
            .saturating_add(u32::from(entry.net_value.value()));
        let score = score
            .saturating_add(120)
            .saturating_sub((freshness_penalty * freshness_weight) / 5);
        scores
            .entry(entry.neighbor_id)
            .and_modify(|current| *current = (*current).max(score))
            .or_insert(score);
    }
    let mut ranked: Vec<(jacquard_core::NodeId, u32)> = scores.into_iter().collect();
    ranked.sort_by(
        |(left_neighbor, left_score), (right_neighbor, right_score)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_neighbor.cmp(right_neighbor))
        },
    );
    ranked
        .into_iter()
        .filter_map(|(neighbor, _)| (neighbor != selected_neighbor).then_some(neighbor))
        .take(
            search_config
                .service_publication_neighbor_limit()
                .min(MAX_CONTINUATION_NEIGHBOR_COUNT),
        )
        .collect()
}

fn node_publication_neighbors(
    destination_state: &DestinationFieldState,
    selected_neighbor: jacquard_core::NodeId,
    search_config: &crate::FieldSearchConfig,
) -> Vec<jacquard_core::NodeId> {
    let support_floor = search_config
        .node_bootstrap_support_floor()
        .saturating_sub(20)
        .max(140);
    let mut scores: std::collections::BTreeMap<jacquard_core::NodeId, u32> =
        std::collections::BTreeMap::new();
    for evidence in &destination_state.pending_forward_evidence {
        if evidence.summary.delivery_support.value() >= support_floor.saturating_sub(20)
            && evidence.summary.uncertainty_penalty.value()
                <= search_config.node_bootstrap_entropy_ceiling()
        {
            let score = u32::from(evidence.summary.delivery_support.value())
                .saturating_add(u32::from(evidence.summary.retention_support.value()))
                .saturating_add(120);
            scores
                .entry(evidence.from_neighbor)
                .and_modify(|current| *current = (*current).max(score))
                .or_insert(score);
        }
    }
    for entry in destination_state.frontier.as_slice() {
        if entry.downstream_support.value() >= support_floor
            || corroborated_node_forward_support(destination_state, entry.neighbor_id)
                >= support_floor
        {
            let score = u32::from(entry.downstream_support.value())
                .saturating_add(u32::from(entry.net_value.value()))
                .saturating_add(80);
            scores
                .entry(entry.neighbor_id)
                .and_modify(|current| *current = (*current).max(score))
                .or_insert(score);
        }
    }
    let mut ranked: Vec<(jacquard_core::NodeId, u32)> = scores.into_iter().collect();
    ranked.sort_by(
        |(left_neighbor, left_score), (right_neighbor, right_score)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_neighbor.cmp(right_neighbor))
        },
    );
    ranked
        .into_iter()
        .filter_map(|(neighbor, _)| (neighbor != selected_neighbor).then_some(neighbor))
        .take(2.min(MAX_CONTINUATION_NEIGHBOR_COUNT))
        .collect()
}

pub(crate) fn corroborated_node_forward_support(
    destination_state: &DestinationFieldState,
    neighbor_id: jacquard_core::NodeId,
) -> u16 {
    destination_state
        .pending_forward_evidence
        .iter()
        .filter(|evidence| evidence.from_neighbor == neighbor_id)
        .map(|evidence| evidence.summary.delivery_support.value())
        .max()
        .unwrap_or(0)
}

fn service_corroborating_branch_count(destination_state: &DestinationFieldState) -> usize {
    if !matches!(destination_state.destination, DestinationKey::Service(_)) {
        return 0;
    }
    let mut neighbors = std::collections::BTreeSet::new();
    for entry in destination_state.frontier.as_slice() {
        if entry.downstream_support.value() >= 140 && entry.net_value.value() >= 180 {
            neighbors.insert(entry.neighbor_id);
        }
    }
    for evidence in &destination_state.pending_forward_evidence {
        if evidence.summary.retention_support.value() >= 120
            && evidence.summary.delivery_support.value() >= 80
            && evidence.summary.uncertainty_penalty.value() <= 900
        {
            neighbors.insert(evidence.from_neighbor);
        }
    }
    neighbors.len()
}

// long-block-exception: service corroboration scoring keeps the fused
// per-neighbor evidence buckets in one audited support calculation.
fn service_corroborated_support_score(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> u16 {
    let mut per_neighbor: std::collections::BTreeMap<jacquard_core::NodeId, u32> =
        std::collections::BTreeMap::new();
    let freshest_forward_tick = destination_state
        .pending_forward_evidence
        .iter()
        .map(|evidence| evidence.observed_at_tick.0)
        .max()
        .unwrap_or(0);
    let freshest_frontier_tick = destination_state
        .frontier
        .as_slice()
        .iter()
        .map(|entry| entry.freshness.0)
        .max()
        .unwrap_or(0);
    for entry in destination_state.frontier.as_slice() {
        let freshness_gap = freshest_frontier_tick.saturating_sub(entry.freshness.0);
        let freshness_penalty = u32::try_from(freshness_gap.min(5))
            .expect("bounded freshness gap fits u32")
            * (u32::from(search_config.service_freshness_weight().clamp(25, 200)) / 10).max(1);
        let score = u32::from(entry.downstream_support.value())
            .saturating_add(u32::from(entry.net_value.value()))
            .saturating_sub(freshness_penalty);
        per_neighbor
            .entry(entry.neighbor_id)
            .and_modify(|current| *current = (*current).max(score))
            .or_insert(score);
    }
    for evidence in &destination_state.pending_forward_evidence {
        let freshness_gap = freshest_forward_tick.saturating_sub(evidence.observed_at_tick.0);
        let freshness_penalty = u32::try_from(freshness_gap.min(5))
            .expect("bounded freshness gap fits u32")
            * (u32::from(search_config.service_freshness_weight().clamp(25, 200)) / 8).max(1);
        let score = u32::from(evidence.summary.delivery_support.value())
            .saturating_add(u32::from(evidence.summary.retention_support.value()))
            .saturating_sub(u32::from(evidence.summary.uncertainty_penalty.value()) / 3)
            .saturating_sub(freshness_penalty);
        per_neighbor
            .entry(evidence.from_neighbor)
            .and_modify(|current| *current = (*current).max(score))
            .or_insert(score);
    }
    let corroborating_count = per_neighbor.len();
    let mut branch_scores: Vec<u32> = per_neighbor.into_values().collect();
    branch_scores.sort_unstable_by(|left, right| right.cmp(left));
    let branch_mass = branch_scores.iter().take(3).copied().sum::<u32>()
        / u32::try_from(corroborating_count.clamp(1, 3)).expect("bounded branch count fits");
    let diversity_floor = branch_scores
        .get(1)
        .copied()
        .unwrap_or(branch_scores.first().copied().unwrap_or(0));
    let score = u32::from(destination_state.posterior.top_corridor_mass.value())
        .max(u32::from(
            destination_state.corridor_belief.delivery_support.value(),
        ))
        .saturating_add(branch_mass / 2)
        .saturating_add(diversity_floor / 4)
        .saturating_add(
            u32::try_from(corroborating_count.saturating_sub(1))
                .expect("branch count fits")
                .saturating_mul(70),
        );
    u16::try_from(score.min(1000)).expect("service support score capped to bucket max")
}

fn publication_confidence_for(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> u16 {
    if matches!(destination_state.destination, DestinationKey::Service(_)) {
        destination_state.posterior.top_corridor_mass.value().max(
            service_corroborated_support_score(destination_state, search_config),
        )
    } else {
        destination_state.posterior.top_corridor_mass.value()
    }
}

#[must_use]
// long-block-exception: promotion assessment keeps the bootstrap, degraded,
// and anti-entropy upgrade rules in one coherent route-state evaluation.
pub(crate) fn promotion_assessment_for_route(
    active_route: &ActiveFieldRoute,
    destination_state: &DestinationFieldState,
    best_neighbor: &crate::state::NeighborContinuation,
    now_tick: Tick,
) -> FieldPromotionAssessment {
    let confirmation_streak = active_route.bootstrap_confirmation_streak;
    let corridor_support = destination_state.corridor_belief.delivery_support.value();
    let corridor_entropy = destination_state.posterior.usability_entropy.value();
    let corridor_retention = destination_state.corridor_belief.retention_affinity.value();
    let corridor_mass = destination_state.posterior.top_corridor_mass.value();
    let promotion_window_score = active_route.promotion_window_score;
    let support_growth = destination_state.corridor_belief.delivery_support.value()
        >= active_route
            .witness_detail
            .corridor_support
            .value()
            .saturating_add(40)
        || destination_state.corridor_belief.delivery_support.value() >= 320
        || (promotion_window_score >= 2
            && corridor_support.saturating_add(25)
                >= active_route.witness_detail.corridor_support.value()
            && corridor_retention >= 280
            && corridor_mass >= 260)
        || (confirmation_streak >= 1
            && corridor_support >= 250
            && corridor_retention >= 300
            && corridor_mass >= 300);
    let uncertainty_reduced = destination_state
        .posterior
        .usability_entropy
        .value()
        .saturating_add(50)
        <= active_route.witness_detail.usability_entropy.value()
        || destination_state.posterior.usability_entropy.value() <= 775
        || (promotion_window_score >= 2
            && corridor_entropy <= 860
            && corridor_retention >= 280
            && corridor_mass >= 260)
        || (confirmation_streak >= 1 && corridor_entropy <= 840 && corridor_mass >= 300);
    let anti_entropy_confirmed = matches!(
        evidence_class_from_state(destination_state),
        EvidenceContributionClass::Direct | EvidenceContributionClass::ReverseFeedback
    ) || destination_state
        .publication
        .last_summary
        .as_ref()
        .is_some_and(|previous_summary| {
            let current_summary = FieldSummary {
                destination: SummaryDestinationKey::from(&DestinationId::from(
                    &destination_state.destination,
                )),
                topology_epoch: previous_summary.topology_epoch,
                freshness_tick: now_tick,
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
            let divergence = summary_divergence(previous_summary, &current_summary).value();
            let recent_publication =
                destination_state
                    .publication
                    .last_sent_at
                    .is_some_and(|tick| {
                        now_tick.0.saturating_sub(tick.0)
                            <= if promotion_window_score >= 2 { 8 } else { 6 }
                    });
            recent_publication
                && divergence
                    <= if confirmation_streak >= 1 || promotion_window_score >= 2 {
                        260
                    } else {
                        180
                    }
                && previous_summary.retention_support.value()
                    >= if confirmation_streak >= 1 || promotion_window_score >= 2 {
                        210
                    } else {
                        260
                    }
                && previous_summary.delivery_support.value().saturating_add(
                    if confirmation_streak >= 1 || promotion_window_score >= 2 {
                        140
                    } else {
                        80
                    },
                ) >= destination_state.corridor_belief.delivery_support.value()
                && (confirmation_streak == 0 || (corridor_retention >= 300 && corridor_mass >= 300))
        });
    let continuation_coherent = active_route
        .continuation_neighbors
        .contains(&best_neighbor.neighbor_id)
        || destination_state.frontier.len() <= 2
        || best_neighbor.net_value.value().saturating_add(120)
            >= destination_state.corridor_belief.delivery_support.value()
        || (promotion_window_score >= 2 && corridor_mass >= 260)
        || (confirmation_streak >= 1
            && best_neighbor.downstream_support.value().saturating_add(140) >= corridor_support);
    let fresh_enough = now_tick.0.saturating_sub(best_neighbor.freshness.0)
        <= if confirmation_streak >= 1 || promotion_window_score >= 2 {
            7
        } else {
            4
        };

    FieldPromotionAssessment {
        support_growth,
        uncertainty_reduced,
        anti_entropy_confirmed,
        continuation_coherent,
        fresh_enough,
    }
}

fn admission_assumptions(
    witness_detail: &FieldWitnessDetail,
    regime: OperatingRegime,
    admission_class: FieldAdmissionClass,
) -> AdmissionAssumptions {
    AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
        failure_model: match regime {
            OperatingRegime::Adversarial => FailureModelClass::ByzantineInterface,
            OperatingRegime::Unstable => FailureModelClass::CrashStop,
            _ => FailureModelClass::Benign,
        },
        runtime_envelope: RuntimeEnvelopeClass::EnvelopeAdmitted,
        node_density_class: match regime {
            OperatingRegime::Sparse => NodeDensityClass::Sparse,
            OperatingRegime::Congested => NodeDensityClass::Dense,
            OperatingRegime::RetentionFavorable
            | OperatingRegime::Unstable
            | OperatingRegime::Adversarial => NodeDensityClass::Moderate,
        },
        connectivity_regime: match regime {
            OperatingRegime::Sparse => ConnectivityRegime::Stable,
            OperatingRegime::Congested | OperatingRegime::RetentionFavorable => {
                ConnectivityRegime::PartitionProne
            }
            OperatingRegime::Unstable | OperatingRegime::Adversarial => {
                ConnectivityRegime::HighChurn
            }
        },
        adversary_regime: match regime {
            OperatingRegime::Adversarial => AdversaryRegime::ActiveAdversarial,
            OperatingRegime::Unstable => AdversaryRegime::BenignUntrusted,
            _ => AdversaryRegime::Cooperative,
        },
        claim_strength: match (
            admission_class,
            witness_detail.evidence_class,
            witness_detail.uncertainty_class,
        ) {
            (FieldAdmissionClass::BootstrapAdmissible, _, _) => ClaimStrength::InterfaceOnly,
            (
                FieldAdmissionClass::SteadyAdmissible,
                EvidenceContributionClass::Direct,
                SummaryUncertaintyClass::Low,
            ) => ClaimStrength::ConservativeUnderProfile,
            (_, _, SummaryUncertaintyClass::High) => ClaimStrength::InterfaceOnly,
            _ => ClaimStrength::ConservativeUnderProfile,
        },
    }
}

fn route_cost_for(
    corridor: &CorridorBeliefEnvelope,
    continuation_neighbor_count: usize,
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
            u32::try_from(continuation_neighbor_count)
                .expect("continuation neighbor count fits u32"),
        ),
        hold_bytes_reserved: Limit::Bounded(hold_bytes_reserved),
        work_step_count_max: Limit::Bounded(
            u32::from(hop_count)
                .saturating_add(
                    u32::try_from(continuation_neighbor_count)
                        .expect("continuation neighbor count fits u32"),
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
        0..=249 => SummaryUncertaintyClass::Low,
        250..=599 => SummaryUncertaintyClass::Medium,
        _ => SummaryUncertaintyClass::High,
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, Environment,
        FactSourceClass, Observation, OriginAuthenticationClass, RatioPermille, RouteEpoch,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
        RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick,
    };
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::RoutingEnginePlanner;

    use super::*;
    use crate::state::{DestinationInterestClass, HopBand, NeighborContinuation, SupportBucket};

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
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
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
        state.posterior.predicted_observation_class = crate::state::ObservationClass::DirectOnly;
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
    fn discovery_config_admits_single_source_node_bootstrap_corridor() {
        let mut engine = FieldEngine::new(node(1), (), ()).with_search_config(
            crate::FieldSearchConfig::default()
                .with_node_bootstrap_support_floor(180)
                .with_node_bootstrap_top_mass_floor(180)
                .with_node_bootstrap_entropy_ceiling(970)
                .enable_node_discovery(),
        );
        let state = engine.state.upsert_destination_interest(
            &DestinationId::Node(node(2)),
            DestinationInterestClass::Transit,
            Tick(4),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(180);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(940);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.corridor_belief.expected_hop_band = HopBand::new(2, 4);
        state.corridor_belief.delivery_support = SupportBucket::new(185);
        state.corridor_belief.retention_affinity = SupportBucket::new(165);
        state.frontier = state.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(2),
            net_value: SupportBucket::new(260),
            downstream_support: SupportBucket::new(220),
            expected_hop_band: HopBand::new(2, 4),
            freshness: Tick(4),
        });
        let topology = supported_topology();
        let objective = sample_objective(node(2));
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let admission = engine
            .admit_route(&objective, &sample_profile(), candidate, &topology)
            .expect("discovery bootstrap corridor should admit");
        assert_eq!(
            admission.admission_check.decision,
            AdmissionDecision::Admissible
        );
    }

    #[test]
    fn admit_route_allows_weak_bootstrap_corridor_when_retention_is_coherent() {
        let mut engine = seeded_engine();
        let state = engine
            .state
            .destinations
            .get_mut(&DestinationKey::from(&DestinationId::Node(node(2))))
            .expect("seeded destination");
        state.posterior.top_corridor_mass = SupportBucket::new(320);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(900);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.corridor_belief.delivery_support = SupportBucket::new(230);
        state.corridor_belief.retention_affinity = SupportBucket::new(320);
        let topology = supported_topology();
        let objective = sample_objective(node(2));
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let admission = engine
            .admit_route(&objective, &sample_profile(), candidate, &topology)
            .expect("weak corridor should still admit");
        assert_eq!(
            admission.admission_check.decision,
            AdmissionDecision::Admissible
        );
        assert_eq!(
            admission.summary.protection,
            RouteProtectionClass::LinkProtected
        );
        assert!(matches!(
            admission.witness.degradation,
            RouteDegradation::Degraded(_)
        ));
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
            .witness_detail_for_destination(&DestinationKey::from(&DestinationId::Node(node(2))))
            .expect("detail");
        assert_eq!(detail.regime, engine.state.regime.current);
        assert_eq!(detail.posture, engine.state.posture.current);
        assert_eq!(detail.uncertainty_class, SummaryUncertaintyClass::Low);
    }

    #[test]
    fn promoted_corridor_admissible_accepts_confirmed_bootstrap_bridge() {
        let mut engine = seeded_engine();
        let state = engine
            .state
            .destinations
            .get_mut(&DestinationKey::from(&DestinationId::Node(node(2))))
            .expect("seeded destination");
        state.posterior.top_corridor_mass = SupportBucket::new(320);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(880);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.corridor_belief.delivery_support = SupportBucket::new(270);
        state.corridor_belief.retention_affinity = SupportBucket::new(340);

        assert!(!steady_corridor_admissible(state));
        assert!(promoted_corridor_admissible(state, 2, 0));
    }

    #[test]
    fn promoted_corridor_admissible_accepts_window_confirmed_bridge() {
        let mut engine = seeded_engine();
        let state = engine
            .state
            .destinations
            .get_mut(&DestinationKey::from(&DestinationId::Node(node(2))))
            .expect("seeded destination");
        state.posterior.top_corridor_mass = SupportBucket::new(250);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(930);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.corridor_belief.delivery_support = SupportBucket::new(220);
        state.corridor_belief.retention_affinity = SupportBucket::new(280);

        assert!(!steady_corridor_admissible(state));
        assert!(promoted_corridor_admissible(state, 0, 3));
    }

    #[test]
    fn service_bootstrap_corridor_accepts_corroborated_fanout() {
        let mut engine = FieldEngine::new(node(1), (), ());
        let destination_id = DestinationId::Service(jacquard_core::ServiceId(vec![5; 16]));
        let state = engine.state.upsert_destination_interest(
            &destination_id,
            crate::state::DestinationInterestClass::Transit,
            Tick(4),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(170);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(930);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
        state.corridor_belief.delivery_support = SupportBucket::new(150);
        state.corridor_belief.retention_affinity = SupportBucket::new(180);
        for (neighbor, support) in [(node(2), 220_u16), (node(3), 210), (node(4), 190)] {
            state.frontier = state.frontier.clone().insert(NeighborContinuation {
                neighbor_id: neighbor,
                net_value: SupportBucket::new(support.saturating_add(40)),
                downstream_support: SupportBucket::new(support),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(4),
            });
            state
                .pending_forward_evidence
                .push(crate::summary::ForwardPropagatedEvidence {
                    from_neighbor: neighbor,
                    summary: FieldSummary {
                        destination: SummaryDestinationKey::from(&destination_id),
                        topology_epoch: RouteEpoch(1),
                        freshness_tick: Tick(4),
                        hop_band: HopBand::new(1, 2),
                        delivery_support: SupportBucket::new(support),
                        congestion_penalty: crate::state::EntropyBucket::default(),
                        retention_support: SupportBucket::new(support.saturating_add(20)),
                        uncertainty_penalty: crate::state::EntropyBucket::new(220),
                        evidence_class: EvidenceContributionClass::ForwardPropagated,
                        uncertainty_class: SummaryUncertaintyClass::Low,
                    },
                    observed_at_tick: Tick(4),
                });
        }

        assert!(bootstrap_corridor_admissible(state));
    }

    #[test]
    fn service_route_publication_carries_multi_branch_corridor() {
        let mut engine = FieldEngine::new(node(1), (), ());
        let destination_id = DestinationId::Service(jacquard_core::ServiceId(vec![6; 16]));
        let state = engine.state.upsert_destination_interest(
            &destination_id,
            crate::state::DestinationInterestClass::Transit,
            Tick(4),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(220);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(900);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
        state.corridor_belief.delivery_support = SupportBucket::new(170);
        state.corridor_belief.retention_affinity = SupportBucket::new(220);
        for (neighbor, support) in [(node(2), 260_u16), (node(3), 230), (node(4), 210)] {
            state.frontier = state.frontier.clone().insert(NeighborContinuation {
                neighbor_id: neighbor,
                net_value: SupportBucket::new(support.saturating_add(30)),
                downstream_support: SupportBucket::new(support),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(4),
            });
            state
                .pending_forward_evidence
                .push(crate::summary::ForwardPropagatedEvidence {
                    from_neighbor: neighbor,
                    summary: FieldSummary {
                        destination: SummaryDestinationKey::from(&destination_id),
                        topology_epoch: RouteEpoch(1),
                        freshness_tick: Tick(4),
                        hop_band: HopBand::new(1, 2),
                        delivery_support: SupportBucket::new(support),
                        congestion_penalty: crate::state::EntropyBucket::default(),
                        retention_support: SupportBucket::new(support.saturating_add(20)),
                        uncertainty_penalty: crate::state::EntropyBucket::new(220),
                        evidence_class: EvidenceContributionClass::ForwardPropagated,
                        uncertainty_class: SummaryUncertaintyClass::Low,
                    },
                    observed_at_tick: Tick(4),
                });
        }
        let topology = supported_topology();
        let objective = RoutingObjective {
            destination: destination_id,
            ..sample_objective(node(2))
        };
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let token = crate::route::decode_backend_token(&candidate.backend_ref.backend_route_id)
            .expect("field backend token");
        assert!(
            token.continuation_neighbors.len() >= 3,
            "service continuation envelope: {:?}",
            token.continuation_neighbors
        );
    }

    #[test]
    fn service_publication_prefers_fresh_corroborated_neighbors() {
        let mut engine = FieldEngine::new(node(1), (), ());
        let destination_id = DestinationId::Service(jacquard_core::ServiceId(vec![7; 16]));
        let state = engine.state.upsert_destination_interest(
            &destination_id,
            crate::state::DestinationInterestClass::Transit,
            Tick(8),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(260);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(860);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
        state.corridor_belief.delivery_support = SupportBucket::new(180);
        state.corridor_belief.retention_affinity = SupportBucket::new(240);
        for (neighbor, support, freshness) in [
            (node(2), 260_u16, Tick(8)),
            (node(3), 240, Tick(8)),
            (node(4), 250, Tick(2)),
        ] {
            state.frontier = state.frontier.clone().insert(NeighborContinuation {
                neighbor_id: neighbor,
                net_value: SupportBucket::new(support.saturating_add(30)),
                downstream_support: SupportBucket::new(support),
                expected_hop_band: HopBand::new(1, 2),
                freshness,
            });
        }

        let published =
            service_publication_neighbors(state, node(2), &crate::FieldSearchConfig::default());
        assert!(published.contains(&node(3)));
        assert!(!published.is_empty());
        assert!(
            published.first() == Some(&node(3)),
            "published neighbors should favor fresh corroborated branches: {:?}",
            published
        );
    }

    #[test]
    fn service_publication_confidence_prefers_diverse_fresh_corridors() {
        let mut engine = FieldEngine::new(node(1), (), ());
        let destination_id = DestinationId::Service(jacquard_core::ServiceId(vec![8; 16]));
        let strong = engine.state.upsert_destination_interest(
            &destination_id,
            crate::state::DestinationInterestClass::Transit,
            Tick(8),
        );
        strong.posterior.top_corridor_mass = SupportBucket::new(220);
        strong.corridor_belief.delivery_support = SupportBucket::new(180);
        strong.corridor_belief.retention_affinity = SupportBucket::new(240);
        strong.frontier = strong.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(2),
            net_value: SupportBucket::new(320),
            downstream_support: SupportBucket::new(260),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(8),
        });
        strong.frontier = strong.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(3),
            net_value: SupportBucket::new(300),
            downstream_support: SupportBucket::new(240),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(8),
        });

        let mut weak_engine = FieldEngine::new(node(1), (), ());
        let weak = weak_engine.state.upsert_destination_interest(
            &destination_id,
            crate::state::DestinationInterestClass::Transit,
            Tick(8),
        );
        weak.posterior.top_corridor_mass = SupportBucket::new(220);
        weak.corridor_belief.delivery_support = SupportBucket::new(180);
        weak.corridor_belief.retention_affinity = SupportBucket::new(240);
        weak.frontier = weak.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(2),
            net_value: SupportBucket::new(320),
            downstream_support: SupportBucket::new(260),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(3),
        });

        assert!(
            publication_confidence_for(strong, &crate::FieldSearchConfig::default())
                > publication_confidence_for(weak, &crate::FieldSearchConfig::default()),
            "diverse fresh service corridor should have higher publication confidence"
        );
    }

    #[test]
    fn service_publication_limit_constrains_extra_neighbors() {
        let mut engine = FieldEngine::new(node(1), (), ());
        let destination_id = DestinationId::Service(jacquard_core::ServiceId(vec![8; 16]));
        let state = engine.state.upsert_destination_interest(
            &destination_id,
            crate::state::DestinationInterestClass::Transit,
            Tick(8),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(280);
        state.corridor_belief.delivery_support = SupportBucket::new(220);
        state.corridor_belief.retention_affinity = SupportBucket::new(280);
        for neighbor in [node(2), node(3), node(4)] {
            state.frontier = state.frontier.clone().insert(NeighborContinuation {
                neighbor_id: neighbor,
                net_value: SupportBucket::new(320),
                downstream_support: SupportBucket::new(260),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(8),
            });
        }

        let constrained = service_publication_neighbors(
            state,
            node(2),
            &crate::FieldSearchConfig::default().with_service_publication_neighbor_limit(1),
        );
        let broad = service_publication_neighbors(
            state,
            node(2),
            &crate::FieldSearchConfig::default().with_service_publication_neighbor_limit(3),
        );

        assert_eq!(constrained.len(), 1);
        assert!(broad.len() >= 2);
    }
}
