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

pub(crate) mod admission;
pub(crate) mod promotion;
pub(crate) mod publication;

use admission::{
    admission_assumptions, admission_check_for, admission_class_for_state_with_config,
    bootstrap_class_for_state_with_config, continuity_band_for_state_with_config,
    delivered_connectivity, delivered_protection, evidence_class_from_state, rejected_check,
    route_cost_for, selected_neighbor_publishable, uncertainty_class_for, AdmissionInputs,
};
#[cfg(test)]
use admission::{bootstrap_corridor_admissible, promoted_corridor_admissible};
use publication::{
    node_publication_neighbors, publication_confidence_for, service_publication_neighbors,
};

use jacquard_core::{
    AdmissionDecision, BackendRouteRef, Belief, Configuration, DestinationId, Estimate,
    ObjectiveVsDelivered, Observation, RouteAdmission, RouteAdmissionCheck,
    RouteAdmissionRejection, RouteCandidate, RouteDegradation, RouteError, RouteEstimate,
    RouteSelectionError, RouteSummary, RouteWitness, RoutingEngineCapabilities, RoutingEngineId,
    SelectedRoutingParameters,
};
use jacquard_traits::RoutingEnginePlanner;

use crate::{
    attractor::rank_frontier_by_attractor,
    route::{encode_backend_token, route_id_for_backend, FieldBackendToken, FieldWitnessDetail},
    state::{DestinationFieldState, DestinationKey, MAX_CONTINUATION_NEIGHBOR_COUNT},
    summary::{derive_degradation_class, FieldSummary, SummaryDestinationKey},
    FieldEngine, FIELD_CAPABILITIES, FIELD_ENGINE_ID,
};

struct PlanningArtifacts {
    candidate: RouteCandidate,
    admission_check: RouteAdmissionCheck,
    witness: RouteWitness,
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

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        ByteCount, ClaimStrength, Configuration, ConnectivityPosture, ControllerId, DestinationId,
        Environment, FactSourceClass, Limit, Observation, OriginAuthenticationClass, RatioPermille,
        RouteEpoch, RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
        RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick,
    };
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::RoutingEnginePlanner;

    use super::*;
    use crate::planner::admission::steady_corridor_admissible;
    use crate::state::{DestinationInterestClass, HopBand, NeighborContinuation, SupportBucket};
    use crate::summary::{EvidenceContributionClass, SummaryUncertaintyClass};

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
