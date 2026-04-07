use jacquard_core::{
    AdaptiveRoutingProfile, AdmissionAssumptions, AdmissionDecision, BackendRouteId, Belief,
    Blake3Digest, Configuration, DestinationId, Estimate, Limit, NodeId, Observation,
    RouteAdmission, RouteAdmissionCheck, RouteAdmissionRejection, RouteCandidate,
    RouteConnectivityProfile, RouteCost, RouteError, RouteEstimate, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteSelectionError, RouteServiceKind, RouteSummary,
    RouteWitness, RoutingObjective, Tick, TimeWindow, ROUTE_HOP_COUNT_MAX,
};
use jacquard_traits::{
    CommitteeSelector, Hashing, MeshTopologyModel, MeshTransport, OrderEffects, RetentionStore,
    RouteEventLogEffects, RoutingEnginePlanner, StorageEffects, TimeEffects,
};

use super::{
    support::{
        confidence_for_segments, decode_backend_token, degradation_for_candidate,
        deterministic_order_key, encode_path_bytes, route_cost_for_segments, shortest_paths,
        unique_protocol_mix,
    },
    CachedCandidate, MeshEngine, MeshPath, MeshRouteClass, MeshRouteSegment,
    MESH_CANDIDATE_COUNT_MAX, MESH_CANDIDATE_VALIDITY_TICKS, MESH_CAPABILITIES, MESH_ENGINE_ID,
};
use crate::{
    committee::mesh_admission_assumptions,
    topology::{
        estimate_hop_link, objective_matches_node, route_capable_for_engine,
        MeshNeighborhoodEstimate, MeshPeerEstimate,
    },
};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
    Hasher: Hashing<Digest = Blake3Digest>,
    Selector: CommitteeSelector<TopologyView = Configuration>,
{
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

    fn derive_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
        destination_node_id: NodeId,
        node_path: &[NodeId],
    ) -> Option<(BackendRouteId, CachedCandidate)> {
        let configuration = &topology.value;
        let destination_node = configuration.nodes.get(&destination_node_id)?;
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
        if segments.is_empty() || segments.len() > usize::from(ROUTE_HOP_COUNT_MAX) {
            return None;
        }

        let hold_capable = destination_node.profile.services.iter().any(|service| {
            service.service_kind == RouteServiceKind::Hold
                && service.routing_engines.contains(&MESH_ENGINE_ID)
                && service.valid_for.contains(topology.observed_at_tick)
        });
        let route_class = self.determine_route_class(objective, segments.len(), hold_capable);
        let connectivity = self.route_connectivity_for_class(objective, &route_class);
        let valid_for = TimeWindow::new(
            topology.observed_at_tick,
            Tick(topology.observed_at_tick.0 + MESH_CANDIDATE_VALIDITY_TICKS),
        )
        .expect("mesh candidates always use a positive validity window");
        let protocol_mix = unique_protocol_mix(&segments);
        let path_bytes = encode_path_bytes(node_path, &segments);
        let backend_route_id = self.candidate_plan_token(node_path);
        let route_id = self.route_id_for_backend(&backend_route_id);
        let order_key = deterministic_order_key(route_id, &self.hashing, &path_bytes);
        let route_cost = route_cost_for_segments(&segments, &route_class);
        let degradation = degradation_for_candidate(configuration, &route_class);
        let estimate = Estimate {
            value: RouteEstimate {
                estimated_protection: RouteProtectionClass::LinkProtected,
                estimated_connectivity: connectivity,
                topology_epoch: configuration.epoch,
                degradation,
            },
            confidence_permille: confidence_for_segments(&segments, configuration),
            updated_at_tick: topology.observed_at_tick,
        };
        let summary = RouteSummary {
            engine: MESH_ENGINE_ID,
            protection: RouteProtectionClass::LinkProtected,
            connectivity,
            protocol_mix,
            hop_count_hint: Belief::Estimated(Estimate {
                value: u8::try_from(segments.len())
                    .expect("segment count is bounded by ROUTE_HOP_COUNT_MAX"),
                confidence_permille: jacquard_core::RatioPermille(1000),
                updated_at_tick: topology.observed_at_tick,
            }),
            valid_for,
        };
        let admission_assumptions = mesh_admission_assumptions(profile, configuration);
        let admission_check = mesh_admission_check(
            objective,
            profile,
            &summary,
            &route_cost,
            &admission_assumptions,
        );
        let witness = RouteWitness {
            objective_protection: objective.target_protection,
            delivered_protection: summary.protection,
            objective_connectivity: objective.target_connectivity,
            delivered_connectivity: summary.connectivity,
            admission_profile: admission_assumptions,
            topology_epoch: configuration.epoch,
            degradation: estimate.value.degradation,
        };
        let committee = self.selector.as_ref().and_then(|selector| {
            selector
                .select_committee(objective, profile, topology)
                .ok()
                .flatten()
        });
        let path = MeshPath {
            route_id,
            epoch: configuration.epoch,
            source: self.local_node_id,
            destination: objective.destination.clone(),
            segments,
            valid_for,
            route_class,
        };
        Some((
            backend_route_id.clone(),
            CachedCandidate {
                route_id,
                summary,
                estimate,
                admission_check,
                witness,
                path,
                committee,
                route_cost,
                ordering_key: order_key,
            },
        ))
    }

    fn derive_candidate_from_backend_ref(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
        backend_route_id: &BackendRouteId,
    ) -> Result<CachedCandidate, RouteError> {
        let node_path =
            decode_backend_token(backend_route_id).ok_or(RouteSelectionError::NoCandidate)?;
        let destination_node_id = *node_path.last().ok_or(RouteSelectionError::NoCandidate)?;
        let (derived_backend_ref, candidate) = self
            .derive_candidate(
                objective,
                profile,
                topology,
                destination_node_id,
                &node_path,
            )
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
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
    Hasher: Hashing<Digest = Blake3Digest>,
    Selector: CommitteeSelector<TopologyView = Configuration>,
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
        let configuration = &topology.value;
        let paths = shortest_paths(&self.local_node_id, configuration);
        let mut cached = Vec::new();
        for (destination_node_id, node_path) in paths {
            if destination_node_id == self.local_node_id {
                continue;
            }
            let Some(destination_node) = configuration.nodes.get(&destination_node_id) else {
                continue;
            };
            if !route_capable_for_engine(destination_node, &MESH_ENGINE_ID, configuration.epoch) {
                continue;
            }
            if !objective_matches_node(
                &destination_node_id,
                destination_node,
                objective,
                &MESH_ENGINE_ID,
                topology.observed_at_tick,
            ) {
                continue;
            }
            if let Some((backend_route_id, candidate)) = self.derive_candidate(
                objective,
                profile,
                topology,
                destination_node_id,
                &node_path,
            ) {
                cached.push((backend_route_id, candidate));
            }
        }

        cached.sort_by_key(|(_backend_route_id, candidate)| {
            (
                candidate.path.segments.len(),
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
