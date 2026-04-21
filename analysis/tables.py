"""Format recommendation, transition, boundary, profile, baseline, comparison, and head-to-head DataFrames into PDF table row lists."""

from __future__ import annotations

import polars as pl

from .plots import break_tick_label, engine_display_label


def recommendation_table_rows(
    recommendations: pl.DataFrame, limit_per_engine: int
) -> list[list[str]]:
    rows: list[list[str]] = []
    for engine_family in [
        "batman-classic",
        "batman-bellman",
        "babel",
        "olsrv2",
        "pathway",
        "scatter",
        "comparison",
        "field",
    ]:
        family = recommendations.filter(pl.col("engine_family") == engine_family).head(
            limit_per_engine
        )
        for row in family.iter_rows(named=True):
            rows.append(
                [
                    f"`{row['config_id']}`",
                    f"{row['mean_score']:.1f}",
                    f"{row['activation_success_mean']:.1f}",
                    f"{row['route_present_mean']:.1f}",
                    str(row["max_sustained_stress_score"]),
                ]
            )
    return rows


def transition_table_rows(transition_metrics: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in transition_metrics.iter_rows(named=True):
        rows.append(
            [
                row["engine_family"],
                f"`{row['config_id']}`",
                f"{row['route_present_mean']:.1f}",
                f"{row['route_present_stddev']:.1f}",
                str(int(row["first_materialization_median"]))
                if row["first_materialization_median"] is not None
                else "-",
                str(int(row["first_loss_median"]))
                if row["first_loss_median"] is not None
                else "-",
                str(int(row["recovery_median"])) if row["recovery_median"] is not None else "-",
                f"{row['route_churn_mean']:.1f}",
            ]
        )
    return rows


def boundary_table_rows(boundary_summary: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in boundary_summary.iter_rows(named=True):
        rows.append(
            [
                row["engine_family"],
                f"`{row['config_id']}`",
                str(row["max_sustained_stress_score"]),
                row["first_failed_family_id"] or "not observed",
                str(row["first_failed_stress_score"])
                if row["first_failed_stress_score"] is not None
                else "not observed",
                row["breakdown_reason"] or "not observed",
            ]
        )
    return rows


def profile_table_rows(profile_recommendations: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in profile_recommendations.iter_rows(named=True):
        rows.append(
            [
                row["engine_family"],
                row["profile_id"],
                f"`{row['config_id']}`",
                f"{row['mean_score']:.1f}",
                f"{row['activation_success_mean']:.1f}",
                f"{row['route_present_mean']:.1f}",
                str(row["max_sustained_stress_score"]),
            ]
        )
    return rows


def field_profile_table_rows(field_profile_recommendations: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in field_profile_recommendations.iter_rows(named=True):
        rows.append(
            [
                row["profile_id"],
                f"`{row['config_id']}`",
                f"{row['mean_score']:.1f}",
                f"{row['route_present_mean']:.1f}",
                f"{row['field_continuation_shift_mean']:.1f}",
                f"{row['field_service_retention_carry_forward_mean']:.1f}",
                f"{row['field_corridor_narrow_mean']:.1f}",
                f"{row['field_degraded_steady_round_mean']:.1f}",
            ]
        )
    return rows


def benchmark_profile_audit_table_rows(benchmark_profile_audit: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in benchmark_profile_audit.iter_rows(named=True):
        rows.append(
            [
                row["engine_set"],
                row["representative_surface_kind"],
                f"`{row['representative_config_id']}`",
                row["calibrated_profile_id"] or "-",
                f"`{row['calibrated_config_id']}`" if row["calibrated_config_id"] else "-",
                "yes" if row["configs_match"] else "no",
            ]
        )
    return rows


def field_routing_regime_table_rows(field_routing_regime_calibration: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in field_routing_regime_calibration.iter_rows(named=True):
        rows.append(
            [
                row["field_regime"],
                row["success_criteria"],
                f"`{row['config_id']}`",
                f"{row['route_present_mean']:.1f}",
                f"{row['transition_health']:.1f}",
                f"{row['continuation_shift_mean']:.1f}",
                f"{row['service_carry_mean']:.1f}",
                str(row["stress_envelope"]),
            ]
        )
    return rows


def baseline_table_rows(baseline_comparison: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in baseline_comparison.iter_rows(named=True):
        rows.append(
            [
                row["engine_family"],
                f"`{row['current_config_id']}`",
                f"`{row['baseline_config_id']}`" if row["baseline_config_id"] else "-",
                f"{row['score_delta']:.1f}",
                f"{row['route_delta']:.1f}",
                f"{row['activation_delta']:.1f}",
            ]
        )
    return rows


def comparison_table_rows(comparison_summary: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for family_id in comparison_summary["family_id"].unique().sort().to_list():
        family = (
            comparison_summary.filter(pl.col("family_id") == family_id)
            .sort("route_present_active_window_permille_mean", descending=True)
            .head(1)
        )
        if family.is_empty():
            continue
        row = family.iter_rows(named=True).__next__()
        rows.append(
            [
                break_tick_label(family_id).replace("\n", " / "),
                str(row["dominant_engine"] or "none"),
                str(row["activation_success_permille_mean"]),
                str(row["route_present_active_window_permille_mean"]),
                str(row["stress_score"]),
            ]
        )
    return rows


def comparison_engine_round_breakdown_table_rows(
    comparison_engine_round_breakdown: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in comparison_engine_round_breakdown.iter_rows(named=True):
        rows.append(
            [
                break_tick_label(row["family_id"]).replace("\n", " / "),
                str(row["dominant_engine"] or "none"),
                str(row["route_present_active_window_permille_mean"]),
                str(row["engine_handoff_count_mean"]),
                str(row["batman_classic_selected_rounds_mean"]),
                str(row["batman_bellman_selected_rounds_mean"]),
                str(row["babel_selected_rounds_mean"]),
                str(row["olsrv2_selected_rounds_mean"]),
                str(row["pathway_selected_rounds_mean"]),
                str(row["scatter_selected_rounds_mean"]),
                str(row["field_selected_rounds_mean"]),
            ]
        )
    return rows


def comparison_config_sensitivity_table_rows(
    comparison_config_sensitivity: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    if comparison_config_sensitivity.is_empty():
        return rows
    for row in comparison_config_sensitivity.sort(
        ["engine_family", "sensitivity_class", "family_id"]
    ).iter_rows(named=True):
        rows.append(
            [
                row["engine_family"],
                break_tick_label(row["family_id"]).replace("\n", " / "),
                row["sensitivity_class"],
                str(row["config_count"]),
                str(row["topline_signature_count"]),
                str(row["selection_signature_count"]),
            ]
        )
    return rows


def head_to_head_table_rows(head_to_head_summary: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    if head_to_head_summary.is_empty() or "family_id" not in head_to_head_summary.columns:
        return rows
    for family_id in head_to_head_summary["family_id"].unique().sort().to_list():
        family_rows = head_to_head_summary.filter(pl.col("family_id") == family_id).sort(
            [
                "route_present_active_window_permille_mean",
                "activation_success_permille_mean",
            ],
            descending=[True, True],
        )
        for index, row in enumerate(family_rows.iter_rows(named=True)):
            engine_set = row["comparison_engine_set"] or "none"
            rows.append(
                [
                    break_tick_label(family_id).replace("\n", " / ") if index == 0 else "",
                    f"`{engine_set}`",
                    str(row["activation_success_permille_mean"]),
                    str(row["route_present_active_window_permille_mean"]),
                    str(row["dominant_engine"] or "none"),
                    str(row["stress_score"]),
                ]
            )
    return rows


def diffusion_engine_summary_table_rows(diffusion_engine_summary: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in diffusion_engine_summary.iter_rows(named=True):
        rows.append(
            [
                break_tick_label(row["family_id"]).replace("\n", " / "),
                f"`{row['config_id']}`",
                str(row["delivery_probability_permille_mean"]),
                str(row["coverage_permille_mean"]),
                str(row["delivery_latency_rounds_mean"])
                if row["delivery_latency_rounds_mean"] is not None
                else "-",
                str(row["bounded_state_mode"]),
                str(row["stress_score"]),
            ]
        )
    return rows


def diffusion_baseline_audit_table_rows(diffusion_baseline_audit: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in diffusion_baseline_audit.iter_rows(named=True):
        rows.append(
            [
                f"`{row['config_id']}`",
                str(row["replication_budget"]),
                str(row["ttl_rounds"]),
                str(row["forward_probability_permille"]),
                str(row["bridge_bias_permille"]),
                f"{row['delivery_probability_mean']:.1f}",
                f"{row['coverage_mean']:.1f}",
                f"{row['cluster_coverage_mean']:.1f}",
                str(row["bounded_state_mode"]),
            ]
        )
    return rows


def diffusion_weight_sensitivity_table_rows(
    diffusion_weight_sensitivity: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in diffusion_weight_sensitivity.iter_rows(named=True):
        rows.append(
            [
                break_tick_label(row["family_id"]).replace("\n", " / "),
                f"`{row['balanced_winner_config_id']}`",
                f"`{row['delivery_heavy_winner_config_id']}`",
                f"`{row['boundedness_heavy_winner_config_id']}`",
                "yes" if row["winner_stable"] else "no",
            ]
        )
    return rows


def diffusion_regime_engine_summary_table_rows(
    diffusion_regime_engine_summary: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in diffusion_regime_engine_summary.iter_rows(named=True):
        rows.append(
            [
                row["diffusion_regime"],
                f"`{row['config_id']}`",
                f"{row['delivery_probability_mean']:.1f}",
                f"{row['coverage_mean']:.1f}",
                f"{row['cluster_coverage_mean']:.1f}",
                f"{row['total_transmissions_mean']:.1f}",
                str(row["bounded_state_mode"]),
                f"{row['regime_score']:.1f}",
            ]
        )
    return rows


def field_vs_best_diffusion_alternative_table_rows(
    field_vs_best_diffusion_alternative: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in field_vs_best_diffusion_alternative.iter_rows(named=True):
        rows.append(
            [
                row["field_regime"],
                f"`{row['best_attempt_config_id']}`",
                "yes" if row["acceptable_candidate"] else "no",
                str(row["field_candidate_bounded_state_mode"]),
                f"`{row['alternative_config_id']}`",
                str(row["alternative_bounded_state_mode"]),
                f"{row['delivery_delta']:.1f}",
                f"{row['coverage_delta']:.1f}",
                f"{row['cluster_coverage_delta']:.1f}",
                f"{row['tx_delta']:.1f}",
                f"{row['regime_score_delta']:.1f}",
            ]
        )
    return rows


def diffusion_engine_comparison_table_rows(diffusion_engine_comparison: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    current_family = None
    for row in diffusion_engine_comparison.iter_rows(named=True):
        family = row["family_id"]
        rows.append(
            [
                break_tick_label(family).replace("\n", " / ") if family != current_family else "",
                f"`{row['config_id']}`",
                str(row["delivery_probability_permille_mean"]),
                str(row["coverage_permille_mean"]),
                str(row["total_transmissions_mean"]),
                str(row["estimated_reproduction_permille_mean"]),
                str(row["bounded_state_mode"]),
            ]
        )
        current_family = family
    return rows


def diffusion_boundary_table_rows(diffusion_boundary_summary: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in diffusion_boundary_summary.iter_rows(named=True):
        rows.append(
            [
                f"`{row['config_id']}`",
                str(row["viable_family_count"]),
                row["first_collapse_family_id"] or "-",
                str(row["first_collapse_stress_score"])
                if row["first_collapse_stress_score"] is not None
                else "-",
                row["first_explosive_family_id"] or "-",
                str(row["first_explosive_stress_score"])
                if row["first_explosive_stress_score"] is not None
                else "-",
            ]
        )
    return rows


def field_diffusion_regime_table_rows(
    field_diffusion_regime_calibration: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in field_diffusion_regime_calibration.iter_rows(named=True):
        transition = "-"
        if row["field_regime"] == "scarcity" and row["first_scarcity_transition_round_mean"] is not None:
            transition = f"scarcity@{int(row['first_scarcity_transition_round_mean'])}"
        elif row["field_regime"] == "congestion" and row["first_congestion_transition_round_mean"] is not None:
            transition = f"congestion@{int(row['first_congestion_transition_round_mean'])}"
        elif row["field_posture_transition_count_mean"] is not None:
            transition = f"{row['field_posture_transition_count_mean']:.1f} shifts"
        configuration = f"`{row['config_id']}`"
        if not row["acceptable_candidate"]:
            configuration = f"no acceptable (`{row['best_attempt_config_id']}`)"
        rows.append(
            [
                row["field_regime"],
                row["success_criteria"],
                configuration,
                str(row["field_posture_mode"] or "none"),
                str(row["bounded_state_mode"]),
                transition,
                f"{row['delivery_probability_mean']:.1f}",
                f"{row['total_transmissions_mean']:.1f}",
                f"{row['regime_fit_score']:.1f}",
            ]
        )
    return rows


def _format_table_number(value: float | None) -> str:
    if value is None:
        return "-"
    return f"{value:.0f}"


def large_population_route_summary_table_rows(
    large_population_route_summary: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in large_population_route_summary.iter_rows(named=True):
        rows.append(
            [
                row["topology_label"],
                engine_display_label(row["comparison_engine_set"]),
                _format_table_number(row["small_route_present"]),
                _format_table_number(row["moderate_route_present"]),
                _format_table_number(row["high_route_present"]),
                _format_table_number(row["small_to_high_route_delta"]),
                _format_table_number(row["high_first_loss_round"]),
            ]
        )
    return rows


def large_population_diffusion_transition_table_rows(
    large_population_diffusion_transitions: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in large_population_diffusion_transitions.iter_rows(named=True):
        rows.append(
            [
                row["question_label"],
                row["size_band"].capitalize(),
                engine_display_label(row["collapse_config_id"]) if row["collapse_config_id"] else "-",
                engine_display_label(row["viable_config_id"]) if row["viable_config_id"] else "-",
                engine_display_label(row["explosive_config_id"]) if row["explosive_config_id"] else "-",
            ]
        )
    return rows


def routing_fitness_crossover_table_rows(
    routing_fitness_crossover_summary: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in routing_fitness_crossover_summary.iter_rows(named=True):
        rows.append(
            [
                row["question_label"],
                row["band_label"].capitalize(),
                engine_display_label(row["comparison_engine_set"]),
                _format_table_number(row["route_present_total_window_permille_mean"]),
                _format_table_number(row["recovery_success_permille_mean"]),
                _format_table_number(row["first_loss_round_mean"]),
                f"{row['route_churn_count_mean']:.1f}",
                f"{row['active_route_hop_count_mean']:.1f}"
                if row["active_route_hop_count_mean"] is not None
                else "-",
            ]
        )
    return rows


def routing_fitness_multiflow_table_rows(
    routing_fitness_multiflow_summary: pl.DataFrame,
) -> list[list[str]]:
    def broker_cell(row: dict[str, object]) -> str:
        participation = row["broker_participation_permille_mean"]
        concentration = row["broker_concentration_permille_mean"]
        if participation is None or concentration is None:
            return "-"
        switches = float(row["broker_route_churn_count_mean"] or 0.0)
        return (
            f"{float(participation) / 10.0:.0f}/"
            f"{float(concentration) / 10.0:.0f}/"
            f"{switches:.1f}"
        )

    rows: list[list[str]] = []
    for row in routing_fitness_multiflow_summary.iter_rows(named=True):
        rows.append(
            [
                row["family_label"],
                engine_display_label(row["comparison_engine_set"]),
                _format_table_number(row["objective_route_presence_min_permille_mean"]),
                _format_table_number(row["objective_route_presence_max_permille_mean"]),
                _format_table_number(row["objective_route_presence_spread_mean"]),
                _format_table_number(row["objective_starvation_count_mean"]),
                broker_cell(row),
                f"{row['concurrent_route_round_count_mean']:.1f}",
                f"{row['route_churn_count_mean']:.1f}",
            ]
        )
    return rows


def routing_fitness_stale_repair_table_rows(
    routing_fitness_stale_repair_summary: pl.DataFrame,
) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in routing_fitness_stale_repair_summary.iter_rows(named=True):
        rows.append(
            [
                row["family_label"],
                engine_display_label(row["comparison_engine_set"]),
                _format_table_number(row["stale_persistence_round_mean"]),
                _format_table_number(row["recovery_success_permille_mean"]),
                _format_table_number(row["unrecovered_after_loss_count_mean"]),
                _format_table_number(row["first_loss_round_mean"]),
                f"{row['route_churn_count_mean']:.1f}",
            ]
        )
    return rows
