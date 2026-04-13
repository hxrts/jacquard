use std::collections::BTreeSet;

use jacquard_core::{DestinationId, NodeId, RouteLifecycleEvent, RoutingEngineId};
use jacquard_traits::RoutingScenario;

use crate::{
    environment::AppliedEnvironmentHook,
    replay::{ActiveRouteSummary, DriverStatusEvent, SimulationFailureSummary},
    JacquardReplayArtifact,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReducedReplayView {
    pub scenario_name: String,
    pub round_count: u32,
    pub rounds: Vec<ReducedReplayRound>,
    pub distinct_engine_ids: Vec<RoutingEngineId>,
    pub driver_status_events: Vec<DriverStatusEvent>,
    pub failure_summaries: Vec<SimulationFailureSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReducedReplayRound {
    pub round_index: u32,
    pub active_routes: Vec<ActiveRouteSummary>,
    pub environment_hooks: Vec<AppliedEnvironmentHook>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReducedRouteKey {
    pub owner_node_id: NodeId,
    pub destination: DestinationId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReducedRouteObservation {
    pub key: ReducedRouteKey,
    pub route_id: jacquard_core::RouteId,
    pub engine_id: RoutingEngineId,
    pub first_seen_round: u32,
    pub last_seen_round: u32,
    pub last_lifecycle_event: RouteLifecycleEvent,
}

impl ReducedReplayView {
    #[must_use]
    pub fn from_replay(replay: &JacquardReplayArtifact) -> Self {
        let rounds = replay
            .rounds
            .iter()
            .map(|round| ReducedReplayRound {
                round_index: round.round_index,
                active_routes: round
                    .host_rounds
                    .iter()
                    .flat_map(|host| host.active_routes.iter().cloned())
                    .collect(),
                environment_hooks: round.environment_artifacts.clone(),
            })
            .collect::<Vec<_>>();
        let distinct_engine_ids = rounds
            .iter()
            .flat_map(|round| {
                round
                    .active_routes
                    .iter()
                    .map(|route| route.engine_id.clone())
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            scenario_name: replay.scenario.name().to_owned(),
            round_count: u32::try_from(rounds.len()).unwrap_or(u32::MAX),
            rounds,
            distinct_engine_ids,
            driver_status_events: replay.driver_status_events.clone(),
            failure_summaries: replay.failure_summaries.clone(),
        }
    }

    #[must_use]
    pub fn route_observations(&self) -> Vec<ReducedRouteObservation> {
        let mut observations = Vec::new();
        for round in &self.rounds {
            for route in &round.active_routes {
                let key = ReducedRouteKey {
                    owner_node_id: route.owner_node_id,
                    destination: route.destination.clone(),
                };
                if let Some(existing) =
                    observations
                        .iter_mut()
                        .find(|entry: &&mut ReducedRouteObservation| {
                            entry.key == key
                                && entry.route_id == route.route_id
                                && entry.engine_id == route.engine_id
                        })
                {
                    existing.last_seen_round = round.round_index;
                    existing.last_lifecycle_event = route.last_lifecycle_event;
                } else {
                    observations.push(ReducedRouteObservation {
                        key,
                        route_id: route.route_id,
                        engine_id: route.engine_id.clone(),
                        first_seen_round: round.round_index,
                        last_seen_round: round.round_index,
                        last_lifecycle_event: route.last_lifecycle_event,
                    });
                }
            }
        }
        observations
    }

    #[must_use]
    pub fn route_seen(&self, owner_node_id: NodeId, destination: &DestinationId) -> bool {
        self.rounds.iter().any(|round| {
            round.active_routes.iter().any(|route| {
                route.owner_node_id == owner_node_id && &route.destination == destination
            })
        })
    }

    #[must_use]
    pub fn route_present_rounds(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Vec<u32> {
        self.rounds
            .iter()
            .filter(|round| {
                round.active_routes.iter().any(|route| {
                    route.owner_node_id == owner_node_id && &route.destination == destination
                })
            })
            .map(|round| round.round_index)
            .collect()
    }

    #[must_use]
    pub fn route_stability_scores(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Vec<u32> {
        self.rounds
            .iter()
            .flat_map(|round| {
                round.active_routes.iter().filter_map(|route| {
                    (route.owner_node_id == owner_node_id && &route.destination == destination)
                        .then_some(route.stability_score.0)
                })
            })
            .collect()
    }

    #[must_use]
    pub fn first_round_with_route(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<u32> {
        self.route_present_rounds(owner_node_id, destination)
            .into_iter()
            .next()
    }

    #[must_use]
    pub fn last_round_with_route(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<u32> {
        self.route_present_rounds(owner_node_id, destination)
            .into_iter()
            .last()
    }

    #[must_use]
    pub fn route_seen_with_engine(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
        engine_id: &RoutingEngineId,
    ) -> bool {
        self.rounds.iter().any(|round| {
            round.active_routes.iter().any(|route| {
                route.owner_node_id == owner_node_id
                    && &route.destination == destination
                    && &route.engine_id == engine_id
            })
        })
    }

    #[must_use]
    pub fn first_round_with_engine(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
        engine_id: &RoutingEngineId,
    ) -> Option<u32> {
        self.rounds
            .iter()
            .find(|round| {
                round.active_routes.iter().any(|route| {
                    route.owner_node_id == owner_node_id
                        && &route.destination == destination
                        && &route.engine_id == engine_id
                })
            })
            .map(|round| round.round_index)
    }

    #[must_use]
    pub fn recovery_delta_rounds(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<u32> {
        let mut seen_active = false;
        let mut first_absent_round = None;
        for round in &self.rounds {
            let active = round.active_routes.iter().any(|route| {
                route.owner_node_id == owner_node_id && &route.destination == destination
            });
            if active {
                if let Some(absent_round) = first_absent_round {
                    return Some(round.round_index.saturating_sub(absent_round));
                }
                seen_active = true;
            } else if seen_active && first_absent_round.is_none() {
                first_absent_round = Some(round.round_index);
            }
        }
        None
    }

    #[must_use]
    pub fn first_round_without_route_after_presence(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<u32> {
        let mut seen_active = false;
        for round in &self.rounds {
            let active = round.active_routes.iter().any(|route| {
                route.owner_node_id == owner_node_id && &route.destination == destination
            });
            if active {
                seen_active = true;
                continue;
            }
            if seen_active {
                return Some(round.round_index);
            }
        }
        None
    }

    #[must_use]
    pub fn recovered_within_rounds(&self, rounds: u32) -> bool {
        let observations = self.route_observations();
        for observation in &observations {
            let key = &observation.key;
            let mut seen_active = false;
            let mut first_absent_round = None;
            for round in &self.rounds {
                let active = round.active_routes.iter().any(|route| {
                    route.owner_node_id == key.owner_node_id && route.destination == key.destination
                });
                if active {
                    if let Some(absent_round) = first_absent_round {
                        if round.round_index.saturating_sub(absent_round) <= rounds {
                            return true;
                        }
                    }
                    seen_active = true;
                } else if seen_active && first_absent_round.is_none() {
                    first_absent_round = Some(round.round_index);
                }
            }
        }
        false
    }

    #[must_use]
    pub fn route_absent_after_round(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
        round_index: u32,
    ) -> bool {
        self.rounds
            .iter()
            .filter(|round| round.round_index > round_index)
            .all(|round| {
                round.active_routes.iter().all(|route| {
                    route.owner_node_id != owner_node_id || &route.destination != destination
                })
            })
    }
}
