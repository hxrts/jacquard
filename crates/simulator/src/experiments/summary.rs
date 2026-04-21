//! Replay reduction: folds per-round events into the stable per-run summary schema.
// long-file-exception: this module keeps the simulator's per-run summary reduction and its audit fixtures together so the exported schema stays traceable to one implementation point.

#![allow(clippy::wildcard_imports)]

use super::*;
use crate::util::stats::min_max_spread_u32;
use crate::{environment::apply_hook, ActiveRouteSummary, ReducedRouteObservation};

fn objective_active_round_count(round_count: u32, activate_at_round: u32) -> u32 {
    round_count.saturating_sub(activate_at_round)
}

fn active_window_route_presence_permille(
    present_rounds: &[u32],
    activate_at_round: u32,
    round_count: u32,
) -> u32 {
    let active_rounds = objective_active_round_count(round_count, activate_at_round);
    let active_present_rounds = u32::try_from(
        present_rounds
            .iter()
            .filter(|round| **round >= activate_at_round)
            .count(),
    )
    .unwrap_or(u32::MAX);
    ratio_permille(active_present_rounds, active_rounds)
}

fn route_observations_for<'a>(
    observations: &'a [ReducedRouteObservation],
    owner_node_id: NodeId,
    destination: &DestinationId,
) -> impl Iterator<Item = &'a ReducedRouteObservation> + 'a {
    let destination = destination.clone();
    observations.iter().filter(move |observation| {
        observation.key.owner_node_id == owner_node_id && observation.key.destination == destination
    })
}

fn route_churn_count_for(
    observations: &[ReducedRouteObservation],
    owner_node_id: NodeId,
    destination: &DestinationId,
) -> u32 {
    let count = route_observations_for(observations, owner_node_id, destination).count();
    u32::try_from(count.saturating_sub(1)).unwrap_or(u32::MAX)
}

