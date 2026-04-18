//! Engine-owned Pathway model helpers for model-lane validation.

use jacquard_core::{
    BackendRouteId, Configuration, NodeId, Observation, RouteError, RouteSelectionError,
    RoutingObjective, SelectedRoutingParameters,
};
use jacquard_traits::{Blake3Hashing, RoutingEnginePlanner};

use crate::{
    engine::first_hop_node_id_from_backend_route_id, DeterministicPathwayTopologyModel,
    PathwayEngine,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct PathwayPlannerDecisionResult {
    pub candidate_count: usize,
    pub backend_route_id: BackendRouteId,
    pub first_hop_node_id: NodeId,
    pub admitted: bool,
}

fn run_planner_decision_fixture(
    local_node_id: NodeId,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    topology: &Observation<Configuration>,
) -> Result<PathwayPlannerDecisionResult, RouteError> {
    let engine = PathwayEngine::without_committee_selector(
        local_node_id,
        DeterministicPathwayTopologyModel::new(),
        (),
        (),
        (),
        Blake3Hashing,
    );
    let candidates = engine.candidate_routes(objective, profile, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission = engine.admit_route(objective, profile, candidate.clone(), topology)?;
    let first_hop_node_id =
        first_hop_node_id_from_backend_route_id(&candidate.backend_ref.backend_route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
    Ok(PathwayPlannerDecisionResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
        first_hop_node_id,
        admitted: matches!(
            admission.admission_check.decision,
            jacquard_core::AdmissionDecision::Admissible
        ),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        Configuration, ConnectivityPosture, DestinationId, Environment, FactSourceClass, Limit,
        NodeId, Observation, OriginAuthenticationClass, RatioPermille, RouteEpoch,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
        RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick,
    };

    use super::run_planner_decision_fixture;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn objective() -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(node(3)),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
            latency_budget_ms: Limit::Bounded(jacquard_core::DurationMs(250)),
            protection_priority: jacquard_core::PriorityPoints(10),
            connectivity_priority: jacquard_core::PriorityPoints(20),
        }
    }

    fn profile() -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(4),
                nodes: BTreeMap::from([
                    (
                        node(1),
                        jacquard_testkit::topology::node(1).pathway().build(),
                    ),
                    (
                        node(2),
                        jacquard_testkit::topology::node(2).pathway().build(),
                    ),
                    (
                        node(3),
                        jacquard_testkit::topology::node(3).pathway().build(),
                    ),
                ]),
                links: BTreeMap::from([
                    (
                        (node(1), node(2)),
                        jacquard_testkit::topology::link(2)
                            .observed_at(Tick(4))
                            .build(),
                    ),
                    (
                        (node(2), node(1)),
                        jacquard_testkit::topology::link(1)
                            .observed_at(Tick(4))
                            .build(),
                    ),
                    (
                        (node(2), node(3)),
                        jacquard_testkit::topology::link(3)
                            .observed_at(Tick(4))
                            .build(),
                    ),
                    (
                        (node(3), node(2)),
                        jacquard_testkit::topology::link(2)
                            .observed_at(Tick(4))
                            .build(),
                    ),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::AdmissionWitnessed,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(4),
        }
    }

    #[test]
    fn planner_decision_fixture_selects_line_first_hop() {
        let result = run_planner_decision_fixture(node(1), &objective(), &profile(), &topology())
            .expect("pathway planner fixture should produce a candidate");
        assert!(result.admitted);
        assert_eq!(result.first_hop_node_id, node(2));
        assert_eq!(result.candidate_count, 1);
    }
}
