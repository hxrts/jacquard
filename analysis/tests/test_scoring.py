from __future__ import annotations

import unittest

import polars as pl

from analysis.constants import ROUTE_VISIBLE_ENGINE_SET_ORDER
from analysis.scoring import (
    benchmark_profile_audit_table,
    diffusion_baseline_audit_table,
    diffusion_family_weight_sensitivity_table,
    field_diffusion_regime_calibration_table,
    field_profile_recommendation_table,
    field_vs_best_diffusion_alternative_table,
    head_to_head_summary_table,
    recommendation_table,
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


class FieldRoutingRecommendationTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
