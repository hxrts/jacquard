//! Engine-owned Field model helpers for model-lane validation.

use jacquard_core::{
    AdmissionDecision, BackendRouteId, Configuration, DestinationId, NodeId, Observation,
    RouteError, RouteSelectionError, RoutingObjective, SelectedRoutingParameters, Tick,
};

use crate::{
    route::decode_backend_token,
    state::{DestinationInterestClass, HopBand, NeighborContinuation, SupportBucket},
    FieldEngine,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldPlannerDecisionResult {
    pub candidate_count: usize,
    pub backend_route_id: BackendRouteId,
    pub selected_neighbor: NodeId,
    pub admitted: bool,
}

pub(crate) fn validate_planner_decision(
    local_node_id: NodeId,
    expected_next_hop: NodeId,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    topology: &Observation<Configuration>,
) -> Result<FieldPlannerDecisionResult, RouteError> {
    let destination = match objective.destination {
        DestinationId::Node(destination) => destination,
        DestinationId::Gateway(_) | DestinationId::Service(_) => {
            return Err(RouteSelectionError::NoCandidate.into());
        }
    };
    let engine = seeded_planner_engine(
        local_node_id,
        objective,
        destination,
        expected_next_hop,
        topology.observed_at_tick,
    );
    let snapshot = engine.planner_snapshot();
    let artifacts = engine.planning_artifacts(&snapshot, objective, profile, topology)?;
    let token = decode_backend_token(&artifacts.candidate.backend_ref.backend_route_id)
        .ok_or(RouteSelectionError::NoCandidate)?;
    Ok(FieldPlannerDecisionResult {
        candidate_count: 1,
        backend_route_id: artifacts.candidate.backend_ref.backend_route_id,
        selected_neighbor: token.selected_neighbor,
        admitted: matches!(
            artifacts.admission_check.decision,
            AdmissionDecision::Admissible
        ),
    })
}

fn seeded_planner_engine(
    local_node_id: NodeId,
    objective: &RoutingObjective,
    destination: NodeId,
    selected_neighbor: NodeId,
    now_tick: Tick,
) -> FieldEngine<(), ()> {
    let mut engine = FieldEngine::new(local_node_id, (), ());
    engine.state.note_tick(now_tick);
    let state = engine.state.upsert_destination_interest(
        &objective.destination,
        DestinationInterestClass::Transit,
        now_tick,
    );
    state.posterior.top_corridor_mass = SupportBucket::new(860);
    state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
    state.corridor_belief.delivery_support = SupportBucket::new(780);
    state.corridor_belief.retention_affinity = SupportBucket::new(640);
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: selected_neighbor,
        net_value: SupportBucket::new(920),
        downstream_support: SupportBucket::new(840),
        expected_hop_band: HopBand::new(1, 2),
        freshness: now_tick,
    });
    if destination == selected_neighbor {
        state.corridor_belief.expected_hop_band = HopBand::new(1, 1);
    }
    engine
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, Environment,
        FactSourceClass, Limit, NodeId, Observation, OriginAuthenticationClass, RatioPermille,
        RouteEpoch, RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
        RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick,
    };
    use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};

    use super::validate_planner_decision;
    use crate::FIELD_ENGINE_ID;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    // long-block-exception: the simulator topology fixture keeps one complete
    // deterministic field planning sample in one place for model-lane tests.
    fn topology() -> Observation<Configuration> {
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
                                Tick(4),
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
                                Tick(4),
                            ),
                            &FIELD_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(3),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(3), ControllerId([3; 32])),
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![3],
                                    ByteCount(128),
                                ),
                                Tick(4),
                            ),
                            &FIELD_ENGINE_ID,
                        )
                        .build(),
                    ),
                ]),
                links: BTreeMap::from([
                    (
                        (node(1), node(2)),
                        LinkPreset::lossy(
                            LinkPresetOptions::new(
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![2],
                                    ByteCount(128),
                                ),
                                Tick(4),
                            )
                            .with_confidence(RatioPermille(950)),
                        )
                        .build(),
                    ),
                    (
                        (node(2), node(1)),
                        LinkPreset::lossy(
                            LinkPresetOptions::new(
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![1],
                                    ByteCount(128),
                                ),
                                Tick(4),
                            )
                            .with_confidence(RatioPermille(950)),
                        )
                        .build(),
                    ),
                    (
                        (node(2), node(3)),
                        LinkPreset::lossy(
                            LinkPresetOptions::new(
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![3],
                                    ByteCount(128),
                                ),
                                Tick(4),
                            )
                            .with_confidence(RatioPermille(950)),
                        )
                        .build(),
                    ),
                    (
                        (node(3), node(2)),
                        LinkPreset::lossy(
                            LinkPresetOptions::new(
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![2],
                                    ByteCount(128),
                                ),
                                Tick(4),
                            )
                            .with_confidence(RatioPermille(950)),
                        )
                        .build(),
                    ),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 2,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(4),
        }
    }

    fn objective() -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(node(3)),
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

    fn profile() -> SelectedRoutingParameters {
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

    #[test]
    fn planner_decision_fixture_selects_seeded_neighbor() {
        let result =
            validate_planner_decision(node(1), node(2), &objective(), &profile(), &topology())
                .expect("planner validation should produce a candidate");
        assert_eq!(result.candidate_count, 1);
        assert_eq!(result.selected_neighbor, node(2));
        assert!(result.admitted);
    }
}
