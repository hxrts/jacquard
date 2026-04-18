//! `RoutingEnginePlanner` impl for `BatmanClassicEngine`.
//!
//! Candidate enumeration reads the best next-hop table; admission validates that
//! the candidate's `BackendRouteRef` still matches the current best next-hop.
//! Destination service support is verified against `BATMAN_CLASSIC_ENGINE_ID`
//! in the node profile before any table lookup.

use jacquard_core::{
    AdmissionDecision, Configuration, DestinationId, Observation, RouteAdmission,
    RouteAdmissionCheck, RouteAdmissionRejection, RouteCandidate, RouteError, RouteSelectionError,
    RoutingEngineCapabilities, RoutingEngineId, SelectedRoutingParameters,
};
use jacquard_traits::{
    RoutingEnginePlanner, RoutingEnginePlannerModel, TimeEffects, TransportSenderEffects,
};

use crate::{
    private_state::{admission_for_candidate, candidate_for_snapshot},
    BatmanClassicEngine, BatmanClassicPlannerSnapshot, BATMAN_CLASSIC_CAPABILITIES,
    BATMAN_CLASSIC_ENGINE_ID,
};

impl<Transport, Effects> RoutingEnginePlanner for BatmanClassicEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn engine_id(&self) -> RoutingEngineId {
        BATMAN_CLASSIC_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        BATMAN_CLASSIC_CAPABILITIES
    }

    fn candidate_routes(
        &self,
        objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        let DestinationId::Node(destination) = objective.destination else {
            return Vec::new();
        };
        if !destination_supports_objective(topology, destination, objective.service_kind) {
            return Vec::new();
        }
        candidate_routes_from_snapshot(&self.planner_snapshot(), objective, topology)
    }

    fn check_candidate(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        let admission =
            self.current_candidate_admission(objective, profile, candidate, topology)?;
        if let AdmissionDecision::Rejected(reason) = admission.admission_check.decision {
            return Err(RouteSelectionError::Inadmissible(reason).into());
        }
        Ok(admission.admission_check)
    }

    fn admit_route(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        let admission =
            self.current_candidate_admission(objective, profile, &candidate, topology)?;
        if let AdmissionDecision::Rejected(reason) = admission.admission_check.decision {
            return Err(RouteSelectionError::Inadmissible(reason).into());
        }
        Ok(admission)
    }
}

impl<Transport, Effects> BatmanClassicEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn current_candidate_admission(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        let DestinationId::Node(destination) = objective.destination else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        if !destination_supports_objective(topology, destination, objective.service_kind) {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        current_candidate_admission_from_snapshot(
            &self.planner_snapshot(),
            objective,
            profile,
            candidate,
            topology,
        )
    }
}

impl<Transport, Effects> RoutingEnginePlannerModel for BatmanClassicEngine<Transport, Effects> {
    type PlannerSnapshot = BatmanClassicPlannerSnapshot;
    type PlannerCandidate = RouteCandidate;
    type PlannerAdmission = RouteAdmission;

    fn candidate_routes_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<Self::PlannerCandidate> {
        candidate_routes_from_snapshot(snapshot, objective, topology)
    }

    fn admit_route_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &Self::PlannerCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<Self::PlannerAdmission, RouteError> {
        admit_route_from_snapshot(snapshot, objective, profile, candidate, topology)
    }
}

#[must_use = "candidate projection from a planner snapshot must be consumed by simulator or planner checks"]
pub fn candidate_routes_from_snapshot(
    snapshot: &BatmanClassicPlannerSnapshot,
    objective: &jacquard_core::RoutingObjective,
    topology: &Observation<Configuration>,
) -> Vec<RouteCandidate> {
    let DestinationId::Node(destination) = objective.destination else {
        return Vec::new();
    };
    if !destination_supports_objective(topology, destination, objective.service_kind) {
        return Vec::new();
    }
    snapshot
        .best_next_hops
        .get(&destination)
        .map(|best| vec![candidate_for_snapshot(snapshot, objective, best)])
        .unwrap_or_default()
}

