//! Engine-owned Scatter model helpers for model-lane validation.

use jacquard_core::{
    BackendRouteId, Configuration, NodeId, Observation, RouteError, RouteSelectionError,
    RoutingObjective, SelectedRoutingParameters, Tick, TransportError,
};
use jacquard_traits::{effect_handler, RoutingEnginePlanner, TimeEffects, TransportSenderEffects};

use crate::{support::decode_backend_token, ScatterEngine, ScatterEngineConfig};

struct NullTransport;

#[effect_handler]
impl TransportSenderEffects for NullTransport {
    fn send_transport(
        &mut self,
        _endpoint: &jacquard_core::LinkEndpoint,
        _payload: &[u8],
    ) -> Result<(), TransportError> {
        Ok(())
    }
}

struct FixedTime {
    now: Tick,
}

#[effect_handler]
impl TimeEffects for FixedTime {
    fn now_tick(&self) -> Tick {
        self.now
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScatterPlannerDecisionResult {
    pub candidate_count: usize,
    pub backend_route_id: BackendRouteId,
    pub admitted: bool,
}

fn run_planner_decision_fixture(
    local_node_id: NodeId,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    topology: &Observation<Configuration>,
) -> Result<ScatterPlannerDecisionResult, RouteError> {
    let engine = ScatterEngine::with_config(
        local_node_id,
        NullTransport,
        FixedTime {
            now: topology.observed_at_tick,
        },
        ScatterEngineConfig::default(),
    );
    let candidates = engine.candidate_routes(objective, profile, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let token = decode_backend_token(&candidate.backend_ref.backend_route_id)
        .ok_or(RouteSelectionError::NoCandidate)?;
    if token.destination != objective.destination {
        return Err(RouteSelectionError::NoCandidate.into());
    }
    let admission = engine.admit_route(objective, profile, candidate.clone(), topology)?;
    Ok(ScatterPlannerDecisionResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
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
        ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, Environment,
        FactSourceClass, Limit, NodeId, Observation, OriginAuthenticationClass, RatioPermille,
        RouteEpoch, RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
        RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick, TransportKind,
    };
    use jacquard_host_support::opaque_endpoint;
    use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};

    use super::run_planner_decision_fixture;
    use crate::SCATTER_ENGINE_ID;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
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
            deployment_profile: jacquard_core::OperatingMode::DenseInteractive,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    // long-block-exception: the test topology fixture keeps one complete
    // deterministic scatter planning sample in one place for model fixtures.
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
                                endpoint(1),
                                Tick(4),
                            ),
                            &SCATTER_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(2),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(2), ControllerId([2; 32])),
                                endpoint(2),
                                Tick(4),
                            ),
                            &SCATTER_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(3),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(3), ControllerId([3; 32])),
                                endpoint(3),
                                Tick(4),
                            ),
                            &SCATTER_ENGINE_ID,
                        )
                        .build(),
                    ),
                ]),
                links: BTreeMap::from([
                    (
                        (node(1), node(2)),
                        LinkPreset::lossy(
                            LinkPresetOptions::new(endpoint(2), Tick(4))
                                .with_confidence(RatioPermille(950)),
                        )
                        .build(),
                    ),
                    (
                        (node(2), node(1)),
                        LinkPreset::lossy(
                            LinkPresetOptions::new(endpoint(1), Tick(4))
                                .with_confidence(RatioPermille(950)),
                        )
                        .build(),
                    ),
                    (
                        (node(2), node(3)),
                        LinkPreset::lossy(
                            LinkPresetOptions::new(endpoint(3), Tick(4))
                                .with_confidence(RatioPermille(950)),
                        )
                        .build(),
                    ),
                    (
                        (node(3), node(2)),
                        LinkPreset::lossy(
                            LinkPresetOptions::new(endpoint(2), Tick(4))
                                .with_confidence(RatioPermille(950)),
                        )
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
    fn planner_decision_fixture_produces_candidate() {
        let result = run_planner_decision_fixture(node(1), &objective(), &profile(), &topology())
            .expect("scatter planner fixture should produce a candidate");
        assert!(result.admitted);
        assert_eq!(result.candidate_count, 1);
    }
}
