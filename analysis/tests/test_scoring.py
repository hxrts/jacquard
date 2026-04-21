from __future__ import annotations

import unittest

import polars as pl

from analysis.constants import ROUTE_VISIBLE_ENGINE_SET_ORDER
from analysis.plots import (
    render_pathway_budget_route_presence,
    render_routing_fitness_crossover,
    render_routing_fitness_multiflow,
    render_routing_fitness_stale_repair,
)
from analysis.sections import routing_fitness_takeaway_lines
from analysis.scoring import (
    benchmark_profile_audit_table,
    comparison_config_sensitivity_table,
    diffusion_baseline_audit_table,
    diffusion_family_weight_sensitivity_table,
    field_diffusion_regime_calibration_table,
    field_profile_recommendation_table,
    field_routing_regime_calibration_table,
    field_vs_best_diffusion_alternative_table,
    head_to_head_summary_table,
    large_population_diffusion_state_points_table,
    large_population_diffusion_transition_table,
    large_population_route_summary_table,
    recommendation_table,
    routing_fitness_crossover_summary_table,
    routing_fitness_multiflow_summary_table,
    routing_fitness_stale_repair_summary_table,
)


LOW_CHURN_CONFIG = "field-4-zero-p1-f140-n180"
BROAD_RESELECTION_CONFIG = "field-6-hop-lower-bound-p3-f170-n90"


def _field_route_visible_aggregates() -> pl.DataFrame:
    common = {
        "engine_family": "field",
        "activation_success_permille_mean": 1000.0,
        "route_present_permille_mean": 810.5555555555555,
        "stability_total_mean": 0.0,
        "stress_score": 60,
        "objective_regime": "service",
        "first_materialization_round_mean": 0.0,
        "recovery_round_mean": 0.0,
        "route_churn_count_mean": 0.0,
        "maintenance_failure_count_mean": 0.0,
        "lost_reachability_count_mean": 0.0,
        "persistent_degraded_count_mean": 0.0,
        "field_bootstrap_activation_permille_mean": 444.44444444444446,
        "field_bootstrap_hold_permille_mean": 111.11111111111111,
        "field_bootstrap_narrow_permille_mean": 0.0,
        "field_bootstrap_upgrade_permille_mean": 222.22222222222223,
        "field_bootstrap_withdraw_permille_mean": 1333.3333333333333,
        "field_degraded_steady_entry_permille_mean": 444.44444444444446,
        "field_degraded_steady_recovery_permille_mean": 0.0,
        "field_degraded_to_bootstrap_permille_mean": 111.11111111111111,
        "field_degraded_steady_round_permille_mean": 31.88888888888889,
        "field_asymmetric_shift_success_permille_mean": 0.0,
        "field_corridor_narrow_count_mean": 0.0,
        "field_continuity_band_mode": "Steady",
        "field_commitment_resolution_mode": "Pending",
        "field_last_outcome_mode": "ContinuationRetained",
        "field_last_continuity_transition_mode": "EnteredDegradedSteady",
        "field_last_promotion_decision_mode": "Promote",
        "field_last_promotion_blocker_mode": "SupportTrend",
    }
    return pl.from_dicts(
        [
            {
                **common,
                "config_id": LOW_CHURN_CONFIG,
                "field_service_retention_carry_forward_permille_mean": 7111.111111111111,
                "field_continuation_shift_count_mean": 2.7777777777777777,
            },
            {
                **common,
                "config_id": BROAD_RESELECTION_CONFIG,
                "field_service_retention_carry_forward_permille_mean": 14333.333333333334,
                "field_continuation_shift_count_mean": 10.0,
            },
        ]
    )


def _field_breakdowns() -> pl.DataFrame:
    return pl.from_dicts(
        [
            {
                "engine_family": "field",
                "config_id": LOW_CHURN_CONFIG,
                "max_sustained_stress_score": 60,
            },
            {
                "engine_family": "field",
                "config_id": BROAD_RESELECTION_CONFIG,
                "max_sustained_stress_score": 60,
            },
        ]
    )


def _scatter_runtime_aggregates() -> pl.DataFrame:
    common = {
        "engine_family": "scatter",
        "activation_success_permille_mean": 1000.0,
        "route_present_permille_mean": 900.0,
        "route_present_total_window_permille_mean": 900.0,
        "stability_total_mean": 0.0,
        "stress_score": 60,
        "objective_regime": "repairable-connected",
        "first_materialization_round_mean": 0.0,
        "recovery_round_mean": 0.0,
        "route_churn_count_mean": 0.0,
        "maintenance_failure_count_mean": 0.0,
        "lost_reachability_count_mean": 0.0,
        "persistent_degraded_count_mean": 0.0,
        "scatter_sparse_rounds_mean": 0.0,
        "scatter_dense_rounds_mean": 16.0,
        "scatter_bridging_rounds_mean": 0.0,
        "scatter_constrained_rounds_mean": 0.0,
        "scatter_replicate_rounds_mean": 0.0,
        "scatter_handoff_rounds_mean": 0.0,
        "scatter_retained_message_peak_mean": 14.0,
        "scatter_delivered_message_peak_mean": 0.0,
    }
    return pl.from_dicts(
        [
            {
                **common,
                "config_id": "scatter-balanced",
                "family_id": "scatter-low-rate-transfer-threshold",
                "scatter_constrained_rounds_mean": 16.0,
                "scatter_handoff_rounds_mean": 14.0,
            },
            {
                **common,
                "config_id": "scatter-balanced",
                "family_id": "scatter-stability-window-threshold",
                "scatter_constrained_rounds_mean": 16.0,
                "scatter_handoff_rounds_mean": 14.0,
            },
            {
                **common,
                "config_id": "scatter-balanced",
                "family_id": "scatter-conservative-constrained-threshold",
                "scatter_dense_rounds_mean": 16.0,
            },
            {
                **common,
                "config_id": "scatter-conservative",
                "family_id": "scatter-low-rate-transfer-threshold",
                "scatter_dense_rounds_mean": 0.0,
                "scatter_constrained_rounds_mean": 16.0,
            },
            {
                **common,
                "config_id": "scatter-conservative",
                "family_id": "scatter-stability-window-threshold",
                "scatter_dense_rounds_mean": 0.0,
                "scatter_constrained_rounds_mean": 16.0,
            },
            {
                **common,
                "config_id": "scatter-conservative",
                "family_id": "scatter-conservative-constrained-threshold",
                "scatter_dense_rounds_mean": 0.0,
                "scatter_constrained_rounds_mean": 16.0,
            },
            {
                **common,
                "config_id": "scatter-degraded-network",
                "family_id": "scatter-low-rate-transfer-threshold",
                "scatter_dense_rounds_mean": 0.0,
                "scatter_sparse_rounds_mean": 16.0,
                "scatter_handoff_rounds_mean": 14.0,
            },
            {
                **common,
                "config_id": "scatter-degraded-network",
                "family_id": "scatter-stability-window-threshold",
                "scatter_dense_rounds_mean": 0.0,
                "scatter_sparse_rounds_mean": 16.0,
                "scatter_handoff_rounds_mean": 14.0,
            },
            {
                **common,
                "config_id": "scatter-degraded-network",
                "family_id": "scatter-conservative-constrained-threshold",
                "scatter_dense_rounds_mean": 0.0,
                "scatter_bridging_rounds_mean": 16.0,
                "scatter_replicate_rounds_mean": 14.0,
            },
        ]
    )


