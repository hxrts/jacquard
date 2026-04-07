//! `RoutingEnginePlanner` implementation for `MeshEngine`.
//!
//! Candidate production runs a five-step deterministic pipeline: BFS from
//! the local node, filter by engine capability and objective match, derive
//! a self-contained `BackendRouteId` plan token plus admission check,
//! sort by hop count, mesh-private topology-model preference, and
//! deterministic order key, then truncate to
//! `MESH_CANDIDATE_COUNT_MAX`. `check_candidate` and `admit_route` take
//! topology explicitly and re-derive from the plan token on cache miss,
//! so the candidate cache is an optimization rather than a required
//! piece of engine state.

use std::cmp::Reverse;

use jacquard_core::{
    AdaptiveRoutingProfile, AdmissionAssumptions, AdmissionDecision, BackendRouteId, Belief,
    CommitteeSelection, Configuration, DestinationId, Estimate, Limit, NodeId, Observation,
    RouteAdmission, RouteAdmissionCheck, RouteAdmissionRejection, RouteCandidate,
    RouteConnectivityProfile, RouteCost, RouteError, RouteEstimate, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteSelectionError, RouteServiceKind, RouteSummary,
    RouteWitness, RoutingObjective, Tick, TimeWindow, ROUTE_HOP_COUNT_MAX,
};
use jacquard_traits::RoutingEnginePlanner;