fn engine_handoff_count_for(
    observations: &[ReducedRouteObservation],
    owner_node_id: NodeId,
    destination: &DestinationId,
) -> u32 {
    let distinct = route_observations_for(observations, owner_node_id, destination)
        .map(|observation| observation.engine_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    u32::try_from(distinct.saturating_sub(1)).unwrap_or(u32::MAX)
}

fn broker_active_route_counts_for(
    routes: &[&ActiveRouteSummary],
    broker_nodes: &BTreeSet<NodeId>,
) -> (u32, u32, BTreeMap<NodeId, u32>) {
    let mut visible_count = 0u32;
    let mut broker_count = 0u32;
    let mut usage_counts = BTreeMap::new();

    for route in routes {
        if let Some(next_hop_node_id) = route.next_hop_node_id {
            visible_count = visible_count.saturating_add(1);
            if broker_nodes.contains(&next_hop_node_id) {
                broker_count = broker_count.saturating_add(1);
                *usage_counts.entry(next_hop_node_id).or_insert(0) += 1;
            }
        }
    }

    (visible_count, broker_count, usage_counts)
}

fn broker_route_churn_count_for(
    routes: &[&ActiveRouteSummary],
    broker_nodes: &BTreeSet<NodeId>,
) -> u32 {
    let count = routes
        .windows(2)
        .filter(|window| {
            let left = window[0]
                .next_hop_node_id
                .filter(|next_hop_node_id| broker_nodes.contains(next_hop_node_id));
            let right = window[1]
                .next_hop_node_id
                .filter(|next_hop_node_id| broker_nodes.contains(next_hop_node_id));
            (left.is_some() || right.is_some()) && left != right
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn active_routes_for<'a>(
    reduced: &'a ReducedReplayView,
    owner_node_id: NodeId,
    destination: &DestinationId,
) -> Vec<&'a ActiveRouteSummary> {
    reduced
        .rounds
        .iter()
        .flat_map(|round| {
            round.active_routes.iter().filter(move |route| {
                route.owner_node_id == owner_node_id && &route.destination == destination
            })
        })
        .collect()
}

fn route_usable_in_configuration(
    route: &ActiveRouteSummary,
    configuration: &Configuration,
) -> bool {
    if route.reachability_state == jacquard_core::ReachabilityState::Unreachable {
        return false;
    }
    match &route.destination {
        DestinationId::Node(destination_node_id) => {
            configuration_path_exists(configuration, route.owner_node_id, *destination_node_id)
        }
        DestinationId::Gateway(_) | DestinationId::Service(_) => true,
    }
}

fn configuration_path_exists(
    configuration: &Configuration,
    source: NodeId,
    destination: NodeId,
) -> bool {
    let mut visited = BTreeSet::from([source]);
    let mut frontier = vec![source];
    while let Some(current) = frontier.pop() {
        if current == destination {
            return true;
        }
        let mut neighbors = configuration
            .links
            .keys()
            .filter_map(|(left, right)| (*left == current).then_some(*right))
            .collect::<Vec<_>>();
        neighbors.sort();
        neighbors.reverse();
        for neighbor in neighbors {
            if visited.insert(neighbor) {
                frontier.push(neighbor);
            }
        }
    }
    false
}

fn usable_route_timing_for(
    scenario: &JacquardScenario,
    reduced: &ReducedReplayView,
    owner_node_id: NodeId,
    destination: &DestinationId,
) -> (Option<u32>, Option<u32>) {
    let mut configuration = scenario.initial_configuration().value.clone();
    let mut seen_usable = false;
    let mut first_unusable_round = None;
    let mut first_loss_round = None;

    for round in &reduced.rounds {
        for applied in &round.environment_hooks {
            apply_hook(&mut configuration, &applied.hook, applied.at_tick);
        }
        let usable = round.active_routes.iter().any(|route| {
            route.owner_node_id == owner_node_id
                && &route.destination == destination
                && route_usable_in_configuration(route, &configuration)
        });
        if usable {
            if let Some(unusable_round) = first_unusable_round {
                return (
                    first_loss_round,
                    Some(round.round_index.saturating_sub(unusable_round)),
                );
            }
            seen_usable = true;
            continue;
        }
        if seen_usable && first_unusable_round.is_none() {
            first_unusable_round = Some(round.round_index);
            first_loss_round = Some(round.round_index);
        }
    }

    (first_loss_round, None)
}

// long-block-exception: one reducer intentionally computes the stable per-run
// summary schema directly from the replay view in one auditable pass.
pub(super) fn summarize_run(
    spec: &ExperimentRunSpec,
    scenario: &JacquardScenario,
    reduced: &ReducedReplayView,
) -> ExperimentRunSummary {
    let mut objective_count = 0u32;
    let mut activation_successes = 0u32;
    let mut present_round_total = 0u32;
    let mut present_round_total_window_total = 0u32;
    let mut objective_route_presence_permille = Vec::new();
    let mut first_route_rounds = Vec::new();
    let mut first_disruption_rounds = Vec::new();
    let mut first_loss_rounds = Vec::new();
    let mut stale_persistence_rounds = Vec::new();
    let mut recovery_rounds = Vec::new();
    let mut recovery_success_count = 0u32;
    let mut unrecovered_after_loss_count = 0u32;
    let mut objective_starvation_count = 0u32;
    let mut churn_count = 0u32;
    let mut handoff_count = 0u32;
    let mut route_observation_count = 0u32;
    let broker_nodes = scenario
        .broker_nodes()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut broker_visible_observation_count = 0u32;
    let mut broker_participating_observation_count = 0u32;
    let mut broker_usage_counts = BTreeMap::new();
    let mut broker_route_churn_count = 0u32;
    let mut hop_counts = Vec::new();
    let mut stability_scores = Vec::new();
    let owner_nodes = scenario
        .bound_objectives()
        .iter()
        .map(|binding| binding.owner_node_id)
        .collect::<BTreeSet<_>>();
    let route_observations = reduced.route_observations();

    for binding in scenario.bound_objectives() {
        objective_count = objective_count.saturating_add(1);
        let mut objective_route_observations = route_observations_for(
            &route_observations,
            binding.owner_node_id,
            &binding.objective.destination,
        )
        .collect::<Vec<_>>();
        objective_route_observations
            .sort_by_key(|observation| (observation.first_seen_round, observation.last_seen_round));
        let present_rounds =
            reduced.route_present_rounds(binding.owner_node_id, &binding.objective.destination);
        let objective_presence_permille = active_window_route_presence_permille(
            &present_rounds,
            binding.activate_at_round,
            reduced.round_count,
        );
        if reduced.route_seen(binding.owner_node_id, &binding.objective.destination) {
            activation_successes = activation_successes.saturating_add(1);
        }
        present_round_total = present_round_total.saturating_add(objective_presence_permille);
        present_round_total_window_total = present_round_total_window_total
            .saturating_add(u32::try_from(present_rounds.len()).unwrap_or(u32::MAX));
        objective_route_presence_permille.push(objective_presence_permille);
        if objective_presence_permille == 0 {
            objective_starvation_count = objective_starvation_count.saturating_add(1);
        }
        first_route_rounds.push(
            reduced.first_round_with_route(binding.owner_node_id, &binding.objective.destination),
        );
        let first_disruption_round =
            reduced.first_round_with_environment_change_at_or_after(binding.activate_at_round);
        first_disruption_rounds.push(first_disruption_round);
        let (first_loss_round, recovery_round) = usable_route_timing_for(
            scenario,
            reduced,
            binding.owner_node_id,
            &binding.objective.destination,
        );
        first_loss_rounds.push(first_loss_round);
        recovery_rounds.push(recovery_round);
        if recovery_round.is_some() {
            recovery_success_count = recovery_success_count.saturating_add(1);
        } else if first_loss_round.is_some() {
            unrecovered_after_loss_count = unrecovered_after_loss_count.saturating_add(1);
        }
        stale_persistence_rounds.push(match (first_disruption_round, first_loss_round) {
            (Some(disruption), Some(loss_round)) if loss_round >= disruption => {
                Some(loss_round.saturating_sub(disruption))
            }
            (Some(_), Some(_)) => Some(0),
            _ => None,
        });
        churn_count = churn_count.saturating_add(route_churn_count_for(
            &route_observations,
            binding.owner_node_id,
            &binding.objective.destination,
        ));
        handoff_count = handoff_count.saturating_add(engine_handoff_count_for(
            &route_observations,
            binding.owner_node_id,
            &binding.objective.destination,
        ));
        route_observation_count = route_observation_count
            .saturating_add(u32::try_from(objective_route_observations.len()).unwrap_or(u32::MAX));
        if !broker_nodes.is_empty() {
            let objective_active_routes = active_routes_for(
                reduced,
                binding.owner_node_id,
                &binding.objective.destination,
            );
            let (visible_count, participation_count, usage_counts) =
                broker_active_route_counts_for(&objective_active_routes, &broker_nodes);
            broker_visible_observation_count =
                broker_visible_observation_count.saturating_add(visible_count);
            broker_participating_observation_count =
                broker_participating_observation_count.saturating_add(participation_count);
            for (broker_node_id, count) in usage_counts {
                *broker_usage_counts.entry(broker_node_id).or_insert(0) += count;
            }
            broker_route_churn_count = broker_route_churn_count.saturating_add(
                broker_route_churn_count_for(&objective_active_routes, &broker_nodes),
            );
        }
        hop_counts.extend(
            reduced.route_hop_counts(binding.owner_node_id, &binding.objective.destination),
        );
        stability_scores.extend(
            reduced.route_stability_scores(binding.owner_node_id, &binding.objective.destination),
        );
    }
    let broker_metrics_observable =
        !broker_nodes.is_empty() && broker_visible_observation_count > 0;
    let broker_participation_permille = broker_metrics_observable.then(|| {
        ratio_permille(
            broker_participating_observation_count,
            broker_visible_observation_count,
        )
    });
    let broker_concentration_permille = broker_metrics_observable.then(|| {
        ratio_permille(
            broker_usage_counts.values().copied().max().unwrap_or(0),
            broker_participating_observation_count.max(1),
        )
    });
    let broker_route_churn_count = broker_metrics_observable.then_some(broker_route_churn_count);

    let (
        objective_route_presence_min_permille,
        objective_route_presence_max_permille,
        objective_route_presence_spread,
    ) = min_max_spread_u32(objective_route_presence_permille.iter().copied());
    let route_present_permille = average_u32(objective_route_presence_permille.iter().copied());
    let concurrent_route_round_count = u32::try_from(
        reduced
            .rounds
            .iter()
            .filter(|round| {
                scenario
                    .bound_objectives()
                    .iter()
                    .filter(|binding| {
                        round.active_routes.iter().any(|route| {
                            route.owner_node_id == binding.owner_node_id
                                && route.destination == binding.objective.destination
                        })
                    })
                    .count()
                    >= 2
            })
            .count(),
    )
    .unwrap_or(u32::MAX);

    let engine_round_counts = engine_round_counts(reduced);
    let no_route_rounds = reduced
        .rounds
        .iter()
        .filter(|round| round.active_routes.is_empty())
        .count();
    let hook_counts = reduced.environment_hook_counts();
    let failure_counts = reduced.failure_class_counts();
    let classified_failure_count = failure_counts
        .no_candidate
        .saturating_add(failure_counts.inadmissible_candidate)
        .saturating_add(failure_counts.lost_reachability)
        .saturating_add(failure_counts.replacement_loop)
        .saturating_add(failure_counts.maintenance_failure)
        .saturating_add(failure_counts.activation_failure)
        .saturating_add(failure_counts.persistent_degraded)
        .saturating_add(failure_counts.other);
    let activation_failure_count = objective_count.saturating_sub(activation_successes);
    let stability_first = stability_scores.first().copied();
    let stability_last = stability_scores.last().copied();
    let stability_total = stability_scores
        .iter()
        .fold(0u32, |acc, score| acc.saturating_add(*score));
    let mut scatter_sparse_rounds = 0u32;
    let mut scatter_dense_rounds = 0u32;
    let mut scatter_bridging_rounds = 0u32;
    let mut scatter_constrained_rounds = 0u32;
    let mut scatter_replicate_rounds = 0u32;
    let mut scatter_handoff_rounds = 0u32;
    let mut scatter_retained_message_peak = None;
    let mut scatter_delivered_message_peak = None;
    let mut field_selected_result_rounds = 0u32;
    let mut field_search_reconfiguration_rounds = 0u32;
    let mut field_bootstrap_active_rounds = 0u32;
    let mut field_continuity_band = None;
    let mut field_commitment_resolution = None;
    let mut field_last_outcome = None;
    let mut field_last_continuity_transition = None;
    let mut field_last_promotion_decision = None;
    let mut field_last_promotion_blocker = None;
    let mut field_bootstrap_activation_count = 0u32;
    let mut field_bootstrap_hold_count = 0u32;
    let mut field_bootstrap_narrow_count = 0u32;
    let mut field_bootstrap_upgrade_count = 0u32;
    let mut field_bootstrap_withdraw_count = 0u32;
    let mut field_degraded_steady_entry_count = 0u32;
    let mut field_degraded_steady_recovery_count = 0u32;
    let mut field_degraded_to_bootstrap_count = 0u32;
    let mut field_degraded_steady_round_count = 0u32;
    let mut field_service_retention_carry_forward_count = 0u32;
    let mut field_asymmetric_shift_success_count = 0u32;
    let mut field_protocol_reconfiguration_count = 0u32;
    let mut field_route_bound_reconfiguration_count = 0u32;
    let mut field_continuation_shift_count = 0u32;
    let mut field_corridor_narrow_count = 0u32;
    let mut field_checkpoint_restore_count = 0u32;
    for round in &reduced.rounds {
        let scatter_routes = round
            .active_routes
            .iter()
            .filter(|route| route.engine_id == SCATTER_ENGINE_ID)
            .collect::<Vec<_>>();
        if scatter_routes.is_empty() {
            continue;
        }
        match scatter_routes
            .iter()
            .find_map(|route| route.scatter_current_regime.as_deref())
        {
            Some("Sparse") => {
                scatter_sparse_rounds = scatter_sparse_rounds.saturating_add(1);
            }
            Some("Dense") => {
                scatter_dense_rounds = scatter_dense_rounds.saturating_add(1);
            }
            Some("Bridging") => {
                scatter_bridging_rounds = scatter_bridging_rounds.saturating_add(1);
            }
            Some("Constrained") => {
                scatter_constrained_rounds = scatter_constrained_rounds.saturating_add(1);
            }
            _ => {}
        }
        if scatter_routes
            .iter()
            .any(|route| route.scatter_last_action.as_deref() == Some("Replicate"))
        {
            scatter_replicate_rounds = scatter_replicate_rounds.saturating_add(1);
        }
        if scatter_routes
            .iter()
            .any(|route| route.scatter_last_action.as_deref() == Some("PreferentialHandoff"))
        {
            scatter_handoff_rounds = scatter_handoff_rounds.saturating_add(1);
        }
        scatter_retained_message_peak = scatter_routes
            .iter()
            .filter_map(|route| route.scatter_retained_message_count)
            .max()
            .into_iter()
            .chain(scatter_retained_message_peak)
            .max();
        scatter_delivered_message_peak = scatter_routes
            .iter()
            .filter_map(|route| route.scatter_delivered_message_count)
            .max()
            .into_iter()
            .chain(scatter_delivered_message_peak)
            .max();
    }
    for owner_node_id in owner_nodes {
        let field_replays = reduced.field_replays_for(owner_node_id);
        field_selected_result_rounds = field_selected_result_rounds.saturating_add(
            u32::try_from(
                field_replays
                    .iter()
                    .filter(|summary| summary.selected_result_present)
                    .count(),
            )
            .unwrap_or(u32::MAX),
        );
        field_search_reconfiguration_rounds = field_search_reconfiguration_rounds.saturating_add(
            u32::try_from(
                field_replays
                    .iter()
                    .filter(|summary| summary.search_reconfiguration_present)
                    .count(),
            )
            .unwrap_or(u32::MAX),
        );
        field_bootstrap_active_rounds = field_bootstrap_active_rounds.saturating_add(
            u32::try_from(
                field_replays
                    .iter()
                    .filter(|summary| summary.bootstrap_active)
                    .count(),
            )
            .unwrap_or(u32::MAX),
        );
        field_continuity_band = field_continuity_band.or_else(|| {
            field_replays
                .iter()
                .find_map(|summary| summary.continuity_band.clone())
        });
        field_last_continuity_transition = field_last_continuity_transition.or_else(|| {
            field_replays
                .iter()
                .find_map(|summary| summary.last_continuity_transition.clone())
        });
        field_last_promotion_decision = field_last_promotion_decision.or_else(|| {
            field_replays
                .iter()
                .find_map(|summary| summary.last_promotion_decision.clone())
        });
        field_last_promotion_blocker = field_last_promotion_blocker.or_else(|| {
            field_replays
                .iter()
                .find_map(|summary| summary.last_promotion_blocker.clone())
        });
        field_bootstrap_activation_count = field_bootstrap_activation_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_activation_count)
                .max()
                .unwrap_or(0),
        );
        field_bootstrap_hold_count = field_bootstrap_hold_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_hold_count)
                .max()
                .unwrap_or(0),
        );
        field_bootstrap_narrow_count = field_bootstrap_narrow_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_narrow_count)
                .max()
                .unwrap_or(0),
        );
        field_bootstrap_upgrade_count = field_bootstrap_upgrade_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_upgrade_count)
                .max()
                .unwrap_or(0),
        );
        field_bootstrap_withdraw_count = field_bootstrap_withdraw_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_withdraw_count)
                .max()
                .unwrap_or(0),
        );
        field_degraded_steady_entry_count = field_degraded_steady_entry_count.max(
            field_replays
                .iter()
                .map(|summary| summary.degraded_steady_entry_count)
                .max()
                .unwrap_or(0),
        );
        field_degraded_steady_recovery_count = field_degraded_steady_recovery_count.max(
            field_replays
                .iter()
                .map(|summary| summary.degraded_steady_recovery_count)
                .max()
                .unwrap_or(0),
        );
        field_degraded_to_bootstrap_count = field_degraded_to_bootstrap_count.max(
            field_replays
                .iter()
                .map(|summary| summary.degraded_to_bootstrap_count)
                .max()
                .unwrap_or(0),
        );
        field_degraded_steady_round_count = field_degraded_steady_round_count.max(
            field_replays
                .iter()
                .map(|summary| summary.degraded_steady_round_count)
                .max()
                .unwrap_or(0),
        );
        field_service_retention_carry_forward_count = field_service_retention_carry_forward_count
            .max(
                field_replays
                    .iter()
                    .map(|summary| summary.service_retention_carry_forward_count)
                    .max()
                    .unwrap_or(0),
            );
        field_asymmetric_shift_success_count = field_asymmetric_shift_success_count.max(
            field_replays
                .iter()
                .map(|summary| summary.asymmetric_shift_success_count)
                .max()
                .unwrap_or(0),
        );
        field_protocol_reconfiguration_count = field_protocol_reconfiguration_count.max(
            field_replays
                .iter()
                .map(|summary| {
                    u32::try_from(summary.protocol_reconfiguration_count).unwrap_or(u32::MAX)
                })
                .max()
                .unwrap_or(0),
        );
        field_route_bound_reconfiguration_count = field_route_bound_reconfiguration_count.max(
            field_replays
                .iter()
                .map(|summary| {
                    u32::try_from(summary.route_bound_reconfiguration_count).unwrap_or(u32::MAX)
                })
                .max()
                .unwrap_or(0),
        );
        field_continuation_shift_count = field_continuation_shift_count.max(
            field_replays
                .iter()
                .map(|summary| summary.continuation_shift_count)
                .max()
                .unwrap_or(0),
        );
        field_corridor_narrow_count = field_corridor_narrow_count.max(
            field_replays
                .iter()
                .map(|summary| summary.corridor_narrow_count)
                .max()
                .unwrap_or(0),
        );
        field_checkpoint_restore_count = field_checkpoint_restore_count.max(
            field_replays
                .iter()
                .map(|summary| summary.checkpoint_restore_count)
                .max()
                .unwrap_or(0),
        );
    }

    for binding in scenario.bound_objectives() {
        field_commitment_resolution = field_commitment_resolution.or_else(|| {
            reduced.last_field_commitment_resolution(
                binding.owner_node_id,
                &binding.objective.destination,
            )
        });
        field_last_outcome = field_last_outcome.or_else(|| {
            reduced.last_field_route_outcome(binding.owner_node_id, &binding.objective.destination)
        });
        field_continuity_band = field_continuity_band.or_else(|| {
            reduced
                .last_field_continuity_band(binding.owner_node_id, &binding.objective.destination)
        });
        field_last_promotion_decision = field_last_promotion_decision.or_else(|| {
            reduced.last_field_promotion_decision(
                binding.owner_node_id,
                &binding.objective.destination,
            )
        });
        field_last_promotion_blocker = field_last_promotion_blocker.or_else(|| {
            reduced
                .last_field_promotion_blocker(binding.owner_node_id, &binding.objective.destination)
        });
        field_continuation_shift_count =
            field_continuation_shift_count.max(reduced.field_continuation_shift_count(
                binding.owner_node_id,
                &binding.objective.destination,
            ));
    }

    ExperimentRunSummary {
        run_id: spec.run_id.clone(),
        suite_id: spec.suite_id.clone(),
        family_id: spec.family_id.clone(),
        scenario_name: scenario.name().to_string(),
        engine_family: spec.engine_family.clone(),
        execution_lane: spec.execution_lane.label().to_string(),
        config_id: spec.parameters.config_id.clone(),
        comparison_engine_set: spec
            .parameters
            .comparison_engine_set_label()
            .map(str::to_string),
        batman_bellman_stale_after_ticks: spec.parameters.batman_bellman_stale_after_ticks,
        batman_bellman_next_refresh_within_ticks: spec
            .parameters
            .batman_bellman_next_refresh_within_ticks,
        batman_classic_stale_after_ticks: spec.parameters.batman_classic_stale_after_ticks,
        batman_classic_next_refresh_within_ticks: spec
            .parameters
            .batman_classic_next_refresh_within_ticks,
        babel_stale_after_ticks: spec.parameters.babel_stale_after_ticks,
        babel_next_refresh_within_ticks: spec.parameters.babel_next_refresh_within_ticks,
        olsrv2_stale_after_ticks: spec.parameters.olsrv2_stale_after_ticks,
        olsrv2_next_refresh_within_ticks: spec.parameters.olsrv2_next_refresh_within_ticks,
        pathway_query_budget: spec.parameters.pathway_query_budget,
        pathway_heuristic_mode: spec.parameters.pathway_heuristic_mode.clone(),
        scatter_profile_id: spec.parameters.scatter_profile_id.clone(),
        field_query_budget: spec.parameters.field_query_budget,
        field_heuristic_mode: spec.parameters.field_heuristic_mode.clone(),
        field_service_publication_neighbor_limit: spec
            .parameters
            .field_service_publication_neighbor_limit,
        field_service_freshness_weight: spec.parameters.field_service_freshness_weight,
        field_service_narrowing_bias: spec.parameters.field_service_narrowing_bias,
        field_node_bootstrap_support_floor: spec.parameters.field_node_bootstrap_support_floor,
        field_node_bootstrap_top_mass_floor: spec.parameters.field_node_bootstrap_top_mass_floor,
        field_node_bootstrap_entropy_ceiling: spec.parameters.field_node_bootstrap_entropy_ceiling,
        field_node_discovery_enabled: spec.parameters.field_node_discovery_enabled,
        seed: spec.seed.0,
        density: spec.regime.density.clone(),
        loss: spec.regime.loss.clone(),
        interference: spec.regime.interference.clone(),
        asymmetry: spec.regime.asymmetry.clone(),
        churn: spec.regime.churn.clone(),
        node_pressure: spec.regime.node_pressure.clone(),
        objective_regime: spec.regime.objective_regime.clone(),
        stress_score: spec.regime.stress_score,
        objective_count,
        activation_success_permille: ratio_permille(activation_successes, objective_count),
        route_present_permille,
        route_present_total_window_permille: ratio_permille(
            present_round_total_window_total,
            objective_count.saturating_mul(reduced.round_count.max(1)),
        ),
        objective_route_presence_min_permille,
        objective_route_presence_max_permille,
        objective_route_presence_spread,
        objective_starvation_count,
        concurrent_route_round_count,
        first_materialization_round_mean: average_option_u32(&first_route_rounds),
        first_disruption_round_mean: average_option_u32(&first_disruption_rounds),
        first_loss_round_mean: average_option_u32(&first_loss_rounds),
        stale_persistence_round_mean: average_option_u32(&stale_persistence_rounds),
        recovery_round_mean: average_option_u32(&recovery_rounds),
        recovery_success_permille: ratio_permille(recovery_success_count, objective_count.max(1)),
        unrecovered_after_loss_count,
        broker_participation_permille,
        broker_concentration_permille,
        broker_route_churn_count,
        active_route_hop_count_mean: (!hop_counts.is_empty())
            .then(|| average_u32(hop_counts.iter().copied())),
        route_churn_count: churn_count,
        engine_handoff_count: handoff_count,
        route_observation_count,
        batman_bellman_selected_rounds: *engine_round_counts.get("batman-bellman").unwrap_or(&0),
        batman_classic_selected_rounds: *engine_round_counts.get("batman-classic").unwrap_or(&0),
        babel_selected_rounds: *engine_round_counts.get("babel").unwrap_or(&0),
        olsrv2_selected_rounds: *engine_round_counts.get("olsrv2").unwrap_or(&0),
        pathway_selected_rounds: *engine_round_counts.get("pathway").unwrap_or(&0),
        scatter_selected_rounds: *engine_round_counts.get("scatter").unwrap_or(&0),
        scatter_sparse_rounds,
        scatter_dense_rounds,
        scatter_bridging_rounds,
        scatter_constrained_rounds,
        scatter_replicate_rounds,
        scatter_handoff_rounds,
        scatter_retained_message_peak,
        scatter_delivered_message_peak,
        field_selected_rounds: *engine_round_counts.get("field").unwrap_or(&0),
        field_selected_result_rounds,
        field_search_reconfiguration_rounds,
        field_bootstrap_active_rounds,
        field_continuity_band,
        field_commitment_resolution,
        field_last_outcome,
        field_last_continuity_transition,
        field_last_promotion_decision,
        field_last_promotion_blocker,
        field_bootstrap_activation_permille: ratio_permille(
            field_bootstrap_activation_count,
            objective_count.max(1),
        ),
        field_bootstrap_hold_permille: ratio_permille(
            field_bootstrap_hold_count,
            objective_count.max(1),
        ),
        field_bootstrap_narrow_permille: ratio_permille(
            field_bootstrap_narrow_count,
            objective_count.max(1),
        ),
        field_bootstrap_upgrade_permille: ratio_permille(
            field_bootstrap_upgrade_count,
            objective_count.max(1),
        ),
        field_bootstrap_withdraw_permille: ratio_permille(
            field_bootstrap_withdraw_count,
            objective_count.max(1),
        ),
        field_degraded_steady_entry_permille: ratio_permille(
            field_degraded_steady_entry_count,
            objective_count.max(1),
        ),
        field_degraded_steady_recovery_permille: ratio_permille(
            field_degraded_steady_recovery_count,
            objective_count.max(1),
        ),
        field_degraded_to_bootstrap_permille: ratio_permille(
            field_degraded_to_bootstrap_count,
            objective_count.max(1),
        ),
        field_degraded_steady_round_permille: ratio_permille(
            field_degraded_steady_round_count,
            objective_count
                .saturating_mul(reduced.round_count.max(1))
                .max(1),
        ),
        field_service_retention_carry_forward_permille: ratio_permille(
            field_service_retention_carry_forward_count,
            objective_count.max(1),
        ),
        field_asymmetric_shift_success_permille: ratio_permille(
            field_asymmetric_shift_success_count,
            objective_count.max(1),
        ),
        field_protocol_reconfiguration_count,
        field_route_bound_reconfiguration_count,
        field_continuation_shift_count,
        field_corridor_narrow_count,
        field_checkpoint_restore_count,
        no_route_rounds: u32::try_from(no_route_rounds).unwrap_or(u32::MAX),
        dominant_engine: dominant_engine(&engine_round_counts),
        stability_min: stability_scores.iter().copied().min(),
        stability_first,
        stability_last,
        stability_median: median_u32(&stability_scores),
        stability_max: stability_scores.iter().copied().max(),
        stability_total,
        maintenance_failure_count: reduced.maintenance_failure_count(),
        failure_summary_count: classified_failure_count,
        no_candidate_count: failure_counts.no_candidate,
        inadmissible_candidate_count: failure_counts.inadmissible_candidate,
        lost_reachability_count: failure_counts.lost_reachability,
        replacement_loop_count: failure_counts.replacement_loop,
        activation_attempt_failure_count: failure_counts.activation_failure,
        activation_failure_count,
        persistent_degraded_count: failure_counts.persistent_degraded,
        other_failure_count: failure_counts.other,
        replace_topology_count: hook_counts.replace_topology,
        medium_degradation_count: hook_counts.medium_degradation,
        asymmetric_degradation_count: hook_counts.asymmetric_degradation,
        partition_count: hook_counts.partition,
        cascade_partition_count: hook_counts.cascade_partition,
        mobility_relink_count: hook_counts.mobility_relink,
        intrinsic_limit_count: hook_counts.intrinsic_limit,
        model_artifact_count: 0,
        equivalence_passed: None,
    }
}

