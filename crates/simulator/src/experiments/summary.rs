//! Replay reduction: folds per-round events into the stable per-run summary schema.

#![allow(clippy::wildcard_imports)]

use super::*;

// long-block-exception: one reducer intentionally computes the stable per-run
// summary schema directly from the replay view in one auditable pass.
pub(super) fn summarize_run(
    spec: &ExperimentRunSpec,
    reduced: &ReducedReplayView,
) -> ExperimentRunSummary {
    let mut objective_count = 0u32;
    let mut activation_successes = 0u32;
    let mut present_round_total = 0u32;
    let mut first_route_rounds = Vec::new();
    let mut first_loss_rounds = Vec::new();
    let mut recovery_rounds = Vec::new();
    let mut churn_count = 0u32;
    let mut handoff_count = 0u32;
    let mut route_observation_count = 0u32;
    let mut stability_scores = Vec::new();
    let owner_nodes = spec
        .scenario
        .bound_objectives()
        .iter()
        .map(|binding| binding.owner_node_id)
        .collect::<BTreeSet<_>>();

    for binding in spec.scenario.bound_objectives() {
        objective_count = objective_count.saturating_add(1);
        if reduced.route_seen(binding.owner_node_id, &binding.objective.destination) {
            activation_successes = activation_successes.saturating_add(1);
        }
        present_round_total = present_round_total.saturating_add(
            u32::try_from(
                reduced
                    .route_present_rounds(binding.owner_node_id, &binding.objective.destination)
                    .len(),
            )
            .unwrap_or(u32::MAX),
        );
        first_route_rounds.push(
            reduced.first_round_with_route(binding.owner_node_id, &binding.objective.destination),
        );
        first_loss_rounds.push(reduced.first_round_without_route_after_presence(
            binding.owner_node_id,
            &binding.objective.destination,
        ));
        recovery_rounds.push(
            reduced.recovery_delta_rounds(binding.owner_node_id, &binding.objective.destination),
        );
        churn_count = churn_count.saturating_add(
            reduced.route_churn_count(binding.owner_node_id, &binding.objective.destination),
        );
        handoff_count = handoff_count.saturating_add(
            reduced.engine_handoff_count(binding.owner_node_id, &binding.objective.destination),
        );
        route_observation_count = route_observation_count.saturating_add(
            u32::try_from(
                reduced
                    .route_observations()
                    .into_iter()
                    .filter(|observation| {
                        observation.key.owner_node_id == binding.owner_node_id
                            && observation.key.destination == binding.objective.destination
                    })
                    .count(),
            )
            .unwrap_or(u32::MAX),
        );
        stability_scores.extend(
            reduced.route_stability_scores(binding.owner_node_id, &binding.objective.destination),
        );
    }

    let engine_round_counts = engine_round_counts(reduced);
    let no_route_rounds = reduced
        .rounds
        .iter()
        .filter(|round| round.active_routes.is_empty())
        .count();
    let hook_counts = reduced.environment_hook_counts();
    let failure_counts = reduced.failure_class_counts();
    let stability_first = stability_scores.first().copied();
    let stability_last = stability_scores.last().copied();
    let stability_total = stability_scores
        .iter()
        .fold(0u32, |acc, score| acc.saturating_add(*score));
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

    for binding in spec.scenario.bound_objectives() {
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
        scenario_name: spec.scenario.name().to_string(),
        engine_family: spec.engine_family.clone(),
        config_id: spec.parameters.config_id.clone(),
        comparison_engine_set: spec.parameters.comparison_engine_set.clone(),
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
        route_present_permille: ratio_permille(
            present_round_total,
            objective_count.saturating_mul(reduced.round_count.max(1)),
        ),
        first_materialization_round_mean: average_option_u32(&first_route_rounds),
        first_loss_round_mean: average_option_u32(&first_loss_rounds),
        recovery_round_mean: average_option_u32(&recovery_rounds),
        route_churn_count: churn_count,
        engine_handoff_count: handoff_count,
        route_observation_count,
        batman_bellman_selected_rounds: *engine_round_counts.get("batman-bellman").unwrap_or(&0),
        batman_classic_selected_rounds: *engine_round_counts.get("batman-classic").unwrap_or(&0),
        babel_selected_rounds: *engine_round_counts.get("babel").unwrap_or(&0),
        olsrv2_selected_rounds: *engine_round_counts.get("olsrv2").unwrap_or(&0),
        pathway_selected_rounds: *engine_round_counts.get("pathway").unwrap_or(&0),
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
        failure_summary_count: u32::try_from(reduced.failure_summaries.len()).unwrap_or(u32::MAX),
        no_candidate_count: failure_counts.no_candidate,
        inadmissible_candidate_count: failure_counts.inadmissible_candidate,
        lost_reachability_count: failure_counts.lost_reachability,
        replacement_loop_count: failure_counts.replacement_loop,
        activation_failure_count: failure_counts.activation_failure,
        persistent_degraded_count: failure_counts.persistent_degraded,
        other_failure_count: failure_counts.other,
        replace_topology_count: hook_counts.replace_topology,
        medium_degradation_count: hook_counts.medium_degradation,
        asymmetric_degradation_count: hook_counts.asymmetric_degradation,
        partition_count: hook_counts.partition,
        cascade_partition_count: hook_counts.cascade_partition,
        mobility_relink_count: hook_counts.mobility_relink,
        intrinsic_limit_count: hook_counts.intrinsic_limit,
    }
}

