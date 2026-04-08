//! `RoutingEnginePlanner` impl for `BatmanEngine`. Candidate enumeration
//! reads the best next-hop table keyed by destination node, admission
//! checks the objective against table entries, and emits one candidate per
//! known destination without searching the shared topology.

use jacquard_core::{
    Configuration, DestinationId, Observation, RouteAdmission, RouteAdmissionCheck,
    RouteAdmissionRejection, RouteCandidate, RouteError, RouteSelectionError,
    RoutingEngineCapabilities, RoutingEngineId, SelectedRoutingParameters,
};
use jacquard_traits::{RoutingEnginePlanner, TimeEffects, TransportEffects};

use crate::{BatmanEngine, BATMAN_CAPABILITIES, BATMAN_ENGINE_ID};

impl<Transport, Effects> RoutingEnginePlanner for BatmanEngine<Transport, Effects>
where
    Transport: TransportEffects,
    Effects: TimeEffects,
{
    fn engine_id(&self) -> RoutingEngineId {
        BATMAN_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        BATMAN_CAPABILITIES
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
        if !destination_supports_objective(
            topology,
            destination,
            objective.service_kind,
        ) {
            return Vec::new();
        }
        self.best_next_hops
            .get(&destination)
            .map(|best| vec![self.candidate_for(objective, best)])
            .unwrap_or_default()
    }

    fn check_candidate(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        self.admit_route(objective, profile, candidate.clone(), topology)
            .map(|admission| admission.admission_check)
    }

    fn admit_route(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        let DestinationId::Node(destination) = objective.destination else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        if !destination_supports_objective(
            topology,
            destination,
            objective.service_kind,
        ) {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        let Some(best) = self.best_next_hops.get(&destination) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let expected = self.candidate_for(objective, best);
        if expected.backend_ref != candidate.backend_ref {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        Ok(self.admission_for(objective, profile, &expected))
    }
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
                    && service.routing_engines.contains(&BATMAN_ENGINE_ID)
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        Configuration, ConnectivityPosture, DestinationId, DurationMs, Environment,
        FactSourceClass, NodeId, Observation, OriginAuthenticationClass, RatioPermille,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
        RoutingEngineId, RoutingEvidenceClass, RoutingObjective, RoutingTickContext,
        SelectedRoutingParameters, Tick,
    };
    use jacquard_mem_link_profile::{
        InMemoryRuntimeEffects, InMemoryTransport, ReferenceLink,
    };
    use jacquard_mem_node_profile::ReferenceNode;
    use jacquard_traits::{RoutingEngine, RoutingEnginePlanner};

    use super::*;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
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
            routing_engine_fallback_policy:
                jacquard_core::RoutingEngineFallbackPolicy::Allowed,
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
                        ReferenceNode::ble_route_capable(1, &BATMAN_ENGINE_ID, Tick(1))
                            .build(),
                    ),
                    (
                        node(2),
                        ReferenceNode::ble_route_capable(2, &BATMAN_ENGINE_ID, Tick(1))
                            .build(),
                    ),
                ]),
                links: BTreeMap::from([(
                    (node(1), node(2)),
                    ReferenceLink::ble_active(2, Tick(1)).build(),
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
    fn candidate_routes_require_destination_service_support_for_batman() {
        let topology = supported_topology();
        let foreign_engine = RoutingEngineId::from_contract_bytes(*b"foreign-mesh-sup");
        let mut unsupported = topology.clone();
        unsupported.value.nodes.insert(
            node(2),
            ReferenceNode::ble_route_capable(2, &foreign_engine, Tick(1)).build(),
        );
        let mut engine = BatmanEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects { now: Tick(1), ..Default::default() },
        );
        engine
            .engine_tick(&RoutingTickContext::new(unsupported.clone()))
            .expect("populate table");

        let candidates = engine.candidate_routes(
            &sample_objective(node(2)),
            &sample_profile(),
            &unsupported,
        );

        assert!(candidates.is_empty());
    }
}