def _scatter_breakdowns() -> pl.DataFrame:
    return pl.from_dicts(
        [
            {
                "engine_family": "scatter",
                "config_id": "scatter-balanced",
                "max_sustained_stress_score": 60,
            },
            {
                "engine_family": "scatter",
                "config_id": "scatter-conservative",
                "max_sustained_stress_score": 60,
            },
            {
                "engine_family": "scatter",
                "config_id": "scatter-degraded-network",
                "max_sustained_stress_score": 60,
            },
        ]
    )


def _head_to_head_aggregates() -> pl.DataFrame:
    return pl.from_dicts(
        [
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-connected-low-loss",
                "config_id": "head-to-head-batman-classic-4-2",
                "comparison_engine_set": "batman-classic",
                "dominant_engine": "batman-classic",
                "activation_success_permille_mean": 1000.0,
                "route_present_permille_mean": 900.0,
                "route_present_total_window_permille_mean": 750.0,
                "stress_score": 18,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-connected-low-loss",
                "config_id": "head-to-head-batman-bellman-1-1",
                "comparison_engine_set": "batman-bellman",
                "dominant_engine": "batman-bellman",
                "activation_success_permille_mean": 1000.0,
                "route_present_permille_mean": 900.0,
                "route_present_total_window_permille_mean": 750.0,
                "stress_score": 18,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-connected-low-loss",
                "config_id": "head-to-head-field-8-hop-lower-bound",
                "comparison_engine_set": "field",
                "dominant_engine": "field",
                "activation_success_permille_mean": 1000.0,
                "route_present_permille_mean": 900.0,
                "route_present_total_window_permille_mean": 750.0,
                "stress_score": 18,
            },
        ]
    )


def _benchmark_profile_recommendations() -> pl.DataFrame:
    return pl.from_dicts(
        [
            {
                "engine_family": "batman-classic",
                "profile_id": "conservative",
                "config_id": "batman-classic-2-1",
                "mean_score": 10.0,
                "activation_success_mean": 1000.0,
                "route_present_mean": 900.0,
                "max_sustained_stress_score": 44,
            },
            {
                "engine_family": "batman-bellman",
                "profile_id": "conservative",
                "config_id": "batman-bellman-1-1",
                "mean_score": 10.0,
                "activation_success_mean": 1000.0,
                "route_present_mean": 900.0,
                "max_sustained_stress_score": 56,
            },
            {
                "engine_family": "field",
                "profile_id": "balanced",
                "config_id": "field-8-hop-lower-bound",
                "mean_score": 10.0,
                "activation_success_mean": 1000.0,
                "route_present_mean": 900.0,
                "max_sustained_stress_score": 60,
            },
        ]
    )


def _routing_fitness_aggregates() -> pl.DataFrame:
    return pl.from_dicts(
        [
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-core-periphery-high",
                "comparison_engine_set": "pathway",
                "route_present_total_window_permille_mean": 720.0,
                "route_present_permille_mean": 790.0,
                "recovery_success_permille_mean": 780.0,
                "first_loss_round_mean": 16.0,
                "recovery_round_mean": 18.0,
                "route_churn_count_mean": 2.0,
                "active_route_hop_count_mean": 3.4,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-core-periphery-high",
                "comparison_engine_set": "pathway-batman-bellman",
                "route_present_total_window_permille_mean": 680.0,
                "route_present_permille_mean": 760.0,
                "recovery_success_permille_mean": 810.0,
                "first_loss_round_mean": 18.0,
                "recovery_round_mean": 19.0,
                "route_churn_count_mean": 2.7,
                "active_route_hop_count_mean": 3.1,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-multi-bottleneck-high",
                "comparison_engine_set": "pathway",
                "route_present_total_window_permille_mean": 640.0,
                "route_present_permille_mean": 710.0,
                "recovery_success_permille_mean": 520.0,
                "first_loss_round_mean": 9.0,
                "recovery_round_mean": 16.0,
                "route_churn_count_mean": 4.8,
                "active_route_hop_count_mean": 4.2,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-multi-bottleneck-high",
                "comparison_engine_set": "pathway-batman-bellman",
                "route_present_total_window_permille_mean": 920.0,
                "route_present_permille_mean": 950.0,
                "recovery_success_permille_mean": 910.0,
                "first_loss_round_mean": 19.0,
                "recovery_round_mean": 20.0,
                "route_churn_count_mean": 1.9,
                "active_route_hop_count_mean": 4.0,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-multi-flow-shared-corridor",
                "comparison_engine_set": "pathway",
                "route_present_total_window_permille_mean": 710.0,
                "objective_route_presence_min_permille_mean": 410.0,
                "objective_route_presence_max_permille_mean": 870.0,
                "objective_route_presence_spread_mean": 460.0,
                "objective_starvation_count_mean": 1.0,
                "concurrent_route_round_count_mean": 6.0,
                "broker_participation_permille_mean": 920.0,
                "broker_concentration_permille_mean": 810.0,
                "broker_route_churn_count_mean": 2.0,
                "route_churn_count_mean": 3.2,
                "active_route_hop_count_mean": 3.5,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-multi-flow-shared-corridor",
                "comparison_engine_set": "pathway-batman-bellman",
                "route_present_total_window_permille_mean": 900.0,
                "objective_route_presence_min_permille_mean": 820.0,
                "objective_route_presence_max_permille_mean": 930.0,
                "objective_route_presence_spread_mean": 110.0,
                "objective_starvation_count_mean": 0.0,
                "concurrent_route_round_count_mean": 8.0,
                "broker_participation_permille_mean": 870.0,
                "broker_concentration_permille_mean": 640.0,
                "broker_route_churn_count_mean": 1.0,
                "route_churn_count_mean": 1.4,
                "active_route_hop_count_mean": 3.1,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-stale-recovery-window",
                "comparison_engine_set": "pathway",
                "route_present_total_window_permille_mean": 620.0,
                "first_disruption_round_mean": 7.0,
                "first_loss_round_mean": 11.0,
                "stale_persistence_round_mean": 4.0,
                "recovery_round_mean": 17.0,
                "recovery_success_permille_mean": 600.0,
                "unrecovered_after_loss_count_mean": 1.0,
                "route_churn_count_mean": 4.1,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-stale-recovery-window",
                "comparison_engine_set": "pathway-batman-bellman",
                "route_present_total_window_permille_mean": 890.0,
                "first_disruption_round_mean": 7.0,
                "first_loss_round_mean": 9.0,
                "stale_persistence_round_mean": 2.0,
                "recovery_round_mean": 12.0,
                "recovery_success_permille_mean": 900.0,
                "unrecovered_after_loss_count_mean": 0.0,
                "route_churn_count_mean": 1.8,
            },
        ]
    )


