"""Score expressions, recommendation tables, transition and boundary metrics, baseline comparison, and recommendations text output."""

from __future__ import annotations

import json
from pathlib import Path

import polars as pl

from .constants import RECOMMENDATION_PROFILES


def score_expression(profile_id: str) -> pl.Expr:
    profile = RECOMMENDATION_PROFILES[profile_id]
    return (
        pl.col("activation_success_permille_mean") * profile["activation_weight"]
        + pl.col("route_present_permille_mean") * profile["route_weight"]
        + (pl.col("stability_total_mean") * profile["stability_weight"])
        + pl.col("max_sustained_stress_score").fill_null(0) * profile["stress_weight"]
        + pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_bootstrap_upgrade_permille_mean") * 0.35)
        .otherwise(0)
        - pl.col("first_materialization_round_mean").fill_null(0)
        * profile["materialization_weight"]
        - pl.col("recovery_round_mean").fill_null(0) * profile["recovery_weight"]
        - pl.col("route_churn_count_mean") * profile["churn_penalty"]
        - pl.col("maintenance_failure_count_mean") * profile["maintenance_penalty"]
        - pl.col("lost_reachability_count_mean") * profile["reachability_penalty"]
        - pl.col("persistent_degraded_count_mean") * profile["degraded_penalty"]
        - pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_bootstrap_hold_permille_mean") * 0.1)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_bootstrap_narrow_permille_mean") * 0.05)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_bootstrap_withdraw_permille_mean") * 0.15)
        .otherwise(0)
    )


def recommendation_table(
    aggregates: pl.DataFrame, breakdowns: pl.DataFrame, profile_id: str = "balanced"
) -> pl.DataFrame:
    joined = aggregates.join(
        breakdowns.select(["engine_family", "config_id", "max_sustained_stress_score"]),
        on=["engine_family", "config_id"],
        how="left",
    )
    filtered = joined
    if profile_id == "degraded-network":
        filtered = filtered.filter(pl.col("stress_score") >= 40)
    elif profile_id == "service-heavy":
        filtered = filtered.filter(
            pl.col("objective_regime").is_in(["service", "explicit-path"])
        )
    filtered = filtered.with_columns(score_expression(profile_id).alias("score"))
    return (
        filtered.group_by("engine_family", "config_id")
        .agg(
            pl.col("score").mean().alias("mean_score"),
            pl.col("activation_success_permille_mean")
            .mean()
            .alias("activation_success_mean"),
            pl.col("route_present_permille_mean").mean().alias("route_present_mean"),
            pl.col("field_bootstrap_activation_permille_mean")
            .mean()
            .alias("field_bootstrap_activation_mean"),
            pl.col("field_bootstrap_hold_permille_mean")
            .mean()
            .alias("field_bootstrap_hold_mean"),
            pl.col("field_bootstrap_narrow_permille_mean")
            .mean()
            .alias("field_bootstrap_narrow_mean"),
            pl.col("field_bootstrap_upgrade_permille_mean")
            .mean()
            .alias("field_bootstrap_upgrade_mean"),
            pl.col("field_bootstrap_withdraw_permille_mean")
            .mean()
            .alias("field_bootstrap_withdraw_mean"),
            pl.col("field_last_promotion_decision_mode")
            .drop_nulls()
            .mode()
            .first()
            .alias("field_last_promotion_decision_mode"),
            pl.col("field_last_promotion_blocker_mode")
            .drop_nulls()
            .mode()
            .first()
            .alias("field_last_promotion_blocker_mode"),
            pl.col("max_sustained_stress_score")
            .max()
            .alias("max_sustained_stress_score"),
            pl.col("maintenance_failure_count_mean")
            .mean()
            .alias("maintenance_failure_mean"),
            pl.col("lost_reachability_count_mean")
            .mean()
            .alias("lost_reachability_mean"),
        )
        .filter(
            (pl.col("activation_success_mean") > 0) | (pl.col("route_present_mean") > 0)
        )
        .sort(["engine_family", "mean_score"], descending=[False, True])
    )


def profile_recommendation_table(
    aggregates: pl.DataFrame, breakdowns: pl.DataFrame
) -> pl.DataFrame:
    frames: list[pl.DataFrame] = []
    for engine_family, profiles in {
        "batman": ["conservative", "aggressive", "degraded-network"],
        "pathway": ["balanced", "service-heavy", "degraded-network"],
        "field": ["balanced", "conservative", "degraded-network"],
    }.items():
        for profile_id in profiles:
            table = recommendation_table(aggregates, breakdowns, profile_id).filter(
                pl.col("engine_family") == engine_family
            )
            if table.is_empty():
                continue
            frames.append(table.head(1).with_columns(pl.lit(profile_id).alias("profile_id")))
    if not frames:
        return pl.DataFrame()
    return pl.concat(frames).select(
        "engine_family",
        "profile_id",
        "config_id",
        "mean_score",
        "activation_success_mean",
        "route_present_mean",
        "max_sustained_stress_score",
    )