// long-block-exception: the aggregate summary intentionally stays in one grouped
// reduction so the output schema remains easy to audit against the run schema.
#[must_use]
pub fn aggregate_runs(runs: &[ExperimentRunSummary]) -> Vec<ExperimentAggregateSummary> {
    type AggregateGroupKey = (String, String, String, Option<String>, String);
    type AggregateGroup<'a> = Vec<&'a ExperimentRunSummary>;

    let mut grouped: BTreeMap<AggregateGroupKey, AggregateGroup<'_>> = BTreeMap::new();
    for run in runs {
        grouped
            .entry((
                run.engine_family.clone(),
                run.family_id.clone(),
                run.execution_lane.clone(),
                run.comparison_engine_set.clone(),
                run.config_id.clone(),
            ))
            .or_default()
            .push(run);
    }

    grouped
        .into_values()
        // long-block-exception: one aggregate-group reduction keeps the
        // complete derived schema together at the grouping site.
        .map(|group| {
            let first = group
                .first()
                .expect("experiment aggregate group must be non-empty");
            let run_count = u32::try_from(group.len()).unwrap_or(u32::MAX);
            let engine_mode = mode(group.iter().filter_map(|run| run.dominant_engine.clone()));
            let activation_success_permille_mean =
                average_u32(group.iter().map(|run| run.activation_success_permille));
            let (
                activation_success_permille_min,
                activation_success_permille_max,
                activation_success_permille_spread,
            ) = min_max_spread_u32(group.iter().map(|run| run.activation_success_permille));
            let route_present_permille_mean =
                average_u32(group.iter().map(|run| run.route_present_permille));
            let (
                route_present_permille_min,
                route_present_permille_max,
                route_present_permille_spread,
            ) = min_max_spread_u32(group.iter().map(|run| run.route_present_permille));
            let route_present_total_window_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.route_present_total_window_permille),
            );
            let objective_route_presence_min_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.objective_route_presence_min_permille),
            );
            let objective_route_presence_max_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.objective_route_presence_max_permille),
            );
            let objective_route_presence_spread_mean =
                average_u32(group.iter().map(|run| run.objective_route_presence_spread));
            let objective_starvation_count_mean =
                average_u32(group.iter().map(|run| run.objective_starvation_count));
            let concurrent_route_round_count_mean =
                average_u32(group.iter().map(|run| run.concurrent_route_round_count));
            let first_materialization_round_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.first_materialization_round_mean),
            );
            let first_disruption_round_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.first_disruption_round_mean),
            );
            let first_loss_round_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.first_loss_round_mean));
            let stale_persistence_round_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.stale_persistence_round_mean),
            );
            let recovery_round_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.recovery_round_mean));
            let recovery_success_permille_mean =
                average_u32(group.iter().map(|run| run.recovery_success_permille));
            let unrecovered_after_loss_count_mean =
                average_u32(group.iter().map(|run| run.unrecovered_after_loss_count));
            let broker_participation_permille_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.broker_participation_permille),
            );
            let broker_concentration_permille_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.broker_concentration_permille),
            );
            let broker_route_churn_count_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.broker_route_churn_count));
            let active_route_hop_count_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.active_route_hop_count_mean),
            );
            let route_churn_count_mean = average_u32(group.iter().map(|run| run.route_churn_count));
            let engine_handoff_count_mean =
                average_u32(group.iter().map(|run| run.engine_handoff_count));
            let route_observation_count_mean =
                average_u32(group.iter().map(|run| run.route_observation_count));
            let stability_first_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.stability_first));
            let stability_last_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.stability_last));
            let stability_median_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.stability_median));
            let stability_total_mean = average_u32(group.iter().map(|run| run.stability_total));
            let maintenance_failure_count_mean =
                average_u32(group.iter().map(|run| run.maintenance_failure_count));
            let failure_summary_count_mean =
                average_u32(group.iter().map(|run| run.failure_summary_count));
            let model_artifact_count_mean =
                average_u32(group.iter().map(|run| run.model_artifact_count));
            let equivalence_pass_count = u32::try_from(
                group
                    .iter()
                    .filter(|run| run.equivalence_passed == Some(true))
                    .count(),
            )
            .unwrap_or(u32::MAX);
            let batman_bellman_selected_rounds_mean =
                average_u32(group.iter().map(|run| run.batman_bellman_selected_rounds));
            let batman_classic_selected_rounds_mean =
                average_u32(group.iter().map(|run| run.batman_classic_selected_rounds));
            let babel_selected_rounds_mean =
                average_u32(group.iter().map(|run| run.babel_selected_rounds));
            let field_selected_result_rounds_mean =
                average_u32(group.iter().map(|run| run.field_selected_result_rounds));
            let olsrv2_selected_rounds_mean =
                average_u32(group.iter().map(|run| run.olsrv2_selected_rounds));
            let pathway_selected_rounds_mean =
                average_u32(group.iter().map(|run| run.pathway_selected_rounds));
            let scatter_selected_rounds_mean =
                average_u32(group.iter().map(|run| run.scatter_selected_rounds));
            let scatter_sparse_rounds_mean =
                average_u32(group.iter().map(|run| run.scatter_sparse_rounds));
            let scatter_dense_rounds_mean =
                average_u32(group.iter().map(|run| run.scatter_dense_rounds));
            let scatter_bridging_rounds_mean =
                average_u32(group.iter().map(|run| run.scatter_bridging_rounds));
            let scatter_constrained_rounds_mean =
                average_u32(group.iter().map(|run| run.scatter_constrained_rounds));
            let scatter_replicate_rounds_mean =
                average_u32(group.iter().map(|run| run.scatter_replicate_rounds));
            let scatter_handoff_rounds_mean =
                average_u32(group.iter().map(|run| run.scatter_handoff_rounds));
            let scatter_retained_message_peak_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.scatter_retained_message_peak),
            );
            let scatter_delivered_message_peak_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.scatter_delivered_message_peak),
            );
            let field_selected_rounds_mean =
                average_u32(group.iter().map(|run| run.field_selected_rounds));
            let field_search_reconfiguration_rounds_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_search_reconfiguration_rounds),
            );
            let field_bootstrap_active_rounds_mean =
                average_u32(group.iter().map(|run| run.field_bootstrap_active_rounds));
            let field_continuity_band_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_continuity_band.clone()),
            );
            let field_commitment_resolution_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_commitment_resolution.clone()),
            );
            let field_last_outcome_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_last_outcome.clone()),
            );
            let field_last_continuity_transition_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_last_continuity_transition.clone()),
            );
            let field_last_promotion_decision_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_last_promotion_decision.clone()),
            );
            let field_last_promotion_blocker_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_last_promotion_blocker.clone()),
            );
            let field_bootstrap_activation_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_bootstrap_activation_permille),
            );
            let field_bootstrap_hold_permille_mean =
                average_u32(group.iter().map(|run| run.field_bootstrap_hold_permille));
            let field_bootstrap_narrow_permille_mean =
                average_u32(group.iter().map(|run| run.field_bootstrap_narrow_permille));
            let field_bootstrap_upgrade_permille_mean =
                average_u32(group.iter().map(|run| run.field_bootstrap_upgrade_permille));
            let field_bootstrap_withdraw_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_bootstrap_withdraw_permille),
            );
            let field_degraded_steady_entry_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_degraded_steady_entry_permille),
            );
            let field_degraded_steady_recovery_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_degraded_steady_recovery_permille),
            );
            let field_degraded_to_bootstrap_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_degraded_to_bootstrap_permille),
            );
            let field_degraded_steady_round_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_degraded_steady_round_permille),
            );
            let field_service_retention_carry_forward_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_service_retention_carry_forward_permille),
            );
            let field_asymmetric_shift_success_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_asymmetric_shift_success_permille),
            );
            let field_protocol_reconfiguration_count_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_protocol_reconfiguration_count),
            );
            let field_route_bound_reconfiguration_count_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_route_bound_reconfiguration_count),
            );
            let field_continuation_shift_count_mean =
                average_u32(group.iter().map(|run| run.field_continuation_shift_count));
            let field_corridor_narrow_count_mean =
                average_u32(group.iter().map(|run| run.field_corridor_narrow_count));
            let field_checkpoint_restore_count_mean =
                average_u32(group.iter().map(|run| run.field_checkpoint_restore_count));
            let no_candidate_count_mean =
                average_u32(group.iter().map(|run| run.no_candidate_count));
            let inadmissible_candidate_count_mean =
                average_u32(group.iter().map(|run| run.inadmissible_candidate_count));
            let lost_reachability_count_mean =
                average_u32(group.iter().map(|run| run.lost_reachability_count));
            let replacement_loop_count_mean =
                average_u32(group.iter().map(|run| run.replacement_loop_count));
            let activation_attempt_failure_count_mean =
                average_u32(group.iter().map(|run| run.activation_attempt_failure_count));
            let activation_failure_count_mean =
                average_u32(group.iter().map(|run| run.activation_failure_count));
            let persistent_degraded_count_mean =
                average_u32(group.iter().map(|run| run.persistent_degraded_count));
            let other_failure_count_mean =
                average_u32(group.iter().map(|run| run.other_failure_count));
            let cascade_partition_count_mean =
                average_u32(group.iter().map(|run| run.cascade_partition_count));
            let intrinsic_limit_count_mean =
                average_u32(group.iter().map(|run| run.intrinsic_limit_count));
            let acceptable = activation_success_permille_mean >= 900
                && route_present_total_window_permille_mean >= 500
                && lost_reachability_count_mean == 0
                && maintenance_failure_count_mean == 0;

            ExperimentAggregateSummary {
                suite_id: first.suite_id.clone(),
                family_id: first.family_id.clone(),
                engine_family: first.engine_family.clone(),
                execution_lane: first.execution_lane.clone(),
                config_id: first.config_id.clone(),
                comparison_engine_set: first.comparison_engine_set.clone(),
                batman_bellman_stale_after_ticks: first.batman_bellman_stale_after_ticks,
                batman_bellman_next_refresh_within_ticks: first
                    .batman_bellman_next_refresh_within_ticks,
                batman_classic_stale_after_ticks: first.batman_classic_stale_after_ticks,
                batman_classic_next_refresh_within_ticks: first
                    .batman_classic_next_refresh_within_ticks,
                babel_stale_after_ticks: first.babel_stale_after_ticks,
                babel_next_refresh_within_ticks: first.babel_next_refresh_within_ticks,
                olsrv2_stale_after_ticks: first.olsrv2_stale_after_ticks,
                olsrv2_next_refresh_within_ticks: first.olsrv2_next_refresh_within_ticks,
                pathway_query_budget: first.pathway_query_budget,
                pathway_heuristic_mode: first.pathway_heuristic_mode.clone(),
                scatter_profile_id: first.scatter_profile_id.clone(),
                field_query_budget: first.field_query_budget,
                field_heuristic_mode: first.field_heuristic_mode.clone(),
                field_service_publication_neighbor_limit: first
                    .field_service_publication_neighbor_limit,
                field_service_freshness_weight: first.field_service_freshness_weight,
                field_service_narrowing_bias: first.field_service_narrowing_bias,
                field_node_bootstrap_support_floor: first.field_node_bootstrap_support_floor,
                field_node_bootstrap_top_mass_floor: first.field_node_bootstrap_top_mass_floor,
                field_node_bootstrap_entropy_ceiling: first.field_node_bootstrap_entropy_ceiling,
                field_node_discovery_enabled: first.field_node_discovery_enabled,
                density: first.density.clone(),
                loss: first.loss.clone(),
                interference: first.interference.clone(),
                asymmetry: first.asymmetry.clone(),
                churn: first.churn.clone(),
                node_pressure: first.node_pressure.clone(),
                objective_regime: first.objective_regime.clone(),
                stress_score: first.stress_score,
                run_count,
                activation_success_permille_mean,
                activation_success_permille_min,
                activation_success_permille_max,
                activation_success_permille_spread,
                route_present_permille_mean,
                route_present_permille_min,
                route_present_permille_max,
                route_present_permille_spread,
                route_present_total_window_permille_mean,
                objective_route_presence_min_permille_mean,
                objective_route_presence_max_permille_mean,
                objective_route_presence_spread_mean,
                objective_starvation_count_mean,
                concurrent_route_round_count_mean,
                first_materialization_round_mean,
                first_disruption_round_mean,
                first_loss_round_mean,
                stale_persistence_round_mean,
                recovery_round_mean,
                recovery_success_permille_mean,
                unrecovered_after_loss_count_mean,
                broker_participation_permille_mean,
                broker_concentration_permille_mean,
                broker_route_churn_count_mean,
                active_route_hop_count_mean,
                route_churn_count_mean,
                engine_handoff_count_mean,
                route_observation_count_mean,
                dominant_engine: engine_mode,
                batman_bellman_selected_rounds_mean,
                batman_classic_selected_rounds_mean,
                babel_selected_rounds_mean,
                olsrv2_selected_rounds_mean,
                pathway_selected_rounds_mean,
                scatter_selected_rounds_mean,
                scatter_sparse_rounds_mean,
                scatter_dense_rounds_mean,
                scatter_bridging_rounds_mean,
                scatter_constrained_rounds_mean,
                scatter_replicate_rounds_mean,
                scatter_handoff_rounds_mean,
                scatter_retained_message_peak_mean,
                scatter_delivered_message_peak_mean,
                field_selected_rounds_mean,
                field_selected_result_rounds_mean,
                field_search_reconfiguration_rounds_mean,
                field_bootstrap_active_rounds_mean,
                field_continuity_band_mode,
                field_commitment_resolution_mode,
                field_last_outcome_mode,
                field_last_continuity_transition_mode,
                field_last_promotion_decision_mode,
                field_last_promotion_blocker_mode,
                field_bootstrap_activation_permille_mean,
                field_bootstrap_hold_permille_mean,
                field_bootstrap_narrow_permille_mean,
                field_bootstrap_upgrade_permille_mean,
                field_bootstrap_withdraw_permille_mean,
                field_degraded_steady_entry_permille_mean,
                field_degraded_steady_recovery_permille_mean,
                field_degraded_to_bootstrap_permille_mean,
                field_degraded_steady_round_permille_mean,
                field_service_retention_carry_forward_permille_mean,
                field_asymmetric_shift_success_permille_mean,
                field_protocol_reconfiguration_count_mean,
                field_route_bound_reconfiguration_count_mean,
                field_continuation_shift_count_mean,
                field_corridor_narrow_count_mean,
                field_checkpoint_restore_count_mean,
                stability_first_mean,
                stability_last_mean,
                stability_median_mean,
                stability_total_mean,
                maintenance_failure_count_mean,
                failure_summary_count_mean,
                no_candidate_count_mean,
                inadmissible_candidate_count_mean,
                lost_reachability_count_mean,
                replacement_loop_count_mean,
                activation_attempt_failure_count_mean,
                activation_failure_count_mean,
                persistent_degraded_count_mean,
                other_failure_count_mean,
                cascade_partition_count_mean,
                intrinsic_limit_count_mean,
                model_artifact_count_mean,
                equivalence_pass_count,
                acceptable,
            }
        })
        .collect()
}

