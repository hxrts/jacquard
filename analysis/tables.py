"""Format recommendation, transition, boundary, profile, baseline, comparison, and head-to-head DataFrames into PDF table row lists."""

from __future__ import annotations

import polars as pl

from .plots import break_tick_label


def recommendation_table_rows(
    recommendations: pl.DataFrame, limit_per_engine: int
) -> list[list[str]]:
    rows: list[list[str]] = []
    for engine_family in [
        "batman-bellman",
        "batman-classic",
        "babel",
        "olsrv2",
        "pathway",
        "field",
        "comparison",
    ]:
        family = recommendations.filter(pl.col("engine_family") == engine_family).head(
            limit_per_engine
        )
        for row in family.iter_rows(named=True):
            rows.append(
                [
                    engine_family,
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
                row["first_failed_family_id"] or "-",
                str(row["first_failed_stress_score"])
                if row["first_failed_stress_score"] is not None
                else "-",
                row["breakdown_reason"] or "-",
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
            .sort("route_present_permille_mean", descending=True)
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
                str(row["route_present_permille_mean"]),
                str(row["stress_score"]),
            ]
        )
    return rows


def head_to_head_table_rows(head_to_head_summary: pl.DataFrame) -> list[list[str]]:
    rows: list[list[str]] = []
    for family_id in head_to_head_summary["family_id"].unique().sort().to_list():
        family_rows = head_to_head_summary.filter(pl.col("family_id") == family_id).sort(
            ["route_present_permille_mean", "activation_success_permille_mean"],
            descending=[True, True],
        )
        for index, row in enumerate(family_rows.iter_rows(named=True)):
            engine_set = row["comparison_engine_set"] or "none"
            rows.append(
                [
                    break_tick_label(family_id).replace("\n", " / ") if index == 0 else "",
                    f"`{engine_set}`",
                    str(row["activation_success_permille_mean"]),
                    str(row["route_present_permille_mean"]),
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