def leading_recommendation_configs(
    recommendations: pl.DataFrame, limit_per_engine: int = 2
) -> pl.DataFrame:
    frames: list[pl.DataFrame] = []
    for engine_family in ["batman", "pathway", "field", "comparison"]:
        family = recommendations.filter(pl.col("engine_family") == engine_family).head(
            limit_per_engine
        )
        if not family.is_empty():
            frames.append(family.select("engine_family", "config_id"))
    return pl.concat(frames) if frames else pl.DataFrame()


def transition_metrics_table(
    runs: pl.DataFrame, recommendations: pl.DataFrame
) -> pl.DataFrame:
    top_configs = leading_recommendation_configs(recommendations, 2)
    return (
        runs.join(top_configs, on=["engine_family", "config_id"], how="inner")
        .group_by("engine_family", "config_id")
        .agg(
            pl.col("route_present_permille").mean().alias("route_present_mean"),
            pl.col("route_present_permille")
            .std()
            .fill_null(0)
            .round(1)
            .alias("route_present_stddev"),
            pl.col("activation_success_permille")
            .mean()
            .alias("activation_success_mean"),
            pl.col("first_materialization_round_mean")
            .median()
            .alias("first_materialization_median"),
            pl.col("first_loss_round_mean")
            .drop_nulls()
            .median()
            .alias("first_loss_median"),
            pl.col("recovery_round_mean")
            .drop_nulls()
            .median()
            .alias("recovery_median"),
            pl.col("route_churn_count").mean().alias("route_churn_mean"),
            pl.col("engine_handoff_count").mean().alias("engine_handoff_mean"),
        )
        .sort(["engine_family", "route_present_mean"], descending=[False, True])
    )


def boundary_summary_table(
    recommendations: pl.DataFrame, breakdowns: pl.DataFrame
) -> pl.DataFrame:
    top_configs = leading_recommendation_configs(recommendations, 2)
    return (
        breakdowns.join(top_configs, on=["engine_family", "config_id"], how="inner")
        .select(
            "engine_family",
            "config_id",
            "max_sustained_stress_score",
            "first_failed_family_id",
            "first_failed_stress_score",
            "breakdown_reason",
        )
        .sort(["engine_family", "max_sustained_stress_score"], descending=[False, True])
    )


def top_recommendation_rows(
    recommendations: pl.DataFrame, engine_family: str, limit: int = 3
) -> list[dict]:
    family = recommendations.filter(pl.col("engine_family") == engine_family).head(limit)
    if family.is_empty():
        return []
    return list(family.iter_rows(named=True))


def top_recommendation_line(recommendations: pl.DataFrame, engine_family: str) -> str:
    rows = top_recommendation_rows(recommendations, engine_family, 1)
    if not rows:
        return f"{engine_family}: no route-visible recommendation available in this artifact set"
    row = rows[0]
    return (
        f"{engine_family}: `{row['config_id']}` "
        f"(score={row['mean_score']:.1f}, "
        f"activation={row['activation_success_mean']:.1f} permille, "
        f"route_presence={row['route_present_mean']:.1f} permille, "
        f"max_stress={row['max_sustained_stress_score']})"
    )


def field_bootstrap_summary_line(recommendations: pl.DataFrame) -> str | None:
    row = top_recommendation_row(recommendations, "field")
    if row is None:
        return None
    return (
        "Field bootstrap front-page metrics: "
        f"activation={row['field_bootstrap_activation_mean']:.1f} permille, "
        f"upgrade={row['field_bootstrap_upgrade_mean']:.1f} permille, "
        f"withdrawal={row['field_bootstrap_withdraw_mean']:.1f} permille."
    )


def top_recommendation_row(recommendations: pl.DataFrame, engine_family: str) -> dict | None:
    rows = top_recommendation_rows(recommendations, engine_family, 1)
    return rows[0] if rows else None


def previous_artifact_dir(artifact_dir: Path) -> Path | None:
    if not artifact_dir.parent.exists():
        return None
    candidates = sorted(
        path
        for path in artifact_dir.parent.iterdir()
        if path.is_dir()
        and path.name < artifact_dir.name
        and (path / "report" / "recommendations.csv").exists()
    )
    return candidates[-1] if candidates else None


