//! Condensed replay view: per-route presence, materialization, and loss summaries.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    DestinationId, NodeId, ReachabilityState, RouteLifecycleEvent, RoutingEngineId,
};
use jacquard_field::FIELD_ENGINE_ID;
use jacquard_traits::RoutingScenario;
use serde::{Deserialize, Serialize};

use crate::{
    environment::{AppliedEnvironmentHook, EnvironmentHook},
    replay::{ActiveRouteSummary, DriverStatusEvent, FieldReplaySummary, SimulationFailureSummary},
    JacquardReplayArtifact, JacquardScenario,
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
    pub field_replays: Vec<ReducedFieldReplayObservation>,
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
    pub next_hop_node_id: Option<NodeId>,
    pub first_seen_round: u32,
    pub last_seen_round: u32,
    pub last_lifecycle_event: RouteLifecycleEvent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReducedFieldReplayObservation {
    pub local_node_id: NodeId,
    pub summary: FieldReplaySummary,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct ReducedFailureClassCounts {
    pub no_candidate: u32,
    pub inadmissible_candidate: u32,
    pub lost_reachability: u32,
    pub replacement_loop: u32,
    pub maintenance_failure: u32,
    pub activation_failure: u32,
    pub persistent_degraded: u32,
    pub other: u32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct ReducedEnvironmentHookCounts {
    pub replace_topology: u32,
    pub medium_degradation: u32,
    pub asymmetric_degradation: u32,
    pub partition: u32,
    pub cascade_partition: u32,
    pub mobility_relink: u32,
    pub intrinsic_limit: u32,
}

pub(crate) type ObjectiveRouteFilter = BTreeMap<NodeId, Vec<DestinationId>>;

#[must_use]
pub(crate) fn objective_route_filter_for(scenario: &JacquardScenario) -> ObjectiveRouteFilter {
    let mut filter = ObjectiveRouteFilter::new();
    for binding in scenario.bound_objectives() {
        filter
            .entry(binding.owner_node_id)
            .or_default()
            .push(binding.objective.destination.clone());
    }
    filter
}

#[must_use]
pub(crate) fn objective_owner_nodes_for(scenario: &JacquardScenario) -> BTreeSet<NodeId> {
    scenario
        .bound_objectives()
        .iter()
        .map(|binding| binding.owner_node_id)
        .collect()
}

#[must_use]
pub(crate) fn filter_active_routes(
    active_routes: impl IntoIterator<Item = ActiveRouteSummary>,
    objective_routes: &ObjectiveRouteFilter,
) -> Vec<ActiveRouteSummary> {
    if objective_routes.is_empty() {
        return active_routes.into_iter().collect();
    }
    active_routes
        .into_iter()
        .filter(|route| {
            objective_routes
                .get(&route.owner_node_id)
                .is_some_and(|destinations| {
                    destinations
                        .iter()
                        .any(|destination| destination == &route.destination)
                })
        })
        .collect()
}

#[must_use]
pub(crate) fn filter_field_replays(
    field_replays: impl IntoIterator<Item = ReducedFieldReplayObservation>,
    objective_owner_nodes: &BTreeSet<NodeId>,
) -> Vec<ReducedFieldReplayObservation> {
    if objective_owner_nodes.is_empty() {
        return field_replays.into_iter().collect();
    }
    field_replays
        .into_iter()
        .filter(|observation| objective_owner_nodes.contains(&observation.local_node_id))
        .collect()
}

#[must_use]
pub(crate) fn distinct_engine_ids_for(rounds: &[ReducedReplayRound]) -> Vec<RoutingEngineId> {
    rounds
        .iter()
        .flat_map(|round| {
            round
                .active_routes
                .iter()
                .map(|route| route.engine_id.clone())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

impl ReducedReplayView {
    #[must_use]
    pub fn from_replay(replay: &JacquardReplayArtifact) -> Self {
        let objective_routes = objective_route_filter_for(&replay.scenario);
        let objective_owner_nodes = objective_owner_nodes_for(&replay.scenario);
        let rounds = replay
            .rounds
            .iter()
            .map(|round| ReducedReplayRound {
                round_index: round.round_index,
                active_routes: filter_active_routes(
                    round
                        .host_rounds
                        .iter()
                        .flat_map(|host| host.active_routes.iter().cloned()),
                    &objective_routes,
                ),
                environment_hooks: round.environment_artifacts.clone(),
                field_replays: filter_field_replays(
                    round.host_rounds.iter().filter_map(|host| {
                        host.field_replay
                            .clone()
                            .map(|summary| ReducedFieldReplayObservation {
                                local_node_id: host.local_node_id,
                                summary,
                            })
                    }),
                    &objective_owner_nodes,
                ),
            })
            .collect::<Vec<_>>();
        let distinct_engine_ids = distinct_engine_ids_for(&rounds);
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
                                && entry.next_hop_node_id == route.next_hop_node_id
                                && entry.last_seen_round.saturating_add(1) == round.round_index
                        })
                {
                    existing.last_seen_round = round.round_index;
                    existing.last_lifecycle_event = route.last_lifecycle_event;
                } else {
                    observations.push(ReducedRouteObservation {
                        key,
                        route_id: route.route_id,
                        engine_id: route.engine_id.clone(),
                        next_hop_node_id: route.next_hop_node_id,
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
    pub fn route_usable_rounds(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Vec<u32> {
        self.rounds
            .iter()
            .filter(|round| {
                round.active_routes.iter().any(|route| {
                    route.owner_node_id == owner_node_id
                        && &route.destination == destination
                        && route.reachability_state != ReachabilityState::Unreachable
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
    pub fn route_hop_counts(&self, owner_node_id: NodeId, destination: &DestinationId) -> Vec<u32> {
        self.rounds
            .iter()
            .flat_map(|round| {
                round.active_routes.iter().filter_map(|route| {
                    (route.owner_node_id == owner_node_id && &route.destination == destination)
                        .then_some(route.hop_count_hint.map(u32::from))
                        .flatten()
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
    pub fn usable_recovery_delta_rounds(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<u32> {
        let mut seen_usable = false;
        let mut first_unusable_round = None;
        for round in &self.rounds {
            let usable = round.active_routes.iter().any(|route| {
                route.owner_node_id == owner_node_id
                    && &route.destination == destination
                    && route.reachability_state != ReachabilityState::Unreachable
            });
            if usable {
                if let Some(unusable_round) = first_unusable_round {
                    return Some(round.round_index.saturating_sub(unusable_round));
                }
                seen_usable = true;
            } else if seen_usable && first_unusable_round.is_none() {
                first_unusable_round = Some(round.round_index);
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
    pub fn first_round_without_usable_route_after_presence(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<u32> {
        let mut seen_usable = false;
        for round in &self.rounds {
            let usable = round.active_routes.iter().any(|route| {
                route.owner_node_id == owner_node_id
                    && &route.destination == destination
                    && route.reachability_state != ReachabilityState::Unreachable
            });
            if usable {
                seen_usable = true;
                continue;
            }
            if seen_usable {
                return Some(round.round_index);
            }
        }
        None
    }

    #[must_use]
    pub fn first_round_with_environment_change_at_or_after(&self, round_index: u32) -> Option<u32> {
        self.rounds
            .iter()
            .find(|round| round.round_index >= round_index && !round.environment_hooks.is_empty())
            .map(|round| round.round_index)
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

    #[must_use]
    pub fn route_churn_count(&self, owner_node_id: NodeId, destination: &DestinationId) -> u32 {
        let observations = self
            .route_observations()
            .into_iter()
            .filter(|observation| {
                observation.key.owner_node_id == owner_node_id
                    && &observation.key.destination == destination
            })
            .count();
        u32::try_from(observations.saturating_sub(1)).unwrap_or(u32::MAX)
    }

    #[must_use]
    pub fn engine_handoff_count(&self, owner_node_id: NodeId, destination: &DestinationId) -> u32 {
        let observations = self
            .route_observations()
            .into_iter()
            .filter(|observation| {
                observation.key.owner_node_id == owner_node_id
                    && &observation.key.destination == destination
            })
            .collect::<Vec<_>>();
        let distinct_engines = observations
            .iter()
            .map(|observation| observation.engine_id.clone())
            .collect::<BTreeSet<_>>()
            .len();
        u32::try_from(distinct_engines.saturating_sub(1)).unwrap_or(u32::MAX)
    }

    #[must_use]
    pub fn maintenance_failure_count(&self) -> u32 {
        u32::try_from(
            self.failure_summaries
                .iter()
                .filter(|summary| summary.detail.contains("route maintenance failed"))
                .count(),
        )
        .unwrap_or(u32::MAX)
    }

    #[must_use]
    pub fn failure_class_counts(&self) -> ReducedFailureClassCounts {
        let mut counts = ReducedFailureClassCounts::default();
        for summary in &self.failure_summaries {
            let detail = summary.detail.to_ascii_lowercase();
            if detail.contains("no deterministic checkpoints were emitted during the run")
                || detail.contains("run completed without any route lifecycle events")
                || detail.contains("driver surfaced ")
            {
                continue;
            } else if detail.contains("objective activation failed") {
                counts.activation_failure = counts.activation_failure.saturating_add(1);
            } else if detail.contains("route maintenance failed") {
                counts.maintenance_failure = counts.maintenance_failure.saturating_add(1);
            } else if detail.contains("no candidate") {
                counts.no_candidate = counts.no_candidate.saturating_add(1);
            } else if detail.contains("inadmissible") {
                counts.inadmissible_candidate = counts.inadmissible_candidate.saturating_add(1);
            } else if detail.contains("lost reachability") {
                counts.lost_reachability = counts.lost_reachability.saturating_add(1);
            } else if detail.contains("replacement")
                && (detail.contains("loop") || detail.contains("churn"))
            {
                counts.replacement_loop = counts.replacement_loop.saturating_add(1);
            } else if detail.contains("degraded") {
                counts.persistent_degraded = counts.persistent_degraded.saturating_add(1);
            } else {
                counts.other = counts.other.saturating_add(1);
            }
        }
        counts
    }

    #[must_use]
    pub fn environment_hook_counts(&self) -> ReducedEnvironmentHookCounts {
        let mut counts = ReducedEnvironmentHookCounts::default();
        for round in &self.rounds {
            for hook in &round.environment_hooks {
                match hook_kind(&hook.hook) {
                    EnvironmentHookKind::ReplaceTopology => {
                        counts.replace_topology = counts.replace_topology.saturating_add(1);
                    }
                    EnvironmentHookKind::MediumDegradation => {
                        counts.medium_degradation = counts.medium_degradation.saturating_add(1);
                    }
                    EnvironmentHookKind::AsymmetricDegradation => {
                        counts.asymmetric_degradation =
                            counts.asymmetric_degradation.saturating_add(1);
                    }
                    EnvironmentHookKind::Partition => {
                        counts.partition = counts.partition.saturating_add(1);
                    }
                    EnvironmentHookKind::CascadePartition => {
                        counts.cascade_partition = counts.cascade_partition.saturating_add(1);
                    }
                    EnvironmentHookKind::MobilityRelink => {
                        counts.mobility_relink = counts.mobility_relink.saturating_add(1);
                    }
                    EnvironmentHookKind::IntrinsicLimit => {
                        counts.intrinsic_limit = counts.intrinsic_limit.saturating_add(1);
                    }
                }
            }
        }
        counts
    }

    #[must_use]
    pub fn field_replays_for(&self, local_node_id: NodeId) -> Vec<&FieldReplaySummary> {
        self.rounds
            .iter()
            .flat_map(|round| {
                round
                    .field_replays
                    .iter()
                    .filter(move |entry| entry.local_node_id == local_node_id)
                    .map(|entry| &entry.summary)
            })
            .collect()
    }

    #[must_use]
    pub fn field_route_summaries_for(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Vec<&ActiveRouteSummary> {
        self.rounds
            .iter()
            .flat_map(|round| {
                round.active_routes.iter().filter(move |route| {
                    route.owner_node_id == owner_node_id
                        && &route.destination == destination
                        && route.engine_id == FIELD_ENGINE_ID
                })
            })
            .collect()
    }

    #[must_use]
    pub fn last_field_commitment_resolution(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<String> {
        self.field_route_summaries_for(owner_node_id, destination)
            .into_iter()
            .rev()
            .find_map(|route| route.commitment_resolution.clone())
    }

    #[must_use]
    pub fn last_field_route_outcome(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<String> {
        self.field_route_summaries_for(owner_node_id, destination)
            .into_iter()
            .rev()
            .find_map(|route| route.field_last_outcome.clone())
    }

    #[must_use]
    pub fn last_field_continuity_band(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<String> {
        self.field_route_summaries_for(owner_node_id, destination)
            .into_iter()
            .rev()
            .find_map(|route| route.field_continuity_band.clone())
    }

    #[must_use]
    pub fn last_field_promotion_decision(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<String> {
        self.field_route_summaries_for(owner_node_id, destination)
            .into_iter()
            .rev()
            .find_map(|route| route.field_last_promotion_decision.clone())
    }

    #[must_use]
    pub fn last_field_promotion_blocker(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> Option<String> {
        self.field_route_summaries_for(owner_node_id, destination)
            .into_iter()
            .rev()
            .find_map(|route| route.field_last_promotion_blocker.clone())
    }

    #[must_use]
    pub fn field_continuation_shift_count(
        &self,
        owner_node_id: NodeId,
        destination: &DestinationId,
    ) -> u32 {
        self.field_route_summaries_for(owner_node_id, destination)
            .into_iter()
            .filter_map(|route| route.field_continuation_shift_count)
            .max()
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnvironmentHookKind {
    ReplaceTopology,
    MediumDegradation,
    AsymmetricDegradation,
    Partition,
    CascadePartition,
    MobilityRelink,
    IntrinsicLimit,
}

fn hook_kind(hook: &EnvironmentHook) -> EnvironmentHookKind {
    match hook {
        EnvironmentHook::ReplaceTopology { .. } => EnvironmentHookKind::ReplaceTopology,
        EnvironmentHook::MediumDegradation { .. } => EnvironmentHookKind::MediumDegradation,
        EnvironmentHook::AsymmetricDegradation { .. } => EnvironmentHookKind::AsymmetricDegradation,
        EnvironmentHook::Partition { .. } => EnvironmentHookKind::Partition,
        EnvironmentHook::CascadePartition { .. } => EnvironmentHookKind::CascadePartition,
        EnvironmentHook::MobilityRelink { .. } => EnvironmentHookKind::MobilityRelink,
        EnvironmentHook::IntrinsicLimit { .. } => EnvironmentHookKind::IntrinsicLimit,
    }
}

#[cfg(test)]
mod tests {
    use crate::{presets, replay::ActiveRouteSummary, ReducedReplayRound, ReducedReplayView};
    use jacquard_core::{
        DestinationId, HealthScore, NodeId, ReachabilityState, RouteId, RouteLifecycleEvent,
    };
    use jacquard_pathway::PATHWAY_ENGINE_ID;
    use jacquard_traits::RoutingSimulator;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn active_route(round_next_hop: NodeId) -> ActiveRouteSummary {
        ActiveRouteSummary {
            owner_node_id: node(1),
            route_id: RouteId([7; 16]),
            destination: DestinationId::Node(node(9)),
            engine_id: PATHWAY_ENGINE_ID,
            next_hop_node_id: Some(round_next_hop),
            hop_count_hint: Some(2),
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            reachability_state: ReachabilityState::Reachable,
            stability_score: HealthScore(900),
            commitment_resolution: None,
            field_continuity_band: None,
            field_last_outcome: None,
            field_last_promotion_decision: None,
            field_last_promotion_blocker: None,
            field_continuation_shift_count: None,
            scatter_current_regime: None,
            scatter_last_action: None,
            scatter_retained_message_count: None,
            scatter_delivered_message_count: None,
            scatter_contact_rate: None,
            scatter_diversity_score: None,
            scatter_resource_pressure_permille: None,
        }
    }

    #[test]
    fn reducers_classify_failures_and_hooks_deterministically() {
        let (scenario, environment) = presets::batman_decay_tuning()
            .into_iter()
            .next()
            .expect("batman tuning preset");
        let mut simulator = crate::JacquardSimulator::new(crate::ReferenceClientAdapter);
        let (replay, _) = simulator
            .run_scenario(&scenario, &environment)
            .expect("run simulator");
        let reduced = ReducedReplayView::from_replay(&replay);

        let hook_counts = reduced.environment_hook_counts();
        assert_eq!(hook_counts.cascade_partition, 1);
        assert_eq!(hook_counts.replace_topology, 1);

        let failure_counts = reduced.failure_class_counts();
        assert_eq!(
            failure_counts.no_candidate, 11,
            "reactivation no-candidate summaries should be classified deterministically"
        );
        assert_eq!(failure_counts.inadmissible_candidate, 0);
        assert_eq!(failure_counts.other, 0);
    }

    #[test]
    fn route_observations_split_continuous_routes_on_next_hop_change() {
        let replay = ReducedReplayView {
            scenario_name: "next-hop-churn".to_string(),
            round_count: 3,
            rounds: vec![
                ReducedReplayRound {
                    round_index: 0,
                    active_routes: vec![active_route(node(3))],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 1,
                    active_routes: vec![active_route(node(3))],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 2,
                    active_routes: vec![active_route(node(4))],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
            ],
            distinct_engine_ids: vec![PATHWAY_ENGINE_ID],
            driver_status_events: Vec::new(),
            failure_summaries: Vec::new(),
        };

        let observations = replay.route_observations();
        assert_eq!(observations.len(), 2);
        assert_eq!(observations[0].next_hop_node_id, Some(node(3)));
        assert_eq!(observations[0].first_seen_round, 0);
        assert_eq!(observations[0].last_seen_round, 1);
        assert_eq!(observations[1].next_hop_node_id, Some(node(4)));
        assert_eq!(observations[1].first_seen_round, 2);
        assert_eq!(observations[1].last_seen_round, 2);
    }

    #[test]
    fn usable_route_loss_counts_unreachable_active_routes() {
        let mut unreachable = active_route(node(3));
        unreachable.reachability_state = ReachabilityState::Unreachable;
        let replay = ReducedReplayView {
            scenario_name: "usable-loss".to_string(),
            round_count: 4,
            rounds: vec![
                ReducedReplayRound {
                    round_index: 0,
                    active_routes: vec![active_route(node(3))],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 1,
                    active_routes: vec![unreachable.clone()],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 2,
                    active_routes: vec![unreachable],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 3,
                    active_routes: vec![active_route(node(3))],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
            ],
            distinct_engine_ids: vec![PATHWAY_ENGINE_ID],
            driver_status_events: Vec::new(),
            failure_summaries: Vec::new(),
        };
        let destination = DestinationId::Node(node(9));

        assert_eq!(
            replay.route_present_rounds(node(1), &destination),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            replay.route_usable_rounds(node(1), &destination),
            vec![0, 3]
        );
        assert_eq!(
            replay.first_round_without_usable_route_after_presence(node(1), &destination),
            Some(1)
        );
        assert_eq!(
            replay.usable_recovery_delta_rounds(node(1), &destination),
            Some(2)
        );
    }
}