pub fn admit_route_from_snapshot(
    snapshot: &BatmanClassicPlannerSnapshot,
    objective: &jacquard_core::RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: &RouteCandidate,
    topology: &Observation<Configuration>,
) -> Result<RouteAdmission, RouteError> {
    current_candidate_admission_from_snapshot(snapshot, objective, profile, candidate, topology)
}

fn current_candidate_admission_from_snapshot(
    snapshot: &BatmanClassicPlannerSnapshot,
    objective: &jacquard_core::RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: &RouteCandidate,
    topology: &Observation<Configuration>,
) -> Result<RouteAdmission, RouteError> {
    let DestinationId::Node(destination) = objective.destination else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    if !destination_supports_objective(topology, destination, objective.service_kind) {
        return Err(
            RouteSelectionError::Inadmissible(RouteAdmissionRejection::BackendUnavailable).into(),
        );
    }
    let Some(best) = snapshot.best_next_hops.get(&destination) else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    let expected = candidate_for_snapshot(snapshot, objective, best);
    if expected.backend_ref != candidate.backend_ref {
        return Err(
            RouteSelectionError::Inadmissible(RouteAdmissionRejection::BackendUnavailable).into(),
        );
    }
    Ok(admission_for_candidate(objective, profile, &expected))
}

fn destination_supports_objective(
    topology: &Observation<Configuration>,
    destination: jacquard_core::NodeId,
    service_kind: jacquard_core::RouteServiceKind,
) -> bool {
    topology
        .value
        .nodes
        .get(&destination)
        .map(|node| {
            node.profile.services.iter().any(|service| {
                service.service_kind == service_kind
                    && service.routing_engines.contains(&BATMAN_CLASSIC_ENGINE_ID)
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_adapter::opaque_endpoint;
    use jacquard_core::{
        ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
        Environment, FactSourceClass, LinkEndpoint, NodeId, Observation, OriginAuthenticationClass,
        RatioPermille, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
        RouteServiceKind, RoutingEngineId, RoutingEvidenceClass, RoutingObjective,
        RoutingTickContext, SelectedRoutingParameters, Tick, TransportKind,
    };
    use jacquard_mem_link_profile::{
        InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
    };
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::{RoutingEngine, RoutingEnginePlanner};

    use super::*;
    use crate::BatmanClassicEngine;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
    }

    fn sample_objective(destination: NodeId) -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(destination),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
            latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(100)),
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
                epoch: jacquard_core::RouteEpoch(2),
                nodes: BTreeMap::from([
                    (
                        node(1),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(1), ControllerId([1; 32])),
                                endpoint(1),
                                Tick(1),
                            ),
                            &BATMAN_CLASSIC_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(2),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(2), ControllerId([2; 32])),
                                endpoint(2),
                                Tick(1),
                            ),
                            &BATMAN_CLASSIC_ENGINE_ID,
                        )
                        .build(),
                    ),
                ]),
                links: BTreeMap::from([(
                    (node(1), node(2)),
                    LinkPreset::active(LinkPresetOptions::new(endpoint(2), Tick(1))).build(),
                )]),
                environment: Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    #[test]
    fn candidate_routes_require_classic_engine_id_in_destination_service() {
        let topology = supported_topology();
        let foreign_engine = RoutingEngineId::from_contract_bytes(*b"foreign-path-sup");
        let mut unsupported = topology.clone();
        unsupported.value.nodes.insert(
            node(2),
            NodePreset::route_capable(
                NodePresetOptions::new(
                    NodeIdentity::new(node(2), ControllerId([2; 32])),
                    endpoint(2),
                    Tick(1),
                ),
                &foreign_engine,
            )
            .build(),
        );
        let mut engine = BatmanClassicEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        // Inject OGM state for node(2) so the engine would otherwise produce a candidate.
        engine.observe_originator_ogm(node(2), node(2), 1, RatioPermille(1000), 1, Tick(1));
        engine.observe_bidirectional_ogm(node(2), 1, Tick(1));
        engine
            .engine_tick(&RoutingTickContext::new(unsupported.clone()))
            .expect("populate table");

        let candidates =
            engine.candidate_routes(&sample_objective(node(2)), &sample_profile(), &unsupported);

        assert!(candidates.is_empty());
    }
}