def _diffusion_regime_aggregates() -> pl.DataFrame:
    base = {
        "family_id": "diffusion-congestion-cascade",
        "delivery_probability_permille_mean": 650.0,
        "coverage_permille_mean": 620.0,
        "cluster_coverage_permille_mean": 480.0,
        "delivery_latency_rounds_mean": 5.0,
        "total_transmissions_mean": 28.0,
        "energy_per_delivered_message_mean": 700.0,
        "storage_utilization_permille_mean": 610.0,
        "estimated_reproduction_permille_mean": 980.0,
        "corridor_persistence_permille_mean": 320.0,
        "observer_leakage_permille_mean": 40.0,
        "field_posture_transition_count_mean": 2.0,
        "field_first_scarcity_transition_round_mean": None,
        "field_first_congestion_transition_round_mean": 3.0,
        "field_protected_budget_used_mean": 2.0,
        "field_generic_budget_used_mean": 4.0,
        "field_bridge_opportunity_count_mean": 0.0,
        "field_protected_bridge_usage_count_mean": 0.0,
        "field_cluster_seed_opportunity_count_mean": 3.0,
        "field_cluster_seed_usage_count_mean": 0.0,
        "field_cluster_coverage_starvation_count_mean": 9.0,
        "field_redundant_forward_suppression_count_mean": 8.0,
        "field_same_cluster_suppression_count_mean": 4.0,
        "field_expensive_transport_suppression_count_mean": 0.0,
        "field_cluster_seeding_rounds_mean": 2.0,
        "field_duplicate_suppressed_rounds_mean": 6.0,
        "bounded_state_mode": "collapse",
    }
    return pl.from_dicts(
        [
            {
                **base,
                "config_id": "field-congestion-search-1",
                "field_posture_mode": "duplicate_suppressed",
            },
            {
                **base,
                "config_id": "field-congestion-search-2",
                "delivery_probability_permille_mean": 690.0,
                "cluster_coverage_permille_mean": 420.0,
                "total_transmissions_mean": 30.0,
                "field_posture_mode": "duplicate_suppressed",
            },
            {
                **base,
                "config_id": "batman-classic",
                "delivery_probability_permille_mean": 760.0,
                "coverage_permille_mean": 710.0,
                "cluster_coverage_permille_mean": 830.0,
                "total_transmissions_mean": 14.0,
                "energy_per_delivered_message_mean": 420.0,
                "storage_utilization_permille_mean": 290.0,
                "estimated_reproduction_permille_mean": 540.0,
                "bounded_state_mode": "viable",
                "field_posture_mode": None,
                "field_posture_transition_count_mean": 0.0,
                "field_first_congestion_transition_round_mean": None,
                "field_protected_budget_used_mean": 0.0,
                "field_generic_budget_used_mean": 0.0,
                "field_cluster_seed_opportunity_count_mean": 0.0,
                "field_cluster_seed_usage_count_mean": 0.0,
                "field_cluster_coverage_starvation_count_mean": 0.0,
                "field_redundant_forward_suppression_count_mean": 0.0,
                "field_same_cluster_suppression_count_mean": 0.0,
                "field_cluster_seeding_rounds_mean": 0.0,
                "field_duplicate_suppressed_rounds_mean": 0.0,
            },
            {
                **base,
                "config_id": "olsrv2",
                "delivery_probability_permille_mean": 720.0,
                "coverage_permille_mean": 690.0,
                "cluster_coverage_permille_mean": 760.0,
                "total_transmissions_mean": 16.0,
                "energy_per_delivered_message_mean": 460.0,
                "storage_utilization_permille_mean": 320.0,
                "estimated_reproduction_permille_mean": 590.0,
                "bounded_state_mode": "viable",
                "field_posture_mode": None,
                "field_posture_transition_count_mean": 0.0,
                "field_first_congestion_transition_round_mean": None,
                "field_protected_budget_used_mean": 0.0,
                "field_generic_budget_used_mean": 0.0,
                "field_cluster_seed_opportunity_count_mean": 0.0,
                "field_cluster_seed_usage_count_mean": 0.0,
                "field_cluster_coverage_starvation_count_mean": 0.0,
                "field_redundant_forward_suppression_count_mean": 0.0,
                "field_same_cluster_suppression_count_mean": 0.0,
                "field_cluster_seeding_rounds_mean": 0.0,
                "field_duplicate_suppressed_rounds_mean": 0.0,
            },
        ]
    )