#[must_use]
pub fn summarize_breakdowns(
    aggregates: &[ExperimentAggregateSummary],
) -> Vec<ExperimentBreakdownSummary> {
    let mut grouped: BTreeMap<(String, String, String), Vec<&ExperimentAggregateSummary>> =
        BTreeMap::new();
    for aggregate in aggregates {
        grouped
            .entry((
                aggregate.engine_family.clone(),
                aggregate.execution_lane.clone(),
                aggregate.config_id.clone(),
            ))
            .or_default()
            .push(aggregate);
    }

    grouped
        .into_iter()
        .map(|((engine_family, execution_lane, config_id), mut group)| {
            group.sort_by_key(|aggregate| (aggregate.stress_score, aggregate.family_id.clone()));
            let max_sustained_stress_score = group
                .iter()
                .filter(|aggregate| aggregate.acceptable)
                .map(|aggregate| aggregate.stress_score)
                .max()
                .unwrap_or(0);
            let first_failed = group.iter().find(|aggregate| !aggregate.acceptable);
            let breakdown_reason = first_failed.map(|aggregate| {
                if aggregate.activation_success_permille_mean < 900 {
                    "activation-success".to_string()
                } else if aggregate.route_present_total_window_permille_mean < 500 {
                    "route-presence".to_string()
                } else if aggregate.lost_reachability_count_mean > 0 {
                    "lost-reachability".to_string()
                } else if aggregate.maintenance_failure_count_mean > 0 {
                    "maintenance-failure".to_string()
                } else if aggregate.unrecovered_after_loss_count_mean > 0 {
                    "unrecovered-loss".to_string()
                } else {
                    "failure-density".to_string()
                }
            });
            ExperimentBreakdownSummary {
                suite_id: group
                    .first()
                    .expect("breakdown groups must be non-empty")
                    .suite_id
                    .clone(),
                engine_family,
                execution_lane,
                config_id,
                max_sustained_stress_score,
                first_failed_family_id: first_failed.map(|aggregate| aggregate.family_id.clone()),
                first_failed_stress_score: first_failed.map(|aggregate| aggregate.stress_score),
                breakdown_reason,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        environment::AppliedEnvironmentHook, ActiveRouteSummary, ReducedReplayRound,
        SimulationExecutionLane, SimulationFailureSummary,
    };
    use jacquard_core::{HealthScore, RouteId, RouteLifecycleEvent};

    fn synthetic_summary_spec() -> ExperimentRunSpec {
        let owner_node_id = node_id(1);
        let destination_node_id = node_id(2);
        let topology = topology_from_byte_nodes_and_edges(
            comparison_topology_nodes_for_bytes(&[1, 2], None),
            &[(1, 2)],
            1,
        );
        let scenario = route_visible_template(
            "summary-synthetic-recovery".to_string(),
            SimulationSeed(41),
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(owner_node_id).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(destination_node_id)
                    .with_profile(best_effort_connected_profile()),
            ],
            vec![
                BoundObjective::new(owner_node_id, connected_objective(destination_node_id))
                    .with_activation_round(1),
            ],
            6,
        )
        .into_scenario(&ExperimentParameterSet::pathway(
            4,
            PathwaySearchHeuristicMode::Zero,
        ))
        .with_broker_nodes(vec![destination_node_id]);
        ExperimentRunSpec {
            run_id: "summary-synthetic-recovery".to_string(),
            suite_id: "summary-tests".to_string(),
            family_id: "summary-synthetic-recovery".to_string(),
            engine_family: "pathway".to_string(),
            execution_lane: SimulationExecutionLane::FullStack,
            seed: SimulationSeed(41),
            regime: regime((
                "synthetic",
                "low",
                "low",
                "none",
                "single-repair",
                "none",
                "connected-only",
                12,
            )),
            parameters: ExperimentParameterSet::pathway(4, PathwaySearchHeuristicMode::Zero),
            world: ExperimentRunWorld::Prepared {
                scenario: Box::new(scenario),
                environment: ScriptedEnvironmentModel::default(),
            },
            model_case: None,
        }
    }

    fn broker_test_route(
        owner_node_id: NodeId,
        destination: DestinationId,
        route_id_byte: u8,
        next_hop_node_id: NodeId,
        last_lifecycle_event: RouteLifecycleEvent,
    ) -> ActiveRouteSummary {
        ActiveRouteSummary {
            owner_node_id,
            route_id: RouteId([route_id_byte; 16]),
            destination,
            engine_id: PATHWAY_ENGINE_ID,
            next_hop_node_id: Some(next_hop_node_id),
            last_lifecycle_event,
            hop_count_hint: Some(2),
            reachability_state: jacquard_core::ReachabilityState::Reachable,
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

    // long-block-exception: this synthetic replay fixture keeps the full reduced surface sample in one place for auditability.
    fn synthetic_reduced_replay() -> ReducedReplayView {
        let owner_node_id = node_id(1);
        let destination_node_id = node_id(2);
        let destination = DestinationId::Node(destination_node_id);
        let route = ActiveRouteSummary {
            owner_node_id,
            route_id: RouteId([7; 16]),
            destination: destination.clone(),
            engine_id: PATHWAY_ENGINE_ID,
            next_hop_node_id: Some(destination_node_id),
            hop_count_hint: Some(1),
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            reachability_state: jacquard_core::ReachabilityState::Reachable,
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
        };
        ReducedReplayView {
            scenario_name: "summary-synthetic-recovery".to_string(),
            round_count: 6,
            rounds: vec![
                ReducedReplayRound {
                    round_index: 0,
                    active_routes: Vec::new(),
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 1,
                    active_routes: vec![route.clone()],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 2,
                    active_routes: vec![route.clone()],
                    environment_hooks: vec![AppliedEnvironmentHook {
                        at_tick: Tick(2),
                        hook: EnvironmentHook::ReplaceTopology {
                            configuration: Observation {
                                value: topology_from_byte_nodes_and_edges(
                                    comparison_topology_nodes_for_bytes(&[1, 2], None),
                                    &[(1, 2)],
                                    1,
                                )
                                .value,
                                source_class: FactSourceClass::Local,
                                evidence_class: RoutingEvidenceClass::DirectObservation,
                                origin_authentication: OriginAuthenticationClass::Controlled,
                                observed_at_tick: Tick(2),
                            }
                            .value,
                        },
                    }],
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 3,
                    active_routes: Vec::new(),
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 4,
                    active_routes: Vec::new(),
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 5,
                    active_routes: vec![ActiveRouteSummary {
                        last_lifecycle_event: RouteLifecycleEvent::Repaired,
                        ..route
                    }],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
            ],
            distinct_engine_ids: vec![PATHWAY_ENGINE_ID],
            driver_status_events: Vec::new(),
            failure_summaries: Vec::new(),
        }
    }

    #[test]
    fn active_window_route_presence_reaches_full_score_after_delayed_activation() {
        let present_rounds = (2..18).collect::<Vec<_>>();

        assert_eq!(
            active_window_route_presence_permille(&present_rounds, 2, 18),
            1000
        );
        assert_eq!(ratio_permille(16, 18), 888);
    }

    #[test]
    fn active_window_route_presence_ignores_pre_activation_dead_time() {
        let present_rounds = vec![3, 4, 5, 6, 7];

        assert_eq!(
            active_window_route_presence_permille(&present_rounds, 3, 8),
            1000
        );
    }

    #[test]
    fn summarize_run_reports_hand_checked_stale_repair_metrics() {
        let spec = synthetic_summary_spec();
        let scenario = spec
            .prepared_scenario()
            .expect("synthetic summary spec should retain a prepared scenario");
        let summary = summarize_run(&spec, scenario, &synthetic_reduced_replay());

        assert_eq!(summary.first_disruption_round_mean, Some(2));
        assert_eq!(summary.first_loss_round_mean, Some(3));
        assert_eq!(summary.stale_persistence_round_mean, Some(1));
        assert_eq!(summary.recovery_round_mean, Some(2));
        assert_eq!(summary.recovery_success_permille, 1000);
        assert_eq!(summary.unrecovered_after_loss_count, 0);
        assert_eq!(summary.broker_participation_permille, Some(1000));
        assert_eq!(summary.broker_concentration_permille, Some(1000));
        assert_eq!(summary.broker_route_churn_count, Some(0));
        assert_eq!(summary.route_present_permille, 600);
        assert_eq!(summary.route_present_total_window_permille, 500);
        assert_eq!(summary.route_churn_count, 1);
        assert_eq!(summary.route_observation_count, 2);
    }

    #[test]
    fn aggregate_runs_uses_window_normalized_route_presence_for_acceptance() {
        let spec = synthetic_summary_spec();
        let scenario = spec
            .prepared_scenario()
            .expect("synthetic summary spec should retain a prepared scenario");
        let mut run = summarize_run(&spec, scenario, &synthetic_reduced_replay());
        run.route_present_permille = 600;
        run.route_present_total_window_permille = 400;
        let aggregate = aggregate_runs(&[run])
            .into_iter()
            .next()
            .expect("single run should yield one aggregate");

        assert_eq!(aggregate.route_present_permille_mean, 600);
        assert_eq!(aggregate.route_present_total_window_permille_mean, 400);
        assert!(!aggregate.acceptable);
    }

    #[test]
    fn summarize_breakdowns_reports_route_presence_from_normalized_window_metric() {
        let spec = synthetic_summary_spec();
        let scenario = spec
            .prepared_scenario()
            .expect("synthetic summary spec should retain a prepared scenario");
        let mut run = summarize_run(&spec, scenario, &synthetic_reduced_replay());
        run.route_present_permille = 600;
        run.route_present_total_window_permille = 400;
        let aggregate = aggregate_runs(&[run])
            .into_iter()
            .next()
            .expect("single run should yield one aggregate");
        let breakdown = summarize_breakdowns(&[aggregate])
            .into_iter()
            .next()
            .expect("single aggregate should yield one breakdown");

        assert_eq!(
            breakdown.breakdown_reason.as_deref(),
            Some("route-presence")
        );
    }

    #[test]
    fn reduced_failure_class_counts_ignore_generic_harness_summaries() {
        let mut replay = synthetic_reduced_replay();
        replay.failure_summaries = vec![
            SimulationFailureSummary {
                round_index: None,
                detail: "run completed without any route lifecycle events".to_string(),
            },
            SimulationFailureSummary {
                round_index: None,
                detail: "driver surfaced 2 status event(s) during the run".to_string(),
            },
            SimulationFailureSummary {
                round_index: None,
                detail: "objective activation failed for owner NodeId(1): missing host bridge"
                    .to_string(),
            },
        ];

        let counts = replay.failure_class_counts();
        assert_eq!(counts.activation_failure, 1);
        assert_eq!(counts.other, 0);
    }

    #[test]
    fn summarize_run_separates_activation_attempt_failures_from_terminal_failures() {
        let spec = synthetic_summary_spec();
        let scenario = spec
            .prepared_scenario()
            .expect("synthetic summary spec should retain a prepared scenario");
        let mut replay = synthetic_reduced_replay();
        replay.failure_summaries = vec![
            SimulationFailureSummary {
                round_index: None,
                detail: "run completed without any route lifecycle events".to_string(),
            },
            SimulationFailureSummary {
                round_index: Some(0),
                detail:
                    "objective activation failed for owner NodeId(1) destination NodeId(2): no candidate"
                        .to_string(),
            },
        ];

        let summary = summarize_run(&spec, scenario, &replay);
        assert_eq!(summary.failure_summary_count, 1);
        assert_eq!(summary.activation_attempt_failure_count, 1);
        assert_eq!(summary.activation_failure_count, 0);
        assert_eq!(summary.activation_success_permille, 1000);
    }

    #[test]
    fn broker_route_churn_counts_only_broker_identity_changes() {
        let broker_a = node_id(3);
        let broker_b = node_id(4);
        let owner_node_id = node_id(1);
        let destination = DestinationId::Node(node_id(9));
        let routes = [
            broker_test_route(
                owner_node_id,
                destination.clone(),
                1,
                broker_a,
                RouteLifecycleEvent::Activated,
            ),
            broker_test_route(
                owner_node_id,
                destination,
                2,
                broker_b,
                RouteLifecycleEvent::Repaired,
            ),
        ];

        let refs = routes.iter().collect::<Vec<_>>();
        let brokers = BTreeSet::from([broker_a, broker_b]);
        assert_eq!(broker_route_churn_count_for(&refs, &brokers), 1);
    }

    #[test]
    fn summarize_run_counts_broker_churn_inside_continuous_route_lifetime() {
        let spec = synthetic_summary_spec();
        let (scenario, _) = spec.materialize_world();
        let scenario = scenario.with_broker_nodes(vec![node_id(3), node_id(4)]);
        let owner_node_id = node_id(1);
        let destination = DestinationId::Node(node_id(2));
        let route = ActiveRouteSummary {
            owner_node_id,
            route_id: RouteId([7; 16]),
            destination: destination.clone(),
            engine_id: PATHWAY_ENGINE_ID,
            next_hop_node_id: Some(node_id(3)),
            hop_count_hint: Some(2),
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            reachability_state: jacquard_core::ReachabilityState::Reachable,
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
        };
        let mut switched = route.clone();
        switched.next_hop_node_id = Some(node_id(4));
        let replay = ReducedReplayView {
            scenario_name: "summary-synthetic-broker-churn".to_string(),
            round_count: 2,
            rounds: vec![
                ReducedReplayRound {
                    round_index: 0,
                    active_routes: vec![route],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 1,
                    active_routes: vec![switched],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
            ],
            distinct_engine_ids: vec![PATHWAY_ENGINE_ID],
            driver_status_events: Vec::new(),
            failure_summaries: Vec::new(),
        };

        let summary = summarize_run(&spec, &scenario, &replay);
        assert_eq!(summary.broker_participation_permille, Some(1000));
        assert_eq!(summary.broker_concentration_permille, Some(500));
        assert_eq!(summary.broker_route_churn_count, Some(1));
    }

    // long-block-exception: this regression fixture exercises every scatter runtime counter in one replay summary.
    #[test]
    fn summarize_run_tracks_scatter_runtime_regimes_actions_and_peaks() {
        let mut spec = synthetic_summary_spec();
        spec.engine_family = "scatter".to_string();
        spec.parameters = ExperimentParameterSet::scatter("balanced");
        let scenario = spec
            .prepared_scenario()
            .expect("synthetic summary spec should retain a prepared scenario");
        let owner_node_id = node_id(1);
        let destination = DestinationId::Node(node_id(2));
        let replay = ReducedReplayView {
            scenario_name: "summary-synthetic-scatter".to_string(),
            round_count: 4,
            rounds: vec![
                ReducedReplayRound {
                    round_index: 0,
                    active_routes: vec![ActiveRouteSummary {
                        owner_node_id,
                        route_id: RouteId([1; 16]),
                        destination: destination.clone(),
                        engine_id: SCATTER_ENGINE_ID,
                        next_hop_node_id: Some(node_id(2)),
                        hop_count_hint: Some(1),
                        last_lifecycle_event: RouteLifecycleEvent::Activated,
                        reachability_state: jacquard_core::ReachabilityState::Reachable,
                        stability_score: HealthScore(900),
                        commitment_resolution: None,
                        field_continuity_band: None,
                        field_last_outcome: None,
                        field_last_promotion_decision: None,
                        field_last_promotion_blocker: None,
                        field_continuation_shift_count: None,
                        scatter_current_regime: Some("Sparse".to_string()),
                        scatter_last_action: Some("Replicate".to_string()),
                        scatter_retained_message_count: Some(3),
                        scatter_delivered_message_count: Some(1),
                        scatter_contact_rate: Some(7),
                        scatter_diversity_score: Some(2),
                        scatter_resource_pressure_permille: Some(150),
                    }],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 1,
                    active_routes: vec![ActiveRouteSummary {
                        owner_node_id,
                        route_id: RouteId([2; 16]),
                        destination: destination.clone(),
                        engine_id: SCATTER_ENGINE_ID,
                        next_hop_node_id: Some(node_id(2)),
                        hop_count_hint: Some(1),
                        last_lifecycle_event: RouteLifecycleEvent::Repaired,
                        reachability_state: jacquard_core::ReachabilityState::Reachable,
                        stability_score: HealthScore(910),
                        commitment_resolution: None,
                        field_continuity_band: None,
                        field_last_outcome: None,
                        field_last_promotion_decision: None,
                        field_last_promotion_blocker: None,
                        field_continuation_shift_count: None,
                        scatter_current_regime: Some("Dense".to_string()),
                        scatter_last_action: Some("KeepCarrying".to_string()),
                        scatter_retained_message_count: Some(5),
                        scatter_delivered_message_count: Some(2),
                        scatter_contact_rate: Some(8),
                        scatter_diversity_score: Some(3),
                        scatter_resource_pressure_permille: Some(180),
                    }],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 2,
                    active_routes: vec![ActiveRouteSummary {
                        owner_node_id,
                        route_id: RouteId([3; 16]),
                        destination: destination.clone(),
                        engine_id: SCATTER_ENGINE_ID,
                        next_hop_node_id: Some(node_id(2)),
                        hop_count_hint: Some(1),
                        last_lifecycle_event: RouteLifecycleEvent::Repaired,
                        reachability_state: jacquard_core::ReachabilityState::Reachable,
                        stability_score: HealthScore(920),
                        commitment_resolution: None,
                        field_continuity_band: None,
                        field_last_outcome: None,
                        field_last_promotion_decision: None,
                        field_last_promotion_blocker: None,
                        field_continuation_shift_count: None,
                        scatter_current_regime: Some("Bridging".to_string()),
                        scatter_last_action: Some("PreferentialHandoff".to_string()),
                        scatter_retained_message_count: Some(4),
                        scatter_delivered_message_count: Some(6),
                        scatter_contact_rate: Some(9),
                        scatter_diversity_score: Some(4),
                        scatter_resource_pressure_permille: Some(210),
                    }],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
                ReducedReplayRound {
                    round_index: 3,
                    active_routes: vec![ActiveRouteSummary {
                        owner_node_id,
                        route_id: RouteId([4; 16]),
                        destination,
                        engine_id: SCATTER_ENGINE_ID,
                        next_hop_node_id: Some(node_id(2)),
                        hop_count_hint: Some(1),
                        last_lifecycle_event: RouteLifecycleEvent::Repaired,
                        reachability_state: jacquard_core::ReachabilityState::Reachable,
                        stability_score: HealthScore(930),
                        commitment_resolution: None,
                        field_continuity_band: None,
                        field_last_outcome: None,
                        field_last_promotion_decision: None,
                        field_last_promotion_blocker: None,
                        field_continuation_shift_count: None,
                        scatter_current_regime: Some("Constrained".to_string()),
                        scatter_last_action: Some("Replicate".to_string()),
                        scatter_retained_message_count: Some(2),
                        scatter_delivered_message_count: Some(4),
                        scatter_contact_rate: Some(6),
                        scatter_diversity_score: Some(1),
                        scatter_resource_pressure_permille: Some(900),
                    }],
                    environment_hooks: Vec::new(),
                    field_replays: Vec::new(),
                },
            ],
            distinct_engine_ids: vec![SCATTER_ENGINE_ID],
            driver_status_events: Vec::new(),
            failure_summaries: Vec::new(),
        };

        let summary = summarize_run(&spec, scenario, &replay);
        assert_eq!(summary.scatter_selected_rounds, 4);
        assert_eq!(summary.scatter_sparse_rounds, 1);
        assert_eq!(summary.scatter_dense_rounds, 1);
        assert_eq!(summary.scatter_bridging_rounds, 1);
        assert_eq!(summary.scatter_constrained_rounds, 1);
        assert_eq!(summary.scatter_replicate_rounds, 2);
        assert_eq!(summary.scatter_handoff_rounds, 1);
        assert_eq!(summary.scatter_retained_message_peak, Some(5));
        assert_eq!(summary.scatter_delivered_message_peak, Some(6));
    }
}