def baseline_comparison_table(
    artifact_dir: Path, recommendations: pl.DataFrame
) -> tuple[pl.DataFrame, Path | None]:
    baseline_dir = previous_artifact_dir(artifact_dir)
    if baseline_dir is None:
        return pl.DataFrame(), None
    baseline = pl.read_csv(baseline_dir / "report" / "recommendations.csv")
    current_frames = []
    prior_frames = []
    for engine_family in ["batman", "pathway", "field", "comparison"]:
        current_family = recommendations.filter(pl.col("engine_family") == engine_family).head(1)
        if not current_family.is_empty():
            current_frames.append(current_family)
        prior_family = baseline.filter(pl.col("engine_family") == engine_family).head(1)
        if not prior_family.is_empty():
            prior_frames.append(prior_family)
    if not current_frames:
        return pl.DataFrame(), baseline_dir
    current = pl.concat(current_frames).rename(
        {
            "config_id": "current_config_id",
            "mean_score": "current_mean_score",
            "activation_success_mean": "current_activation_success_mean",
            "route_present_mean": "current_route_present_mean",
            "max_sustained_stress_score": "current_max_sustained_stress_score",
        }
    )
    if not prior_frames:
        return pl.DataFrame(), baseline_dir
    prior = pl.concat(prior_frames).rename(
        {
            "config_id": "baseline_config_id",
            "mean_score": "baseline_mean_score",
            "activation_success_mean": "baseline_activation_success_mean",
            "route_present_mean": "baseline_route_present_mean",
            "max_sustained_stress_score": "baseline_max_sustained_stress_score",
        }
    )
    joined = current.join(prior, on="engine_family", how="left").with_columns(
        (pl.col("current_mean_score") - pl.col("baseline_mean_score").fill_null(0))
        .round(1)
        .alias("score_delta"),
        (
            pl.col("current_route_present_mean")
            - pl.col("baseline_route_present_mean").fill_null(0)
        )
        .round(1)
        .alias("route_delta"),
        (
            pl.col("current_activation_success_mean")
            - pl.col("baseline_activation_success_mean").fill_null(0)
        )
        .round(1)
        .alias("activation_delta"),
    )
    return joined.select(
        "engine_family",
        "current_config_id",
        "baseline_config_id",
        "score_delta",
        "route_delta",
        "activation_delta",
    ), baseline_dir


def write_recommendations(path: Path, recommendations: pl.DataFrame) -> None:
    lines = [
        "# Tuning Recommendations",
        "",
        "These recommendations are derived from the aggregate sweep artifacts in this run.",
        "They should be read as robust defaults for this tuning corpus, not as single-scenario winners.",
        "",
    ]
    for engine_family in ["batman", "pathway", "field", "comparison"]:
        rows = top_recommendation_rows(recommendations, engine_family, 3)
        if not rows and engine_family != "field":
            continue
        if not rows and engine_family == "field":
            lines.append("## Field")
            lines.append("")
            lines.append("No measured Field default is published for this artifact set.")
            lines.append("")
            lines.append(
                "The simulator still exports Field replay, search, reconfiguration, and bootstrap surfaces, but this corpus does not yield a stable bootstrap-to-steady route-visible default. Field should therefore be read as diagnostic-only in this report."
            )
            lines.append("")
            continue
        top = rows[0]
        lines.append(f"## {engine_family.capitalize()}")
        lines.append("")
        lines.append(
            f"Primary recommendation: `{top['config_id']}` "
            f"(score={top['mean_score']:.1f}, activation={top['activation_success_mean']:.1f} permille, "
            f"route_presence={top['route_present_mean']:.1f} permille, "
            f"max_stress={top['max_sustained_stress_score']})."
        )
        if engine_family == "field":
            lines.append(
                "Bootstrap profile: "
                f"activation={top['field_bootstrap_activation_mean']:.1f} permille, "
                f"upgrade={top['field_bootstrap_upgrade_mean']:.1f} permille, "
                f"withdrawal={top['field_bootstrap_withdraw_mean']:.1f} permille."
            )
        lines.append("")
        lines.append("Nearby acceptable range:")
        for row in rows:
            lines.append(
                f"- `{row['config_id']}` score={row['mean_score']:.1f}, "
                f"activation={row['activation_success_mean']:.1f} permille, "
                f"route_presence={row['route_present_mean']:.1f} permille, "
                f"max_stress={row['max_sustained_stress_score']}"
            )
        lines.append("")
    path.write_text("\n".join(lines))


def comparison_summary_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    return (
        aggregates.filter(pl.col("engine_family") == "comparison")
        .select(
            "family_id",
            "config_id",
            "dominant_engine",
            "activation_success_permille_mean",
            "route_present_permille_mean",
            "stress_score",
        )
        .sort(["family_id", "route_present_permille_mean"], descending=[False, True])
    )


def head_to_head_summary_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    return (
        aggregates.filter(pl.col("engine_family") == "head-to-head")
        .select(
            "family_id",
            "config_id",
            "comparison_engine_set",
            "dominant_engine",
            "activation_success_permille_mean",
            "route_present_permille_mean",
            "stress_score",
        )
        .sort(
            ["family_id", "route_present_permille_mean", "activation_success_permille_mean"],
            descending=[False, True, True],
        )
    )