def _large_population_route_aggregates() -> pl.DataFrame:
    return pl.from_dicts(
        [
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-connected-low-loss",
                "comparison_engine_set": "field",
                "route_present_permille_mean": 920.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": None,
                "recovery_round_mean": None,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-core-periphery-moderate",
                "comparison_engine_set": "field",
                "route_present_permille_mean": 760.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": 18.0,
                "recovery_round_mean": None,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-core-periphery-high",
                "comparison_engine_set": "field",
                "route_present_permille_mean": 640.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": 14.0,
                "recovery_round_mean": None,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-connected-low-loss",
                "comparison_engine_set": "batman-classic",
                "route_present_permille_mean": 900.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": None,
                "recovery_round_mean": None,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-core-periphery-moderate",
                "comparison_engine_set": "batman-classic",
                "route_present_permille_mean": 600.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": 12.0,
                "recovery_round_mean": None,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-core-periphery-high",
                "comparison_engine_set": "batman-classic",
                "route_present_permille_mean": 380.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": 8.0,
                "recovery_round_mean": None,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-medium-bridge-repair",
                "comparison_engine_set": "field",
                "route_present_permille_mean": 880.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": 20.0,
                "recovery_round_mean": 4.0,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-multi-bottleneck-moderate",
                "comparison_engine_set": "field",
                "route_present_permille_mean": 700.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": 16.0,
                "recovery_round_mean": None,
            },
            {
                "engine_family": "head-to-head",
                "family_id": "head-to-head-large-multi-bottleneck-high",
                "comparison_engine_set": "field",
                "route_present_permille_mean": 520.0,
                "activation_success_permille_mean": 1000.0,
                "first_loss_round_mean": 10.0,
                "recovery_round_mean": None,
            },
        ]
    )


def _large_population_diffusion_aggregates() -> pl.DataFrame:
    return pl.from_dicts(
        [
            {
                "family_id": "diffusion-large-sparse-threshold-high",
                "config_id": "transition-tight",
                "delivery_probability_permille_mean": 420.0,
                "coverage_permille_mean": 390.0,
                "cluster_coverage_permille_mean": 360.0,
                "total_transmissions_mean": 9.0,
                "estimated_reproduction_permille_mean": 280.0,
                "bounded_state_mode": "collapse",
            },
            {
                "family_id": "diffusion-large-sparse-threshold-high",
                "config_id": "transition-balanced",
                "delivery_probability_permille_mean": 760.0,
                "coverage_permille_mean": 700.0,
                "cluster_coverage_permille_mean": 660.0,
                "total_transmissions_mean": 16.0,
                "estimated_reproduction_permille_mean": 620.0,
                "bounded_state_mode": "viable",
            },
            {
                "family_id": "diffusion-large-sparse-threshold-high",
                "config_id": "transition-broad",
                "delivery_probability_permille_mean": 920.0,
                "coverage_permille_mean": 910.0,
                "cluster_coverage_permille_mean": 880.0,
                "total_transmissions_mean": 32.0,
                "estimated_reproduction_permille_mean": 1320.0,
                "bounded_state_mode": "explosive",
            },
            {
                "family_id": "diffusion-large-congestion-threshold-moderate",
                "config_id": "transition-tight",
                "delivery_probability_permille_mean": 360.0,
                "coverage_permille_mean": 340.0,
                "cluster_coverage_permille_mean": 300.0,
                "total_transmissions_mean": 8.0,
                "estimated_reproduction_permille_mean": 260.0,
                "bounded_state_mode": "collapse",
            },
            {
                "family_id": "diffusion-large-congestion-threshold-moderate",
                "config_id": "field-congestion",
                "delivery_probability_permille_mean": 710.0,
                "coverage_permille_mean": 660.0,
                "cluster_coverage_permille_mean": 640.0,
                "total_transmissions_mean": 14.0,
                "estimated_reproduction_permille_mean": 540.0,
                "bounded_state_mode": "viable",
            },
        ]
    )


