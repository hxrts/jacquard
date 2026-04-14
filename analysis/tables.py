"""Format recommendation, transition, boundary, profile, baseline, comparison, and head-to-head DataFrames into PDF table row lists."""

from __future__ import annotations

import polars as pl

from .plots import break_tick_label


def recommendation_table_rows(
    recommendations: pl.DataFrame, limit_per_engine: int
) -> list[list[str]]:
    rows: list[list[str]] = []
    for engine_family in ["batman-bellman", "batman-classic", "babel", "pathway", "field", "comparison"]:
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