use super::{
    support::{
        confidence_for_segments, decode_backend_token, degradation_for_candidate,
        deterministic_order_key, encode_backend_token, encode_path_bytes,
        node_path_from_plan_token, route_cost_for_segments, shortest_paths, unique_protocol_mix,
        MeshPlanToken,
    },
    CachedCandidate, MeshEngine, MeshHasherBounds, MeshRouteClass, MeshRouteSegment,
    MeshSelectorBounds, MESH_CANDIDATE_COUNT_MAX, MESH_CANDIDATE_VALIDITY_TICKS, MESH_CAPABILITIES,
    MESH_ENGINE_ID,
};
use crate::{
    committee::mesh_admission_assumptions,
    topology::{
        estimate_hop_link, objective_matches_node, optional_health_score_value,
        route_capable_for_engine,
    },
};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
{
    fn candidate_preference_score(
        &self,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
    ) -> u32 {
        let first_hop = node_path.get(1).copied();
        let peer_score = first_hop
            .and_then(|peer_node_id| {
                self.topology_model.peer_estimate(
                    &self.local_node_id,
                    &peer_node_id,
                    topology.observed_at_tick,
                    &topology.value,
                )
            })
            .map(|estimate| {
                optional_health_score_value(estimate.relay_value_score)
                    .saturating_add(optional_health_score_value(estimate.retention_value_score))
                    .saturating_add(optional_health_score_value(estimate.stability_score))
                    .saturating_add(optional_health_score_value(estimate.service_score))
            })
            .unwrap_or(0);
        let neighborhood = self.topology_model.neighborhood_estimate(
            &self.local_node_id,
            topology.observed_at_tick,
            &topology.value,
        );
        let neighborhood_bonus = neighborhood
            .as_ref()
            .map(|estimate| {
                optional_health_score_value(estimate.density_score).saturating_add(
                    optional_health_score_value(estimate.service_stability_score),
                )
            })
            .unwrap_or(0);
        let neighborhood_penalty = neighborhood
            .as_ref()
            .map(|estimate| {
                optional_health_score_value(estimate.repair_pressure_score)
                    .saturating_add(optional_health_score_value(estimate.partition_risk_score))
            })
            .unwrap_or(0);
        peer_score
            .saturating_add(neighborhood_bonus)
            .saturating_sub(neighborhood_penalty)
    }

    // Route class order of precedence: a Gateway destination always
    // yields Gateway; otherwise multi-hop routes to hold-capable
    // destinations with hold-fallback allowed become DeferredDelivery;
    // single-hop routes are Direct; everything else is MultiHop.
    fn determine_route_class(
        &self,
        objective: &RoutingObjective,
        hop_count: usize,
        hold_capable: bool,
    ) -> MeshRouteClass {
        if matches!(objective.destination, DestinationId::Gateway(_)) {
            MeshRouteClass::Gateway
        } else if hold_capable
            && objective.hold_fallback_policy == jacquard_core::HoldFallbackPolicy::Allowed
            && hop_count > 1
        {
            MeshRouteClass::DeferredDelivery
        } else if hop_count <= 1 {
            MeshRouteClass::Direct
        } else {
            MeshRouteClass::MultiHop
        }
    }

    fn route_connectivity_for_class(
        &self,
        objective: &RoutingObjective,
        route_class: &MeshRouteClass,
    ) -> RouteConnectivityProfile {
        match route_class {
            MeshRouteClass::DeferredDelivery => RouteConnectivityProfile {
                repair: RouteRepairClass::Repairable,
                partition: if objective.hold_fallback_policy
                    == jacquard_core::HoldFallbackPolicy::Allowed
                {
                    RoutePartitionClass::PartitionTolerant
                } else {
                    RoutePartitionClass::ConnectedOnly
                },
            },
            _ => RouteConnectivityProfile {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
        }
    }

    fn derive_segments(
        &self,
        configuration: &Configuration,
        node_path: &[NodeId],
    ) -> Option<Vec<MeshRouteSegment>> {
        let segments = node_path
            .windows(2)
            .filter_map(|pair| {
                estimate_hop_link(&pair[0], &pair[1], configuration).map(|(endpoint, _)| {
                    MeshRouteSegment {
                        node_id: pair[1],
                        endpoint,
                    }
                })
            })
            .collect::<Vec<_>>();
        // Paths that would exceed the workspace hop limit are rejected
        // here rather than silently truncated. This keeps the hop count
        // representable in a u8 everywhere downstream.
        if segments.is_empty() || segments.len() > usize::from(ROUTE_HOP_COUNT_MAX) {
            return None;
        }
        Some(segments)
    }

    fn hold_capable_for_destination(
        &self,
        destination_node: &jacquard_core::Node,
        observed_at_tick: Tick,
    ) -> bool {
        let service_advertised = destination_node.profile.services.iter().any(|service| {
            service.service_kind == RouteServiceKind::Hold
                && service.routing_engines.contains(&MESH_ENGINE_ID)
                && service.valid_for.contains(observed_at_tick)
                && matches!(
                    service.capacity,
                    Belief::Estimated(Estimate {
                        value: jacquard_core::CapacityHint {
                            hold_capacity_bytes: Belief::Estimated(Estimate { value, .. }),
                            ..
                        },
                        ..
                    }) if value.0 > 0
                )
        });
        let state_ready = matches!(
            destination_node.state.hold_capacity_available_bytes,
            Belief::Estimated(Estimate { value, .. }) if value.0 > 0
        );

        // Deferred delivery is only honest when the destination both
        // advertises Hold for mesh and currently reports positive hold
        // capacity in shared node state. Advertisement alone is a
        // capability claim, not current readiness.
        service_advertised && state_ready
    }

    fn build_candidate_summary(
        &self,
        topology: &Observation<Configuration>,
        connectivity: RouteConnectivityProfile,
        segments: &[MeshRouteSegment],
        valid_for: TimeWindow,
    ) -> RouteSummary {
        RouteSummary {
            engine: MESH_ENGINE_ID,
            protection: RouteProtectionClass::LinkProtected,
            connectivity,
            protocol_mix: unique_protocol_mix(segments),
            hop_count_hint: Belief::Estimated(Estimate {
                value: u8::try_from(segments.len())
                    .expect("segment count is bounded by ROUTE_HOP_COUNT_MAX"),
                confidence_permille: jacquard_core::RatioPermille(1000),
                updated_at_tick: topology.observed_at_tick,
            }),
            valid_for,
        }
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
{
    fn build_candidate_estimate(
        &self,
        topology: &Observation<Configuration>,
        connectivity: RouteConnectivityProfile,
        route_class: &MeshRouteClass,
        segments: &[MeshRouteSegment],
    ) -> Estimate<RouteEstimate> {
        let configuration = &topology.value;
        Estimate {
            value: RouteEstimate {
                estimated_protection: RouteProtectionClass::LinkProtected,
                estimated_connectivity: connectivity,
                topology_epoch: configuration.epoch,
                degradation: degradation_for_candidate(configuration, route_class),
            },
            confidence_permille: confidence_for_segments(segments, configuration),
            updated_at_tick: topology.observed_at_tick,
        }
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Selector: MeshSelectorBounds,
{
    fn maybe_select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Result<Option<CommitteeSelection>, RouteError> {
        self.selector.as_ref().map_or(Ok(None), |selector| {
            selector.select_committee(objective, profile, topology)
        })
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    // Assembles a candidate from a BFS node path: segments, route class,
    // validity window, plan token, route id, cost, summary, estimate,
    // admission assumptions, admission check, witness, order key, and
    // optional committee. Every derivation is a function of the inputs
    // so re-running on cache miss produces the same CachedCandidate.
    fn candidate_for_path(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        destination_node: &jacquard_core::Node,
    ) -> Option<(BackendRouteId, CachedCandidate)> {
        let segments = self.derive_segments(&topology.value, node_path)?;
        let hold_capable =
            self.hold_capable_for_destination(destination_node, topology.observed_at_tick);
        let route_class = self.determine_route_class(objective, segments.len(), hold_capable);
        let connectivity = self.route_connectivity_for_class(objective, &route_class);
        let valid_for = TimeWindow::new(
            topology.observed_at_tick,
            Tick(topology.observed_at_tick.0 + MESH_CANDIDATE_VALIDITY_TICKS),
        )
        .expect("mesh candidates always use a positive validity window");
        let committee = self.maybe_select_committee(objective, profile, topology);
        let selected_committee = match &committee {
            Ok(selected) => selected.clone(),
            Err(_) => None,
        };
        let plan = MeshPlanToken {
            epoch: topology.value.epoch,
            source: self.local_node_id,
            destination: objective.destination.clone(),
            segments: segments.clone(),
            valid_for,
            route_class: route_class.clone(),
            committee: selected_committee.clone(),
        };
        let path_bytes = encode_path_bytes(node_path, &segments);
        let backend_route_id = encode_backend_token(&plan);
        let route_id = self.route_id_for_backend(&backend_route_id);
        let route_cost = route_cost_for_segments(&segments, &route_class);
        let summary = self.build_candidate_summary(topology, connectivity, &segments, valid_for);
        let estimate =
            self.build_candidate_estimate(topology, connectivity, &route_class, &segments);
        let admission_assumptions = mesh_admission_assumptions(profile, &topology.value);
        let mut admission_check = mesh_admission_check(
            objective,
            profile,
            &summary,
            &route_cost,
            &admission_assumptions,
        );
        if committee.is_err() {
            admission_check.decision =
                AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable);
        }
        let witness = RouteWitness {
            objective_protection: objective.target_protection,
            delivered_protection: summary.protection,
            objective_connectivity: objective.target_connectivity,
            delivered_connectivity: summary.connectivity,
            admission_profile: admission_assumptions,
            topology_epoch: topology.value.epoch,
            degradation: estimate.value.degradation,
        };
        let ordering_key = deterministic_order_key(route_id, &self.hashing, &path_bytes);
        Some((
            backend_route_id,
            CachedCandidate {
                route_id,
                summary,
                estimate,
                admission_check,
                witness,
                ordering_key,
            },
        ))
    }

    // The cache-miss path for `check_candidate` and `admit_route`.
    // Decodes the self-contained plan token, verifies that the explicit
    // topology still supports the encoded path and route class, then
    // re-derives the shared planning judgment. Materialization later
    // decodes the same token without consulting the candidate cache.
    fn derive_candidate_from_backend_ref(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
        backend_route_id: &BackendRouteId,
    ) -> Result<CachedCandidate, RouteError> {
        let plan =
            decode_backend_token(backend_route_id).ok_or(RouteSelectionError::NoCandidate)?;
        if plan.source != self.local_node_id || plan.destination != objective.destination {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        let node_path = node_path_from_plan_token(&plan);
        let destination_node_id = *node_path.last().ok_or(RouteSelectionError::NoCandidate)?;
        let destination_node = topology
            .value
            .nodes
            .get(&destination_node_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let hold_capable =
            self.hold_capable_for_destination(destination_node, topology.observed_at_tick);
        let route_class = self.determine_route_class(objective, plan.segments.len(), hold_capable);
        if route_class != plan.route_class {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        let derived_segments = self
            .derive_segments(&topology.value, &node_path)
            .ok_or(RouteSelectionError::NoCandidate)?;
        if derived_segments != plan.segments {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        let (derived_backend_ref, candidate) = self
            .candidate_for_path(objective, profile, topology, &node_path, destination_node)
            .ok_or(RouteSelectionError::NoCandidate)?;
        if &derived_backend_ref != backend_route_id {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        Ok(candidate)
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> RoutingEnginePlanner
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn engine_id(&self) -> jacquard_core::RoutingEngineId {
        MESH_ENGINE_ID
    }

    fn capabilities(&self) -> jacquard_core::RoutingEngineCapabilities {
        MESH_CAPABILITIES.clone()
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        // Five-step deterministic pipeline: BFS shortest paths, filter
        // to route-capable destinations matching the objective, build a
        // cached candidate per path, sort by hop count, topology-model
        // preference, and order key, then truncate to
        // MESH_CANDIDATE_COUNT_MAX. The deterministic sort makes
        // candidate ordering stable across replays.
        let configuration = &topology.value;
        let mut cached = shortest_paths(&self.local_node_id, configuration)
            .into_iter()
            .filter(|(destination_node_id, _)| *destination_node_id != self.local_node_id)
            .filter_map(|(destination_node_id, node_path)| {
                let destination_node = configuration.nodes.get(&destination_node_id)?;
                if !route_capable_for_engine(
                    destination_node,
                    &MESH_ENGINE_ID,
                    topology.observed_at_tick,
                ) {
                    return None;
                }
                if !objective_matches_node(
                    &destination_node_id,
                    destination_node,
                    objective,
                    &MESH_ENGINE_ID,
                    topology.observed_at_tick,
                ) {
                    return None;
                }
                self.candidate_for_path(objective, profile, topology, &node_path, destination_node)
            })
            .collect::<Vec<_>>();

        cached.sort_by_key(|(backend_route_id, candidate)| {
            let preference = decode_backend_token(backend_route_id)
                .map(|plan| {
                    let node_path = node_path_from_plan_token(&plan);
                    self.candidate_preference_score(topology, &node_path)
                })
                .unwrap_or(0);
            (
                usize::from(candidate.admission_check.route_cost.hop_count),
                Reverse(preference),
                candidate.ordering_key.stable_key,
                candidate.ordering_key.tie_break,
            )
        });
        cached.truncate(MESH_CANDIDATE_COUNT_MAX);

        let mut cache = self.candidate_cache.borrow_mut();
        cache.clear();

        cached
            .into_iter()
            .map(|(backend_route_id, candidate)| {
                cache.insert(backend_route_id.clone(), candidate.clone());
                RouteCandidate {
                    summary: candidate.summary,
                    estimate: candidate.estimate,
                    backend_ref: jacquard_core::BackendRouteRef {
                        engine: MESH_ENGINE_ID,
                        backend_route_id,
                    },
                }
            })
            .collect()
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        // Cache hit is the fast path. On cache miss (e.g. after an
        // engine_tick cleared the cache) we re-derive from the plan
        // token against the supplied topology. Same inputs produce the
        // same admission check either way.
        if let Some(cached) = self
            .candidate_cache
            .borrow()
            .get(&candidate.backend_ref.backend_route_id)
        {
            return Ok(cached.admission_check.clone());
        }
        let derived = self.derive_candidate_from_backend_ref(
            objective,
            profile,
            topology,
            &candidate.backend_ref.backend_route_id,
        )?;
        Ok(derived.admission_check)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        let cached = self
            .candidate_cache
            .borrow()
            .get(&candidate.backend_ref.backend_route_id)
            .cloned()
            .map_or_else(
                || {
                    self.derive_candidate_from_backend_ref(
                        objective,
                        profile,
                        topology,
                        &candidate.backend_ref.backend_route_id,
                    )
                },
                Ok,
            )?;

        match cached.admission_check.decision {
            AdmissionDecision::Admissible => Ok(RouteAdmission {
                route_id: cached.route_id,
                backend_ref: candidate.backend_ref,
                objective: objective.clone(),
                profile: profile.clone(),
                admission_check: cached.admission_check,
                summary: cached.summary,
                witness: cached.witness,
            }),
            AdmissionDecision::Rejected(rejection) => {
                Err(RouteSelectionError::Inadmissible(rejection).into())
            }
        }
    }
}

// Admission has three rejection paths and one admit path. Order
// matters: the protection floor is the hard security invariant so it
// is checked first. Repair and partition mismatches are the
// profile-driven connectivity requirements checked only after
// protection passes.
fn mesh_admission_check(
    objective: &RoutingObjective,
    profile: &AdaptiveRoutingProfile,
    summary: &RouteSummary,
    route_cost: &RouteCost,
    assumptions: &AdmissionAssumptions,
) -> RouteAdmissionCheck {
    let decision = if summary.protection < objective.protection_floor {
        AdmissionDecision::Rejected(RouteAdmissionRejection::ProtectionFloorUnsatisfied)
    } else if profile.selected_connectivity.repair == RouteRepairClass::Repairable
        && summary.connectivity.repair != RouteRepairClass::Repairable
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::BranchingInfeasible)
    } else if profile.selected_connectivity.partition == RoutePartitionClass::PartitionTolerant
        && summary.connectivity.partition != RoutePartitionClass::PartitionTolerant
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable)
    } else {
        AdmissionDecision::Admissible
    };

    RouteAdmissionCheck {
        decision,
        profile: assumptions.clone(),
        productive_step_bound: Limit::Bounded(route_cost.hop_count.into()),
        total_step_bound: Limit::Bounded(route_cost.hop_count.into()),
        route_cost: route_cost.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MESH_ENGINE_ID;
    use jacquard_core::{
        AdversaryRegime, ClaimStrength, ConnectivityRegime, FailureModelClass, HoldFallbackPolicy,
        MessageFlowAssumptionClass, NodeDensityClass, RatioPermille, RuntimeEnvelopeClass, Tick,
    };

    fn neutral_assumptions() -> AdmissionAssumptions {
        AdmissionAssumptions {
            message_flow_assumption: MessageFlowAssumptionClass::PerRouteSequenced,
            failure_model: FailureModelClass::Benign,
            runtime_envelope: RuntimeEnvelopeClass::Canonical,
            node_density_class: NodeDensityClass::Sparse,
            connectivity_regime: ConnectivityRegime::Stable,
            adversary_regime: AdversaryRegime::BenignUntrusted,
            claim_strength: ClaimStrength::ConservativeUnderProfile,
        }
    }

    fn objective_with_floor(floor: RouteProtectionClass) -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(NodeId([3; 32])),
            service_kind: RouteServiceKind::Move,
            target_protection: floor,
            protection_floor: floor,
            target_connectivity: RouteConnectivityProfile {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: HoldFallbackPolicy::Allowed,
            latency_budget_ms: Limit::Unbounded,
            protection_priority: jacquard_core::PriorityPoints(0),
            connectivity_priority: jacquard_core::PriorityPoints(0),
        }
    }

    fn profile_with(
        repair: RouteRepairClass,
        partition: RoutePartitionClass,
    ) -> AdaptiveRoutingProfile {
        AdaptiveRoutingProfile {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: RouteConnectivityProfile { repair, partition },
            deployment_profile: jacquard_core::DeploymentProfile::FieldPartitionTolerant,
            diversity_floor: 1,
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn summary_with(
        protection: RouteProtectionClass,
        repair: RouteRepairClass,
        partition: RoutePartitionClass,
    ) -> RouteSummary {
        RouteSummary {
            engine: MESH_ENGINE_ID,
            protection,
            connectivity: RouteConnectivityProfile { repair, partition },
            protocol_mix: Vec::new(),
            hop_count_hint: Belief::Estimated(Estimate {
                value: 1_u8,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(0),
            }),
            valid_for: TimeWindow::new(Tick(0), Tick(100)).unwrap(),
        }
    }

    fn unit_route_cost() -> RouteCost {
        RouteCost {
            message_count_max: Limit::Bounded(1),
            byte_count_max: Limit::Bounded(jacquard_core::ByteCount(1024)),
            hop_count: 1,
            repair_attempt_count_max: Limit::Bounded(1),
            hold_bytes_reserved: Limit::Bounded(jacquard_core::ByteCount(0)),
            work_step_count_max: Limit::Bounded(2),
        }
    }

    #[test]
    fn admission_check_rejects_protection_floor_regression() {
        let objective = objective_with_floor(RouteProtectionClass::TopologyProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
        );
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(RouteAdmissionRejection::ProtectionFloorUnsatisfied),
        );
    }

    #[test]
    fn admission_check_rejects_repair_mismatch() {
        let objective = objective_with_floor(RouteProtectionClass::LinkProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::BestEffort,
            RoutePartitionClass::ConnectedOnly,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
        );
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(RouteAdmissionRejection::BranchingInfeasible),
        );
    }

    #[test]
    fn admission_check_rejects_partition_mismatch() {
        let objective = objective_with_floor(RouteProtectionClass::LinkProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
        );
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable),
        );
    }

    #[test]
    fn admission_check_admits_matching_profile_and_summary() {
        let objective = objective_with_floor(RouteProtectionClass::LinkProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
        );
        assert_eq!(check.decision, AdmissionDecision::Admissible);
    }
}