class FieldRoutingRecommendationTests(unittest.TestCase):
    def test_recommendation_table_prefers_total_window_route_presence(self) -> None:
        common = _field_route_visible_aggregates().row(0, named=True)
        aggregates = pl.from_dicts(
            [
                {
                    **common,
                    "engine_family": "pathway",
                    "family_id": "pathway-budget-pressure",
                    "config_id": "pathway-a",
                    "route_present_permille_mean": 1000000.0,
                    "route_present_total_window_permille_mean": 700.0,
                },
                {
                    **common,
                    "engine_family": "pathway",
                    "family_id": "pathway-budget-pressure",
                    "config_id": "pathway-b",
                    "route_present_permille_mean": 500000.0,
                    "route_present_total_window_permille_mean": 900.0,
                },
            ]
        )
        breakdowns = pl.from_dicts(
            [
                {
                    "engine_family": "pathway",
                    "config_id": "pathway-a",
                    "max_sustained_stress_score": 40,
                },
                {
                    "engine_family": "pathway",
                    "config_id": "pathway-b",
                    "max_sustained_stress_score": 40,
                },
            ]
        )

        recommendations = recommendation_table(aggregates, breakdowns, "balanced")

        top_pathway = recommendations.filter(pl.col("engine_family") == "pathway").row(
            0, named=True
        )
        self.assertEqual(top_pathway["config_id"], "pathway-b")
        self.assertEqual(top_pathway["route_present_mean"], 900.0)

    def test_balanced_recommendation_prefers_low_churn_when_route_presence_ties(self) -> None:
        recommendations = recommendation_table(
            _field_route_visible_aggregates(), _field_breakdowns(), "balanced"
        )
        top_field = recommendations.filter(pl.col("engine_family") == "field").row(
            0, named=True
        )
        self.assertEqual(top_field["config_id"], LOW_CHURN_CONFIG)

    def test_field_profiles_keep_broad_reselection_opt_in(self) -> None:
        profile_table = field_profile_recommendation_table(
            _field_route_visible_aggregates(), _field_breakdowns()
        )
        rows = {
            row["profile_id"]: row["config_id"]
            for row in profile_table.iter_rows(named=True)
        }
        self.assertEqual(rows["field-low-churn"], LOW_CHURN_CONFIG)
        self.assertEqual(rows["field-broad-reselection"], BROAD_RESELECTION_CONFIG)

    def test_scatter_profiles_use_runtime_surface_when_route_metrics_tie(self) -> None:
        balanced = recommendation_table(
            _scatter_runtime_aggregates(), _scatter_breakdowns(), "balanced"
        )
        conservative = recommendation_table(
            _scatter_runtime_aggregates(), _scatter_breakdowns(), "conservative"
        )
        degraded = recommendation_table(
            _scatter_runtime_aggregates(), _scatter_breakdowns(), "degraded-network"
        )

        self.assertEqual(
            balanced.filter(pl.col("engine_family") == "scatter").row(0, named=True)[
                "config_id"
            ],
            "scatter-balanced",
        )
        self.assertEqual(
            conservative.filter(pl.col("engine_family") == "scatter").row(0, named=True)[
                "config_id"
            ],
            "scatter-conservative",
        )
        self.assertEqual(
            degraded.filter(pl.col("engine_family") == "scatter").row(0, named=True)[
                "config_id"
            ],
            "scatter-degraded-network",
        )

    def test_field_routing_regime_calibration_uses_stable_mode_tie_break(self) -> None:
        calibration = field_routing_regime_calibration_table(
            pl.from_dicts(
                [
                    {
                        "engine_family": "field",
                        "family_id": "field-bootstrap-upgrade-window",
                        "config_id": LOW_CHURN_CONFIG,
                        "route_present_permille_mean": 1000.0,
                        "activation_success_permille_mean": 1000.0,
                        "stress_score": 50,
                        "field_bootstrap_upgrade_permille_mean": 1000.0,
                        "field_bootstrap_withdraw_permille_mean": 0.0,
                        "field_degraded_steady_recovery_permille_mean": 0.0,
                        "field_degraded_to_bootstrap_permille_mean": 0.0,
                        "field_service_retention_carry_forward_permille_mean": 0.0,
                        "field_asymmetric_shift_success_permille_mean": 0.0,
                        "field_continuation_shift_count_mean": 1.0,
                        "field_route_bound_reconfiguration_count_mean": 0.0,
                        "route_churn_count_mean": 0.0,
                        "field_continuity_band_mode": "DegradedSteady",
                        "field_last_continuity_transition_mode": "EnteredDegradedSteady",
                    },
                    {
                        "engine_family": "field",
                        "family_id": "field-partial-observability-bridge",
                        "config_id": LOW_CHURN_CONFIG,
                        "route_present_permille_mean": 1000.0,
                        "activation_success_permille_mean": 1000.0,
                        "stress_score": 50,
                        "field_bootstrap_upgrade_permille_mean": 1000.0,
                        "field_bootstrap_withdraw_permille_mean": 0.0,
                        "field_degraded_steady_recovery_permille_mean": 0.0,
                        "field_degraded_to_bootstrap_permille_mean": 0.0,
                        "field_service_retention_carry_forward_permille_mean": 0.0,
                        "field_asymmetric_shift_success_permille_mean": 0.0,
                        "field_continuation_shift_count_mean": 1.0,
                        "field_route_bound_reconfiguration_count_mean": 0.0,
                        "route_churn_count_mean": 0.0,
                        "field_continuity_band_mode": "Bootstrap",
                        "field_last_continuity_transition_mode": "DowngradedToBootstrap",
                    },
                ]
            )
        )
        row = calibration.row(0, named=True)
        self.assertEqual(row["field_continuity_band_mode"], "Bootstrap")
        self.assertEqual(
            row["field_last_continuity_transition_mode"], "DowngradedToBootstrap"
        )

    def test_head_to_head_summary_uses_consistent_engine_order_on_ties(self) -> None:
        summary = head_to_head_summary_table(_head_to_head_aggregates())
        engine_sets = summary["comparison_engine_set"].to_list()
        self.assertEqual(
            engine_sets,
            [engine for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER if engine in engine_sets],
        )

    def test_benchmark_profile_audit_marks_fixed_and_calibrated_surfaces(self) -> None:
        audit = benchmark_profile_audit_table(
            _head_to_head_aggregates(), _benchmark_profile_recommendations()
        )
        rows = {row["engine_set"]: row for row in audit.iter_rows(named=True)}
        self.assertEqual(
            rows["batman-classic"]["representative_surface_kind"], "fixed-representative"
        )
        self.assertEqual(
            rows["batman-classic"]["calibrated_surface_kind"], "calibrated-best"
        )

    def test_benchmark_profile_audit_requires_calibrated_profile_for_single_engine_sets(
        self,
    ) -> None:
        audit = benchmark_profile_audit_table(
            _head_to_head_aggregates(), _benchmark_profile_recommendations()
        )
        rows = {row["engine_set"]: row for row in audit.iter_rows(named=True)}
        self.assertIsNotNone(rows["batman-classic"]["calibrated_profile_id"])
        self.assertIsNotNone(rows["batman-bellman"]["calibrated_profile_id"])
        self.assertIsNotNone(rows["field"]["calibrated_profile_id"])

    def test_field_diffusion_regime_calibration_fails_closed_when_all_field_candidates_collapse(
        self,
    ) -> None:
        calibration = field_diffusion_regime_calibration_table(
            _diffusion_regime_aggregates()
        )
        congestion = calibration.filter(pl.col("field_regime") == "congestion").row(
            0, named=True
        )
        self.assertFalse(congestion["acceptable_candidate"])
        self.assertEqual(congestion["config_id"], "no-acceptable-field-candidate")
        self.assertTrue(
            congestion["best_attempt_config_id"].startswith("field-congestion-search-")
        )

    def test_field_vs_best_alternative_uses_best_non_field_regime_candidate(self) -> None:
        aggregates = _diffusion_regime_aggregates()
        calibration = field_diffusion_regime_calibration_table(aggregates)
        delta = field_vs_best_diffusion_alternative_table(aggregates, calibration)
        congestion = delta.filter(pl.col("field_regime") == "congestion").row(0, named=True)
        self.assertEqual(congestion["alternative_config_id"], "batman-classic")
        self.assertLess(congestion["regime_score_delta"], 0.0)

    def test_diffusion_baseline_audit_lists_non_field_benchmarks(self) -> None:
        audit = diffusion_baseline_audit_table(_diffusion_regime_aggregates())
        self.assertEqual(audit["config_id"].to_list(), ["batman-classic", "olsrv2"])

    def test_diffusion_weight_sensitivity_marks_unstable_generic_winners(self) -> None:
        aggregates = pl.from_dicts(
            [
                {
                    "family_id": "diffusion-sensitivity-check",
                    "config_id": "bounded",
                    "replication_budget": 2,
                    "message_horizon": 20,
                    "forward_probability_permille": 320,
                    "bridge_bias_permille": 60,
                    "delivery_probability_permille_mean": 780.0,
                    "coverage_permille_mean": 760.0,
                    "cluster_coverage_permille_mean": 760.0,
                    "corridor_persistence_permille_mean": 300.0,
                    "delivery_latency_rounds_mean": 8.0,
                    "total_transmissions_mean": 10.0,
                    "energy_per_delivered_message_mean": 380.0,
                    "storage_utilization_permille_mean": 180.0,
                    "estimated_reproduction_permille_mean": 400.0,
                    "observer_leakage_permille_mean": 10.0,
                    "bounded_state_mode": "viable",
                },
                {
                    "family_id": "diffusion-sensitivity-check",
                    "config_id": "fast-but-expensive",
                    "replication_budget": 6,
                    "message_horizon": 28,
                    "forward_probability_permille": 560,
                    "bridge_bias_permille": 180,
                    "delivery_probability_permille_mean": 980.0,
                    "coverage_permille_mean": 960.0,
                    "cluster_coverage_permille_mean": 960.0,
                    "corridor_persistence_permille_mean": 220.0,
                    "delivery_latency_rounds_mean": 2.0,
                    "total_transmissions_mean": 24.0,
                    "energy_per_delivered_message_mean": 900.0,
                    "storage_utilization_permille_mean": 650.0,
                    "estimated_reproduction_permille_mean": 900.0,
                    "observer_leakage_permille_mean": 50.0,
                    "bounded_state_mode": "viable",
                },
            ]
        )
        sensitivity = diffusion_family_weight_sensitivity_table(aggregates)
        row = sensitivity.row(0, named=True)
        self.assertEqual(row["balanced_winner_config_id"], "fast-but-expensive")
        self.assertEqual(row["boundedness_heavy_winner_config_id"], "bounded")
        self.assertFalse(row["winner_stable"])


    def test_large_population_route_summary_tracks_small_to_high_drop(self) -> None:
        summary = large_population_route_summary_table(_large_population_route_aggregates())
        row = summary.filter(
            (pl.col("topology_class") == "diameter-fanout")
            & (pl.col("comparison_engine_set") == "field")
        ).row(0, named=True)
        self.assertEqual(row["small_route_present"], 920.0)
        self.assertEqual(row["moderate_route_present"], 760.0)
        self.assertEqual(row["high_route_present"], 640.0)
        self.assertEqual(row["small_to_high_route_delta"], -280.0)
        self.assertEqual(row["high_first_loss_round"], 14.0)

    def test_large_population_diffusion_tables_select_state_representatives(self) -> None:
        points = large_population_diffusion_state_points_table(
            _large_population_diffusion_aggregates()
        )
        transitions = large_population_diffusion_transition_table(
            _large_population_diffusion_aggregates()
        )
        sparse_viable = points.filter(
            (pl.col("family_id") == "diffusion-large-sparse-threshold-high")
            & (pl.col("bounded_state_mode") == "viable")
        ).row(0, named=True)
        self.assertEqual(sparse_viable["config_id"], "transition-balanced")
        sparse_row = transitions.filter(
            pl.col("family_id") == "diffusion-large-sparse-threshold-high"
        ).row(0, named=True)
        self.assertEqual(sparse_row["collapse_config_id"], "transition-tight")
        self.assertEqual(sparse_row["viable_config_id"], "transition-balanced")
        self.assertEqual(sparse_row["explosive_config_id"], "transition-broad")

    def test_routing_fitness_crossover_summary_keeps_high_band_ordering(self) -> None:
        summary = routing_fitness_crossover_summary_table(_routing_fitness_aggregates())
        high_rows = summary.filter(
            (pl.col("question") == "maintenance-benefit") & (pl.col("band_label") == "high")
        )
        self.assertEqual(
            high_rows["comparison_engine_set"].to_list(),
            ["pathway", "pathway-batman-bellman"],
        )
        best = high_rows.sort(
            ["route_present_total_window_permille_mean", "recovery_success_permille_mean"],
            descending=[True, True],
        ).row(0, named=True)
        self.assertEqual(best["comparison_engine_set"], "pathway-batman-bellman")
        self.assertEqual(best["route_present_total_window_permille_mean"], 920.0)

    def test_routing_fitness_multiflow_summary_preserves_fairness_metrics(self) -> None:
        summary = routing_fitness_multiflow_summary_table(_routing_fitness_aggregates())
        row = summary.filter(
            (pl.col("family_label") == "Shared corridor")
            & (pl.col("comparison_engine_set") == "pathway-batman-bellman")
        ).row(0, named=True)
        self.assertEqual(row["objective_route_presence_min_permille_mean"], 820.0)
        self.assertEqual(row["objective_route_presence_spread_mean"], 110.0)
        self.assertEqual(row["objective_starvation_count_mean"], 0.0)
        self.assertEqual(row["broker_participation_permille_mean"], 870.0)
        self.assertEqual(row["broker_concentration_permille_mean"], 640.0)
        self.assertEqual(row["broker_route_churn_count_mean"], 1.0)

    def test_routing_fitness_stale_summary_preserves_repair_metrics(self) -> None:
        summary = routing_fitness_stale_repair_summary_table(_routing_fitness_aggregates())
        row = summary.filter(
            (pl.col("family_label") == "Recovery window")
            & (pl.col("comparison_engine_set") == "pathway-batman-bellman")
        ).row(0, named=True)
        self.assertEqual(row["stale_persistence_round_mean"], 2.0)
        self.assertEqual(row["recovery_success_permille_mean"], 900.0)
        self.assertEqual(row["unrecovered_after_loss_count_mean"], 0.0)

    def test_routing_fitness_stale_summary_keeps_no_recovery_rows(self) -> None:
        summary = routing_fitness_stale_repair_summary_table(
            pl.from_dicts(
                [
                    {
                        "engine_family": "head-to-head",
                        "family_id": "head-to-head-stale-recovery-window",
                        "comparison_engine_set": "pathway",
                        "route_present_total_window_permille_mean": 410.0,
                        "route_present_permille_mean": 410.0,
                        "first_disruption_round_mean": 5.0,
                        "first_loss_round_mean": 8.0,
                        "stale_persistence_round_mean": 3.0,
                        "recovery_round_mean": None,
                        "recovery_success_permille_mean": 0.0,
                        "unrecovered_after_loss_count_mean": 1.0,
                        "route_churn_count_mean": 2.0,
                    }
                ]
            )
        )

        row = summary.row(0, named=True)
        self.assertEqual(row["stale_persistence_round_mean"], 3.0)
        self.assertEqual(row["recovery_round_mean"], None)
        self.assertEqual(row["unrecovered_after_loss_count_mean"], 1.0)

    def test_routing_fitness_takeaways_close_with_tested_envelope(self) -> None:
        lines = routing_fitness_takeaway_lines(
            pl.from_dicts(
                [
                    {
                        "question": "search-burden",
                        "question_label": "Search burden crossover",
                        "band_label": "high",
                        "comparison_engine_set": "pathway",
                        "route_present_total_window_permille_mean": 720.0,
                        "recovery_success_permille_mean": 780.0,
                    },
                    {
                        "question": "maintenance-benefit",
                        "question_label": "Maintenance benefit crossover",
                        "band_label": "high",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "route_present_total_window_permille_mean": 920.0,
                        "recovery_success_permille_mean": 910.0,
                    },
                ]
            ),
            pl.from_dicts(
                [
                    {
                        "family_label": "Shared corridor",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "objective_route_presence_min_permille_mean": 820.0,
                        "objective_starvation_count_mean": 0.0,
                    },
                    {
                        "family_label": "Detour choice",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "objective_route_presence_min_permille_mean": 760.0,
                        "objective_starvation_count_mean": 0.0,
                    },
                    {
                        "family_label": "Asymmetric demand",
                        "comparison_engine_set": "pathway",
                        "objective_route_presence_min_permille_mean": 340.0,
                        "objective_starvation_count_mean": 1.0,
                    },
                ]
            ),
            pl.from_dicts(
                [
                    {
                        "family_label": "Recovery window",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "route_present_total_window_permille_mean": 930.0,
                        "stale_persistence_round_mean": 2.0,
                        "recovery_success_permille_mean": 900.0,
                    },
                    {
                        "family_label": "Delayed observation",
                        "comparison_engine_set": "pathway",
                        "route_present_total_window_permille_mean": 640.0,
                        "stale_persistence_round_mean": 4.0,
                        "recovery_success_permille_mean": 600.0,
                    },
                ]
            ),
        )

        self.assertTrue(lines)
        self.assertTrue(
            any(
                "fit-for-purpose inside the tested search-plus-maintenance envelope"
                in line
                for line in lines
            )
        )

    def test_routing_fitness_takeaways_keep_tied_best_engine_labels_stable(self) -> None:
        lines = routing_fitness_takeaway_lines(
            pl.from_dicts(
                [
                    {
                        "question": "search-burden",
                        "question_label": "Search burden crossover",
                        "band_label": "high",
                        "comparison_engine_set": "field",
                        "route_present_total_window_permille_mean": 720.0,
                        "recovery_success_permille_mean": 780.0,
                    },
                    {
                        "question": "search-burden",
                        "question_label": "Search burden crossover",
                        "band_label": "high",
                        "comparison_engine_set": "pathway",
                        "route_present_total_window_permille_mean": 720.0,
                        "recovery_success_permille_mean": 780.0,
                    },
                    {
                        "question": "search-burden",
                        "question_label": "Search burden crossover",
                        "band_label": "high",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "route_present_total_window_permille_mean": 720.0,
                        "recovery_success_permille_mean": 780.0,
                    },
                    {
                        "question": "maintenance-benefit",
                        "question_label": "Maintenance benefit crossover",
                        "band_label": "high",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "route_present_total_window_permille_mean": 920.0,
                        "recovery_success_permille_mean": 910.0,
                    },
                ]
            ),
            pl.from_dicts(
                [
                    {
                        "family_label": "Shared corridor",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "objective_route_presence_min_permille_mean": 820.0,
                        "objective_starvation_count_mean": 0.0,
                    },
                    {
                        "family_label": "Detour choice",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "objective_route_presence_min_permille_mean": 760.0,
                        "objective_starvation_count_mean": 0.0,
                    },
                    {
                        "family_label": "Asymmetric demand",
                        "comparison_engine_set": "pathway",
                        "objective_route_presence_min_permille_mean": 340.0,
                        "objective_starvation_count_mean": 1.0,
                    },
                ]
            ),
            pl.from_dicts(
                [
                    {
                        "family_label": "Recovery window",
                        "comparison_engine_set": "pathway-batman-bellman",
                        "route_present_total_window_permille_mean": 930.0,
                        "stale_persistence_round_mean": 2.0,
                        "recovery_success_permille_mean": 900.0,
                    },
                    {
                        "family_label": "Delayed observation",
                        "comparison_engine_set": "pathway",
                        "route_present_total_window_permille_mean": 640.0,
                        "stale_persistence_round_mean": 4.0,
                        "recovery_success_permille_mean": 600.0,
                    },
                ]
            ),
        )

        self.assertTrue(
            any(
                "`field`, `pathway`, and `pathway-batman-bellman`" in line
                for line in lines
            )
        )

    def test_routing_fitness_renderers_keep_deterministic_family_order(self) -> None:
        crossover = render_routing_fitness_crossover(
            routing_fitness_crossover_summary_table(_routing_fitness_aggregates()),
            1200,
            600,
        ).to_dict()
        crossover_rows = crossover["datasets"][crossover["data"]["name"]]
        self.assertEqual(crossover_rows[0]["question_label"], "Maintenance benefit crossover")
        self.assertEqual(crossover_rows[0]["band_label"], "High")

        multiflow = render_routing_fitness_multiflow(
            routing_fitness_multiflow_summary_table(_routing_fitness_aggregates()),
            1200,
            600,
        ).to_dict()
        multiflow_rows = multiflow["datasets"][multiflow["data"]["name"]]
        self.assertEqual(multiflow_rows[0]["family_label"], "Shared corridor")
        self.assertEqual(multiflow_rows[0]["engine_key"], "pathway")

        stale = render_routing_fitness_stale_repair(
            routing_fitness_stale_repair_summary_table(_routing_fitness_aggregates()),
            1200,
            600,
        ).to_dict()
        stale_rows = stale["datasets"][stale["data"]["name"]]
        self.assertEqual(stale_rows[0]["family_label"], "Recovery window")
        self.assertEqual(stale_rows[0]["engine_key"], "pathway")
        self.assertIn("route=", stale_rows[0]["route_label"])

    def test_route_presence_percent_transform_uses_permille_scale(self) -> None:
        chart = render_pathway_budget_route_presence(
            pl.from_dicts(
                [
                    {
                        "engine_family": "pathway",
                        "family_id": "pathway-search-budget-pressure",
                        "pathway_query_budget": 1,
                        "pathway_heuristic_mode": "zero",
                        "route_present_permille_mean": 1000.0,
                    }
                ]
            ),
            900,
            300,
        )
        self.assertIsNotNone(chart)
        chart_dict = chart.to_dict()
        rows = next(iter(chart_dict["datasets"].values()))
        self.assertEqual(rows[0]["y_value"], 100.0)

    def test_comparison_config_sensitivity_marks_flat_and_separating_families(self) -> None:
        aggregates = pl.from_dicts(
            [
                {
                    "engine_family": "comparison",
                    "family_id": "comparison-flat",
                    "config_id": "comparison-a",
                    "activation_success_permille_mean": 1000,
                    "route_present_total_window_permille_mean": 900,
                    "first_materialization_round_mean": 2,
                    "first_loss_round_mean": None,
                    "recovery_success_permille_mean": 0,
                    "route_churn_count_mean": 0,
                    "dominant_engine": "tie",
                },
                {
                    "engine_family": "comparison",
                    "family_id": "comparison-flat",
                    "config_id": "comparison-b",
                    "activation_success_permille_mean": 1000,
                    "route_present_total_window_permille_mean": 900,
                    "first_materialization_round_mean": 2,
                    "first_loss_round_mean": None,
                    "recovery_success_permille_mean": 0,
                    "route_churn_count_mean": 0,
                    "dominant_engine": "tie",
                },
                {
                    "engine_family": "comparison",
                    "family_id": "comparison-separating",
                    "config_id": "comparison-a",
                    "activation_success_permille_mean": 1000,
                    "route_present_total_window_permille_mean": 900,
                    "first_materialization_round_mean": 2,
                    "first_loss_round_mean": None,
                    "recovery_success_permille_mean": 0,
                    "route_churn_count_mean": 0,
                    "dominant_engine": "tie",
                },
                {
                    "engine_family": "comparison",
                    "family_id": "comparison-separating",
                    "config_id": "comparison-b",
                    "activation_success_permille_mean": 1000,
                    "route_present_total_window_permille_mean": 760,
                    "first_materialization_round_mean": 4,
                    "first_loss_round_mean": 8,
                    "recovery_success_permille_mean": 0,
                    "route_churn_count_mean": 1,
                    "dominant_engine": "pathway",
                },
                {
                    "engine_family": "comparison",
                    "family_id": "comparison-selection-only",
                    "config_id": "comparison-a",
                    "activation_success_permille_mean": 1000,
                    "route_present_total_window_permille_mean": 900,
                    "first_materialization_round_mean": 2,
                    "first_loss_round_mean": None,
                    "recovery_success_permille_mean": 0,
                    "route_churn_count_mean": 0,
                    "dominant_engine": "batman-bellman",
                },
                {
                    "engine_family": "comparison",
                    "family_id": "comparison-selection-only",
                    "config_id": "comparison-b",
                    "activation_success_permille_mean": 1000,
                    "route_present_total_window_permille_mean": 900,
                    "first_materialization_round_mean": 2,
                    "first_loss_round_mean": None,
                    "recovery_success_permille_mean": 0,
                    "route_churn_count_mean": 0,
                    "dominant_engine": "pathway",
                },
            ],
            infer_schema_length=None,
        )

        table = comparison_config_sensitivity_table(aggregates)
        flat = table.filter(pl.col("family_id") == "comparison-flat").row(0, named=True)
        separating = table.filter(pl.col("family_id") == "comparison-separating").row(
            0, named=True
        )
        selection_only = table.filter(
            pl.col("family_id") == "comparison-selection-only"
        ).row(0, named=True)
        self.assertEqual(flat["config_count"], 2)
        self.assertTrue(flat["topline_flat"])
        self.assertTrue(flat["selection_flat"])
        self.assertEqual(flat["sensitivity_class"], "flat-control")
        self.assertEqual(separating["topline_signature_count"], 2)
        self.assertFalse(separating["topline_flat"])
        self.assertFalse(separating["selection_flat"])
        self.assertEqual(separating["sensitivity_class"], "topline-and-selection")
        self.assertTrue(selection_only["topline_flat"])
        self.assertFalse(selection_only["selection_flat"])
        self.assertEqual(selection_only["sensitivity_class"], "selection-only")


if __name__ == "__main__":
    unittest.main()
