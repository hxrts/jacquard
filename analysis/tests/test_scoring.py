from __future__ import annotations

import unittest

import polars as pl

from analysis.scoring import (
    field_profile_recommendation_table,
    field_diffusion_regime_calibration_table,
    field_vs_best_diffusion_alternative_table,
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


if __name__ == "__main__":
    unittest.main()