// long-block-exception: the aggregate summary intentionally stays in one grouped
// reduction so the output schema remains easy to audit against the run schema.
pub(super) fn aggregate_runs(runs: &[ExperimentRunSummary]) -> Vec<ExperimentAggregateSummary> {
    let mut grouped: BTreeMap<
        (String, String, Option<String>, String),
        Vec<&ExperimentRunSummary>,
    > = BTreeMap::new();
    for run in runs {
        grouped
            .entry((
                run.engine_family.clone(),
                run.family_id.clone(),
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
            let route_present_permille_mean =
                average_u32(group.iter().map(|run| run.route_present_permille));
            let first_materialization_round_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.first_materialization_round_mean),
            );
            let first_loss_round_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.first_loss_round_mean));
            let recovery_round_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.recovery_round_mean));
            let route_churn_count_mean = average_u32(group.iter().map(|run| run.route_churn_count));
            let engine_handoff_count_mean =
                average_u32(group.iter().map(|run| run.engine_handoff_count));
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
            let field_selected_result_rounds_mean =
                average_u32(group.iter().map(|run| run.field_selected_result_rounds));
            let olsrv2_selected_rounds_mean =
                average_u32(group.iter().map(|run| run.olsrv2_selected_rounds));
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
            let persistent_degraded_count_mean =
                average_u32(group.iter().map(|run| run.persistent_degraded_count));
            let acceptable = activation_success_permille_mean >= 900
                && route_present_permille_mean >= 500
                && lost_reachability_count_mean == 0
                && maintenance_failure_count_mean == 0;

            ExperimentAggregateSummary {
                suite_id: first.suite_id.clone(),
                family_id: first.family_id.clone(),
                engine_family: first.engine_family.clone(),
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
                route_present_permille_mean,
                first_materialization_round_mean,
                first_loss_round_mean,
                recovery_round_mean,
                route_churn_count_mean,
                engine_handoff_count_mean,
                dominant_engine: engine_mode,
                olsrv2_selected_rounds_mean,
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
                persistent_degraded_count_mean,
                acceptable,
            }
        })
        .collect()
}

pub(super) fn summarize_breakdowns(
    aggregates: &[ExperimentAggregateSummary],
) -> Vec<ExperimentBreakdownSummary> {
    let mut grouped: BTreeMap<(String, String), Vec<&ExperimentAggregateSummary>> = BTreeMap::new();
    for aggregate in aggregates {
        grouped
            .entry((aggregate.engine_family.clone(), aggregate.config_id.clone()))
            .or_default()
            .push(aggregate);
    }

    grouped
        .into_iter()
        .map(|((engine_family, config_id), mut group)| {
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
                } else if aggregate.route_present_permille_mean < 500 {
                    "route-presence".to_string()
                } else if aggregate.lost_reachability_count_mean > 0 {
                    "lost-reachability".to_string()
                } else if aggregate.maintenance_failure_count_mean > 0 {
                    "maintenance-failure".to_string()
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
                config_id,
                max_sustained_stress_score,
                first_failed_family_id: first_failed.map(|aggregate| aggregate.family_id.clone()),
                first_failed_stress_score: first_failed.map(|aggregate| aggregate.stress_score),
                breakdown_reason,
            }
        })
        .collect()
}
