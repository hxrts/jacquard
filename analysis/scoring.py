"""Score expressions, recommendation tables, transition and boundary metrics, baseline comparison, and recommendations text output."""

from __future__ import annotations

import json
from pathlib import Path

import polars as pl

from .constants import (
    LARGE_POPULATION_DIFFUSION_FAMILIES,
    LARGE_POPULATION_ROUTE_FAMILIES,
    RECOMMENDATION_PROFILES,
    ROUTING_FITNESS_CROSSOVER_FAMILIES,
    ROUTING_FITNESS_MULTI_FLOW_FAMILIES,
    ROUTING_FITNESS_STALE_FAMILIES,
    ROUTE_VISIBLE_ENGINE_SET_ORDER,
)


def score_expression(profile_id: str) -> pl.Expr:
    profile = RECOMMENDATION_PROFILES[profile_id]
    field_upgrade_bonus = profile.get("field_bootstrap_upgrade_bonus", 0.35)
    field_hold_penalty = profile.get("field_bootstrap_hold_penalty", 0.1)
    field_narrow_penalty = profile.get("field_bootstrap_narrow_penalty", 0.05)
    field_withdraw_penalty = profile.get("field_bootstrap_withdraw_penalty", 0.15)
    field_shift_penalty = profile.get("field_shift_penalty", 0.0)
    field_shift_reward = profile.get("field_shift_reward", 0.0)
    field_service_reward = profile.get("field_service_reward", 0.0)
    field_service_penalty = profile.get("field_service_penalty", 0.0)
    field_narrow_reward = profile.get("field_narrow_reward", 0.0)
    field_continuity_narrow_penalty = profile.get("field_narrow_penalty", 0.0)
    field_degraded_round_penalty = profile.get("field_degraded_round_penalty", 0.0)
    scatter_handoff_reward = profile.get("scatter_handoff_reward", 0.0)
    scatter_replicate_reward = profile.get("scatter_replicate_reward", 0.0)
    scatter_bridging_reward = profile.get("scatter_bridging_reward", 0.0)
    scatter_constrained_reward = profile.get("scatter_constrained_reward", 0.0)
    scatter_handoff_penalty = profile.get("scatter_handoff_penalty", 0.0)
    scatter_constrained_penalty = profile.get("scatter_constrained_penalty", 0.0)
    scatter_sparse_penalty = profile.get("scatter_sparse_penalty", 0.0)
    scatter_bridging_penalty = profile.get("scatter_bridging_penalty", 0.0)
    return (
        pl.col("activation_success_permille_mean") * profile["activation_weight"]
        + _aggregate_route_presence_expr() * profile["route_weight"]
        + (pl.col("stability_total_mean") * profile["stability_weight"])
        + pl.col("max_sustained_stress_score").fill_null(0) * profile["stress_weight"]
        + pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_bootstrap_upgrade_permille_mean") * field_upgrade_bonus)
        .otherwise(0)
        + pl.when(pl.col("engine_family") == "field")
        .then(
            pl.col("field_service_retention_carry_forward_permille_mean")
            * field_service_reward
        )
        .otherwise(0)
        + pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_continuation_shift_count_mean") * field_shift_reward)
        .otherwise(0)
        + pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_corridor_narrow_count_mean") * field_narrow_reward)
        .otherwise(0)
        + pl.when(pl.col("engine_family") == "scatter")
        .then(pl.col("scatter_handoff_rounds_mean") * scatter_handoff_reward)
        .otherwise(0)
        + pl.when(pl.col("engine_family") == "scatter")
        .then(pl.col("scatter_replicate_rounds_mean") * scatter_replicate_reward)
        .otherwise(0)
        + pl.when(pl.col("engine_family") == "scatter")
        .then(pl.col("scatter_bridging_rounds_mean") * scatter_bridging_reward)
        .otherwise(0)
        + pl.when(pl.col("engine_family") == "scatter")
        .then(pl.col("scatter_constrained_rounds_mean") * scatter_constrained_reward)
        .otherwise(0)
        - pl.col("first_materialization_round_mean").fill_null(0)
        * profile["materialization_weight"]
        - pl.col("recovery_round_mean").fill_null(0) * profile["recovery_weight"]
        - pl.col("route_churn_count_mean") * profile["churn_penalty"]
        - pl.col("maintenance_failure_count_mean") * profile["maintenance_penalty"]
        - pl.col("lost_reachability_count_mean") * profile["reachability_penalty"]
        - pl.col("persistent_degraded_count_mean") * profile["degraded_penalty"]
        - pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_bootstrap_hold_permille_mean") * field_hold_penalty)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_bootstrap_narrow_permille_mean") * field_narrow_penalty)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_bootstrap_withdraw_permille_mean") * field_withdraw_penalty)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_continuation_shift_count_mean") * field_shift_penalty)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "field")
        .then(
            pl.col("field_service_retention_carry_forward_permille_mean")
            * field_service_penalty
        )
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "field")
        .then(pl.col("field_corridor_narrow_count_mean") * field_continuity_narrow_penalty)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "field")
        .then(
            pl.col("field_degraded_steady_round_permille_mean")
            * field_degraded_round_penalty
        )
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "scatter")
        .then(pl.col("scatter_handoff_rounds_mean") * scatter_handoff_penalty)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "scatter")
        .then(pl.col("scatter_constrained_rounds_mean") * scatter_constrained_penalty)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "scatter")
        .then(pl.col("scatter_sparse_rounds_mean") * scatter_sparse_penalty)
        .otherwise(0)
        - pl.when(pl.col("engine_family") == "scatter")
        .then(pl.col("scatter_bridging_rounds_mean") * scatter_bridging_penalty)
        .otherwise(0)
    )


_OPTIONAL_FLOAT_COLUMNS = [
    "route_present_total_window_permille_mean",
    "activation_success_permille_min",
    "activation_success_permille_max",
    "activation_success_permille_spread",
    "route_present_permille_min",
    "route_present_permille_max",
    "route_present_permille_spread",
    "objective_route_presence_min_permille_mean",
    "objective_route_presence_max_permille_mean",
    "objective_route_presence_spread_mean",
    "objective_starvation_count_mean",
    "concurrent_route_round_count_mean",
    "first_disruption_round_mean",
    "stale_persistence_round_mean",
    "recovery_success_permille_mean",
    "unrecovered_after_loss_count_mean",
    "broker_participation_permille_mean",
    "broker_concentration_permille_mean",
    "broker_route_churn_count_mean",
    "active_route_hop_count_mean",
    "route_observation_count_mean",
    "batman_bellman_selected_rounds_mean",
    "batman_classic_selected_rounds_mean",
    "babel_selected_rounds_mean",
    "olsrv2_selected_rounds_mean",
    "pathway_selected_rounds_mean",
    "scatter_selected_rounds_mean",
    "scatter_sparse_rounds_mean",
    "scatter_dense_rounds_mean",
    "scatter_bridging_rounds_mean",
    "scatter_constrained_rounds_mean",
    "scatter_replicate_rounds_mean",
    "scatter_handoff_rounds_mean",
    "scatter_retained_message_peak_mean",
    "scatter_delivered_message_peak_mean",
    "field_selected_rounds_mean",
    "field_bootstrap_activation_permille_mean",
    "field_bootstrap_hold_permille_mean",
    "field_bootstrap_narrow_permille_mean",
    "field_bootstrap_upgrade_permille_mean",
    "field_bootstrap_withdraw_permille_mean",
    "field_degraded_steady_entry_permille_mean",
    "field_degraded_steady_recovery_permille_mean",
    "field_degraded_to_bootstrap_permille_mean",
    "field_degraded_steady_round_permille_mean",
    "field_service_retention_carry_forward_permille_mean",
    "field_asymmetric_shift_success_permille_mean",
    "field_protocol_reconfiguration_count_mean",
    "field_route_bound_reconfiguration_count_mean",
    "field_continuation_shift_count_mean",
    "field_corridor_narrow_count_mean",
]

_OPTIONAL_STR_COLUMNS = [
    "field_continuity_band_mode",
    "field_commitment_resolution_mode",
    "field_last_outcome_mode",
    "field_last_continuity_transition_mode",
    "field_last_promotion_decision_mode",
    "field_last_promotion_blocker_mode",
]


def _ensure_optional_columns(df: pl.DataFrame) -> pl.DataFrame:
    for col in _OPTIONAL_FLOAT_COLUMNS:
        if col not in df.columns:
            df = df.with_columns(pl.lit(None).cast(pl.Float64).alias(col))
    for col in _OPTIONAL_STR_COLUMNS:
        if col not in df.columns:
            df = df.with_columns(pl.lit(None).cast(pl.String).alias(col))
    return df


def _stable_mode_expr(column: str) -> pl.Expr:
    return pl.col(column).drop_nulls().mode().sort().first().alias(column)


def _aggregate_route_presence_expr() -> pl.Expr:
    return pl.coalesce(
        [
            pl.col("route_present_total_window_permille_mean"),
            pl.col("route_present_permille_mean"),
        ]
    )


def _run_route_presence_expr() -> pl.Expr:
    return pl.coalesce(
        [
            pl.col("route_present_total_window_permille"),
            pl.col("route_present_permille"),
        ]
    )


def recommendation_table(
    aggregates: pl.DataFrame, breakdowns: pl.DataFrame, profile_id: str = "balanced"
) -> pl.DataFrame:
    aggregates = _ensure_optional_columns(aggregates)
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
            _aggregate_route_presence_expr().mean().alias("route_present_mean"),
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
            pl.col("field_degraded_steady_entry_permille_mean")
            .mean()
            .alias("field_degraded_steady_entry_mean"),
            pl.col("field_degraded_steady_recovery_permille_mean")
            .mean()
            .alias("field_degraded_steady_recovery_mean"),
            pl.col("field_degraded_to_bootstrap_permille_mean")
            .mean()
            .alias("field_degraded_to_bootstrap_mean"),
            pl.col("field_degraded_steady_round_permille_mean")
            .mean()
            .alias("field_degraded_steady_round_mean"),
            pl.col("field_service_retention_carry_forward_permille_mean")
            .mean()
            .alias("field_service_retention_carry_forward_mean"),
            pl.col("field_asymmetric_shift_success_permille_mean")
            .mean()
            .alias("field_asymmetric_shift_success_mean"),
            pl.col("field_continuation_shift_count_mean")
            .mean()
            .alias("field_continuation_shift_mean"),
            pl.col("field_corridor_narrow_count_mean")
            .mean()
            .alias("field_corridor_narrow_mean"),
            pl.col("scatter_sparse_rounds_mean").mean().alias("scatter_sparse_mean"),
            pl.col("scatter_bridging_rounds_mean").mean().alias("scatter_bridging_mean"),
            pl.col("scatter_constrained_rounds_mean").mean().alias("scatter_constrained_mean"),
            pl.col("scatter_replicate_rounds_mean").mean().alias("scatter_replicate_mean"),
            pl.col("scatter_handoff_rounds_mean").mean().alias("scatter_handoff_mean"),
            pl.col("scatter_retained_message_peak_mean")
            .mean()
            .alias("scatter_retained_peak_mean"),
            pl.col("scatter_delivered_message_peak_mean")
            .mean()
            .alias("scatter_delivered_peak_mean"),
            _stable_mode_expr("field_continuity_band_mode"),
            _stable_mode_expr("field_commitment_resolution_mode"),
            _stable_mode_expr("field_last_outcome_mode"),
            _stable_mode_expr("field_last_continuity_transition_mode"),
            _stable_mode_expr("field_last_promotion_decision_mode"),
            _stable_mode_expr("field_last_promotion_blocker_mode"),
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
        .sort(
            ["engine_family", "mean_score", "config_id"],
            descending=[False, True, False],
        )
    )


def profile_recommendation_table(
    aggregates: pl.DataFrame, breakdowns: pl.DataFrame
) -> pl.DataFrame:
    frames: list[pl.DataFrame] = []
    for engine_family, profiles in {
        "batman-bellman": ["conservative", "aggressive", "degraded-network"],
        "batman-classic": ["conservative", "aggressive", "degraded-network"],
        "babel": ["conservative", "aggressive", "degraded-network"],
        "olsrv2": ["conservative", "aggressive", "degraded-network"],
        "scatter": ["balanced", "conservative", "degraded-network"],
        "pathway": ["balanced", "service-heavy", "degraded-network"],
        "field": [
            "balanced",
            "field-stable-service",
            "field-low-churn",
            "field-broad-reselection",
            "field-conservative-publication",
        ],
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
        "field_continuation_shift_mean",
        "field_service_retention_carry_forward_mean",
        "field_corridor_narrow_mean",
        "field_degraded_steady_round_mean",
        "scatter_sparse_mean",
        "scatter_bridging_mean",
        "scatter_constrained_mean",
        "scatter_replicate_mean",
        "scatter_handoff_mean",
        "scatter_retained_peak_mean",
        "scatter_delivered_peak_mean",
        "max_sustained_stress_score",
    )


def field_profile_recommendation_table(
    aggregates: pl.DataFrame, breakdowns: pl.DataFrame
) -> pl.DataFrame:
    frames: list[pl.DataFrame] = []
    for profile_id in [
        "field-stable-service",
        "field-low-churn",
        "field-broad-reselection",
        "field-conservative-publication",
    ]:
        table = recommendation_table(aggregates, breakdowns, profile_id).filter(
            pl.col("engine_family") == "field"
        )
        if table.is_empty():
            continue
        frames.append(table.head(1).with_columns(pl.lit(profile_id).alias("profile_id")))
    if not frames:
        return pl.DataFrame()
    return pl.concat(frames).select(
        "profile_id",
        "config_id",
        "mean_score",
        "activation_success_mean",
        "route_present_mean",
        "field_continuation_shift_mean",
        "field_service_retention_carry_forward_mean",
        "field_corridor_narrow_mean",
        "field_degraded_steady_round_mean",
        "max_sustained_stress_score",
    )


def _field_routing_regime_expr() -> pl.Expr:
    return (
        pl.when(
            pl.col("family_id").is_in(
                [
                    "field-partial-observability-bridge",
                    "field-bootstrap-upgrade-window",
                ]
            )
        )
        .then(pl.lit("bootstrap-upgrade"))
        .when(
            pl.col("family_id").is_in(
                [
                    "field-asymmetric-envelope-shift",
                    "field-reconfiguration-recovery",
                    "field-bridge-anti-entropy-continuity",
                ]
            )
        )
        .then(pl.lit("continuity-transition"))
        .when(
            pl.col("family_id").is_in(
                [
                    "field-uncertain-service-fanout",
                    "field-service-overlap-reselection",
                    "field-service-freshness-inversion",
                    "field-service-publication-pressure",
                ]
            )
        )
        .then(pl.lit("service-continuity"))
        .otherwise(pl.lit(None))
    )


def _field_routing_success_criteria_expr() -> pl.Expr:
    return (
        pl.when(pl.col("field_regime") == "bootstrap-upgrade")
        .then(
            pl.lit(
                "upgrade bootstrap cleanly and avoid withdrawal or degraded fallback"
            )
        )
        .when(pl.col("field_regime") == "continuity-transition")
        .then(
            pl.lit(
                "retain corridor continuity through recovery with bounded shift pressure"
            )
        )
        .when(pl.col("field_regime") == "service-continuity")
        .then(
            pl.lit(
                "preserve service continuity while keeping continuation churn bounded"
            )
        )
        .otherwise(pl.lit("none"))
    )


def field_routing_regime_calibration_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    aggregates = _ensure_optional_columns(aggregates)
    field_rows = (
        aggregates.filter(pl.col("engine_family") == "field")
        .with_columns(_field_routing_regime_expr().alias("field_regime"))
        .filter(pl.col("field_regime").is_not_null())
    )
    if field_rows.is_empty():
        return pl.DataFrame()
    grouped = field_rows.group_by("field_regime", "config_id").agg(
        _aggregate_route_presence_expr().mean().alias("route_present_mean"),
        pl.col("activation_success_permille_mean").mean().alias("activation_success_mean"),
        pl.col("stress_score").max().alias("stress_envelope"),
        pl.col("field_bootstrap_upgrade_permille_mean").mean().alias("bootstrap_upgrade_mean"),
        pl.col("field_bootstrap_withdraw_permille_mean").mean().alias("bootstrap_withdraw_mean"),
        pl.col("field_degraded_steady_recovery_permille_mean")
        .mean()
        .alias("degraded_recovery_mean"),
        pl.col("field_degraded_to_bootstrap_permille_mean")
        .mean()
        .alias("degraded_to_bootstrap_mean"),
        pl.col("field_service_retention_carry_forward_permille_mean")
        .mean()
        .alias("service_carry_mean"),
        pl.col("field_asymmetric_shift_success_permille_mean")
        .mean()
        .alias("asymmetric_shift_success_mean"),
        pl.col("field_continuation_shift_count_mean")
        .mean()
        .alias("continuation_shift_mean"),
        pl.col("field_route_bound_reconfiguration_count_mean")
        .mean()
        .alias("route_reconfiguration_mean"),
        pl.col("route_churn_count_mean").mean().alias("route_churn_mean"),
        _stable_mode_expr("field_continuity_band_mode"),
        _stable_mode_expr("field_last_continuity_transition_mode"),
    )
    scored = grouped.with_columns(
        pl.when(pl.col("field_regime") == "bootstrap-upgrade")
        .then(
            pl.col("route_present_mean") * 1.0
            + pl.col("activation_success_mean") * 0.5
            + pl.col("stress_envelope") * 8.0
            + pl.col("bootstrap_upgrade_mean") * 0.55
            + pl.col("degraded_recovery_mean") * 0.18
            - pl.col("bootstrap_withdraw_mean") * 0.35
            - pl.col("degraded_to_bootstrap_mean") * 0.22
            - pl.col("continuation_shift_mean") * 12.0
        )
        .when(pl.col("field_regime") == "continuity-transition")
        .then(
            pl.col("route_present_mean") * 1.2
            + pl.col("stress_envelope") * 8.0
            + pl.col("degraded_recovery_mean") * 0.25
            + pl.col("asymmetric_shift_success_mean") * 0.22
            - pl.col("continuation_shift_mean") * 26.0
            - pl.col("route_churn_mean") * 35.0
            - pl.col("degraded_to_bootstrap_mean") * 0.12
        )
        .otherwise(
            pl.col("route_present_mean") * 1.15
            + pl.col("stress_envelope") * 7.0
            + pl.col("service_carry_mean") * 0.012
            - pl.col("continuation_shift_mean") * 18.0
            - pl.col("route_reconfiguration_mean") * 12.0
            - pl.col("route_churn_mean") * 28.0
        )
        .alias("regime_fit_score")
    ).with_columns(
        (
            pl.when(pl.col("field_regime") == "bootstrap-upgrade")
            .then(
                pl.col("bootstrap_upgrade_mean") * 0.6
                + pl.col("degraded_recovery_mean") * 0.2
                - pl.col("bootstrap_withdraw_mean") * 0.4
                - pl.col("degraded_to_bootstrap_mean") * 0.2
            )
            .when(pl.col("field_regime") == "continuity-transition")
            .then(
                pl.col("degraded_recovery_mean") * 0.3
                + pl.col("asymmetric_shift_success_mean") * 0.3
                - pl.col("continuation_shift_mean") * 10.0
                - pl.col("route_churn_mean") * 12.0
            )
            .otherwise(
                pl.col("service_carry_mean") * 0.01
                - pl.col("continuation_shift_mean") * 8.0
                - pl.col("route_reconfiguration_mean") * 6.0
            )
        ).alias("transition_health")
    )
    return (
        scored.sort(["field_regime", "regime_fit_score", "config_id"], descending=[False, True, False])
        .group_by("field_regime")
        .agg(
            pl.first("config_id").alias("config_id"),
            pl.first("route_present_mean").alias("route_present_mean"),
            pl.first("activation_success_mean").alias("activation_success_mean"),
            pl.first("stress_envelope").alias("stress_envelope"),
            pl.first("transition_health").alias("transition_health"),
            pl.first("continuation_shift_mean").alias("continuation_shift_mean"),
            pl.first("service_carry_mean").alias("service_carry_mean"),
            pl.first("field_continuity_band_mode").alias("field_continuity_band_mode"),
            pl.first("field_last_continuity_transition_mode").alias(
                "field_last_continuity_transition_mode"
            ),
            pl.first("regime_fit_score").alias("regime_fit_score"),
        )
        .with_columns(_field_routing_success_criteria_expr().alias("success_criteria"))
        .select(
            "field_regime",
            "success_criteria",
            "config_id",
            "route_present_mean",
            "transition_health",
            "continuation_shift_mean",
            "service_carry_mean",
            "stress_envelope",
            "field_continuity_band_mode",
            "field_last_continuity_transition_mode",
            "regime_fit_score",
        )
        .sort("field_regime")
    )


def _field_diffusion_regime_expr() -> pl.Expr:
    return (
        pl.when(
            pl.col("family_id").is_in(
                [
                    "diffusion-bridge-drought",
                    "diffusion-partitioned-clusters",
                    "diffusion-sparse-long-delay",
                    "diffusion-mobility-shift",
                ]
            )
        )
        .then(pl.lit("continuity"))
        .when(pl.col("family_id") == "diffusion-energy-starved-relay")
        .then(pl.lit("scarcity"))
        .when(
            pl.col("family_id").is_in(
                [
                    "diffusion-congestion-cascade",
                    "diffusion-high-density-overload",
                    "diffusion-disaster-broadcast",
                ]
            )
        )
        .then(pl.lit("congestion"))
        .when(pl.col("family_id") == "diffusion-adversarial-observation")
        .then(pl.lit("privacy"))
        .otherwise(pl.lit("balanced"))
    )


def _field_diffusion_success_criteria_expr() -> pl.Expr:
    return (
        pl.when(pl.col("field_regime") == "continuity")
        .then(
            pl.lit(
                "preserve protected bridge budget long enough to exploit rare continuity opportunities without overspread"
            )
        )
        .when(pl.col("field_regime") == "scarcity")
        .then(
            pl.lit(
                "enter scarcity early and cut generic spread, expensive transport use, and energy before explosiveness"
            )
        )
        .when(pl.col("field_regime") == "congestion")
        .then(
            pl.lit(
                "enter congestion suppression early enough to bound redundant spread with deterministic suppression memory"
            )
        )
        .when(pl.col("field_regime") == "privacy")
        .then(
            pl.lit(
                "reduce observer-adjacent dissemination while preserving delivery"
            )
        )
        .otherwise(pl.lit("stay balanced when no stronger regime dominates"))
    )


def _field_diffusion_config_family_expr() -> pl.Expr:
    return (
        pl.when(pl.col("config_id").str.starts_with("field-continuity"))
        .then(pl.lit("continuity"))
        .when(pl.col("config_id").str.starts_with("field-scarcity"))
        .then(pl.lit("scarcity"))
        .when(pl.col("config_id").str.starts_with("field-congestion"))
        .then(pl.lit("congestion"))
        .when(pl.col("config_id").str.starts_with("field-privacy"))
        .then(pl.lit("privacy"))
        .otherwise(pl.lit("balanced"))
    )


def _field_diffusion_regime_match_bonus_expr() -> pl.Expr:
    return (
        pl.when(
            (pl.col("field_regime") == "balanced") & (pl.col("config_id") == "field")
        )
        .then(40.0)
        .when(
            (pl.col("field_regime") == "continuity")
            & pl.col("config_id").str.starts_with("field-continuity")
        )
        .then(40.0)
        .when(
            (pl.col("field_regime") == "scarcity")
            & pl.col("config_id").str.starts_with("field-scarcity")
        )
        .then(40.0)
        .when(
            (pl.col("field_regime") == "congestion")
            & pl.col("config_id").str.starts_with("field-congestion")
        )
        .then(40.0)
        .when(
            (pl.col("field_regime") == "privacy")
            & pl.col("config_id").str.starts_with("field-privacy")
        )
        .then(40.0)
        .otherwise(0.0)
    )


def _grouped_diffusion_regime_candidates(
    diffusion_aggregates: pl.DataFrame, regime_column: str
) -> pl.DataFrame:
    return diffusion_aggregates.with_columns(
        _field_diffusion_regime_expr().alias(regime_column)
    ).group_by(regime_column, "config_id").agg(
        _stable_mode_expr("field_posture_mode"),
        pl.col("delivery_probability_permille_mean").mean().alias("delivery_probability_mean"),
        pl.col("coverage_permille_mean").mean().alias("coverage_mean"),
        pl.col("cluster_coverage_permille_mean").mean().alias("cluster_coverage_mean"),
        pl.col("delivery_latency_rounds_mean").mean().alias("delivery_latency_mean"),
        pl.col("total_transmissions_mean").mean().alias("total_transmissions_mean"),
        pl.col("energy_per_delivered_message_mean").mean().alias(
            "energy_per_delivered_message_mean"
        ),
        pl.col("storage_utilization_permille_mean").mean().alias("storage_utilization_mean"),
        pl.col("estimated_reproduction_permille_mean").mean().alias(
            "estimated_reproduction_mean"
        ),
        pl.col("corridor_persistence_permille_mean").mean().alias(
            "corridor_persistence_mean"
        ),
        pl.col("observer_leakage_permille_mean").mean().alias("observer_leakage_mean"),
        pl.col("field_posture_transition_count_mean").mean().alias(
            "field_posture_transition_count_mean"
        ),
        pl.col("field_first_scarcity_transition_round_mean").mean().alias(
            "first_scarcity_transition_round_mean"
        ),
        pl.col("field_first_congestion_transition_round_mean").mean().alias(
            "first_congestion_transition_round_mean"
        ),
        pl.col("field_protected_budget_used_mean").mean().alias("protected_budget_used_mean"),
        pl.col("field_generic_budget_used_mean").mean().alias("generic_budget_used_mean"),
        pl.col("field_bridge_opportunity_count_mean").mean().alias(
            "bridge_opportunity_count_mean"
        ),
        pl.col("field_protected_bridge_usage_count_mean").mean().alias(
            "protected_bridge_usage_count_mean"
        ),
        pl.col("field_cluster_seed_opportunity_count_mean").mean().alias(
            "cluster_seed_opportunity_count_mean"
        ),
        pl.col("field_cluster_seed_usage_count_mean").mean().alias(
            "cluster_seed_usage_count_mean"
        ),
        pl.col("field_cluster_coverage_starvation_count_mean").mean().alias(
            "cluster_coverage_starvation_count_mean"
        ),
        pl.col("field_redundant_forward_suppression_count_mean").mean().alias(
            "redundant_forward_suppression_count_mean"
        ),
        pl.col("field_same_cluster_suppression_count_mean").mean().alias(
            "same_cluster_suppression_count_mean"
        ),
        pl.col("field_expensive_transport_suppression_count_mean").mean().alias(
            "expensive_transport_suppression_count_mean"
        ),
        pl.col("field_cluster_seeding_rounds_mean").mean().alias(
            "cluster_seeding_rounds_mean"
        ),
        pl.col("field_duplicate_suppressed_rounds_mean").mean().alias(
            "duplicate_suppressed_rounds_mean"
        ),
        _stable_mode_expr("bounded_state_mode"),
    )


def _regime_scored_diffusion_candidates(diffusion_aggregates: pl.DataFrame) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    grouped = _grouped_diffusion_regime_candidates(diffusion_aggregates, "diffusion_regime")
    return grouped.with_columns(
        (
            pl.when(pl.col("diffusion_regime") == "continuity")
            .then(
                pl.col("delivery_probability_mean") * 0.95
                + pl.col("coverage_mean") * 0.35
                + pl.col("cluster_coverage_mean") * 0.25
                + pl.col("corridor_persistence_mean") * 0.25
                - pl.col("total_transmissions_mean") * 8.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.08
            )
            .when(pl.col("diffusion_regime") == "scarcity")
            .then(
                pl.col("delivery_probability_mean") * 0.9
                + pl.col("coverage_mean") * 0.25
                - pl.col("total_transmissions_mean") * 14.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.35
                - pl.col("storage_utilization_mean") * 0.2
                - pl.col("estimated_reproduction_mean") * 0.15
            )
            .when(pl.col("diffusion_regime") == "congestion")
            .then(
                pl.col("delivery_probability_mean") * 0.55
                + pl.col("coverage_mean") * 0.2
                + pl.col("cluster_coverage_mean") * 1.0
                - pl.col("total_transmissions_mean") * 10.0
                - pl.col("storage_utilization_mean") * 0.18
                - pl.col("estimated_reproduction_mean") * 0.18
            )
            .when(pl.col("diffusion_regime") == "privacy")
            .then(
                pl.col("delivery_probability_mean") * 0.9
                + pl.col("coverage_mean") * 0.25
                - pl.col("observer_leakage_mean") * 1.5
                - pl.col("total_transmissions_mean") * 6.0
            )
            .otherwise(
                pl.col("delivery_probability_mean") * 0.95
                + pl.col("coverage_mean") * 0.4
                + pl.col("cluster_coverage_mean") * 0.2
                - pl.col("total_transmissions_mean") * 8.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.1
                - pl.col("observer_leakage_mean") * 0.3
            )
            - pl.when(pl.col("bounded_state_mode") == "explosive").then(420.0).otherwise(0.0)
            - pl.when(pl.col("bounded_state_mode") == "collapse").then(520.0).otherwise(0.0)
        ).alias("regime_score")
    )


def field_diffusion_regime_calibration_table(
    diffusion_aggregates: pl.DataFrame,
) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    grouped = _grouped_diffusion_regime_candidates(
        diffusion_aggregates.filter(pl.col("config_id").str.starts_with("field")),
        "field_regime",
    ).with_columns(_field_diffusion_config_family_expr().alias("config_family"))
    scored = grouped.with_columns(
        pl.when(pl.col("bridge_opportunity_count_mean") > 0)
        .then(
            pl.col("protected_bridge_usage_count_mean")
            * 1000.0
            / pl.col("bridge_opportunity_count_mean")
        )
        .otherwise(0.0)
        .alias("bridge_capture_ratio")
    ).with_columns(
        (
            pl.when(pl.col("field_regime") == "continuity")
            .then(
                pl.col("delivery_probability_mean") * 1.0
                + pl.col("coverage_mean") * 0.4
                + pl.col("corridor_persistence_mean") * 0.45
                + pl.col("bridge_capture_ratio") * 0.12
                + pl.col("protected_budget_used_mean") * 18.0
                - pl.col("total_transmissions_mean") * 9.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.05
                - pl.col("generic_budget_used_mean") * 12.0
                - pl.when(pl.col("field_posture_mode") == "continuity_biased")
                .then(0.0)
                .otherwise(140.0)
                - pl.when(pl.col("config_family") == "continuity")
                .then(0.0)
                .otherwise(120.0)
                - pl.col("field_posture_transition_count_mean") * 24.0
                - pl.when(pl.col("bounded_state_mode") == "explosive").then(320.0).otherwise(0.0)
                - pl.when(pl.col("bounded_state_mode") == "collapse").then(420.0).otherwise(0.0)
            )
            .when(pl.col("field_regime") == "scarcity")
            .then(
                pl.col("delivery_probability_mean") * 0.9
                + pl.col("coverage_mean") * 0.3
                - pl.col("total_transmissions_mean") * 16.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.42
                - pl.col("storage_utilization_mean") * 0.35
                - pl.col("estimated_reproduction_mean") * 0.22
                - pl.col("generic_budget_used_mean") * 34.0
                + pl.col("same_cluster_suppression_count_mean").clip(0.0, 20.0) * 12.0
                + pl.col("expensive_transport_suppression_count_mean").clip(0.0, 12.0) * 18.0
                - pl.when(pl.col("field_posture_mode") == "scarcity_conservative")
                .then(0.0)
                .otherwise(180.0)
                - pl.when(pl.col("config_family") == "scarcity")
                .then(0.0)
                .otherwise(120.0)
                - pl.col("first_scarcity_transition_round_mean").fill_null(20) * 18.0
                - pl.when(pl.col("bounded_state_mode") == "explosive").then(380.0).otherwise(0.0)
                - pl.when(pl.col("bounded_state_mode") == "collapse").then(520.0).otherwise(0.0)
            )
            .when(pl.col("field_regime") == "congestion")
            .then(
                pl.col("delivery_probability_mean") * 0.78
                + pl.col("coverage_mean") * 0.2
                + pl.col("cluster_coverage_mean") * 0.95
                - pl.col("total_transmissions_mean") * 15.0
                - pl.col("storage_utilization_mean") * 0.42
                - pl.col("estimated_reproduction_mean") * 0.3
                - pl.col("generic_budget_used_mean") * 20.0
                + pl.col("cluster_seed_usage_count_mean").clip(0.0, 8.0) * 24.0
                - pl.col("cluster_coverage_starvation_count_mean").clip(0.0, 12.0) * 32.0
                + pl.col("redundant_forward_suppression_count_mean").clip(0.0, 40.0) * 4.0
                + pl.col("same_cluster_suppression_count_mean").clip(0.0, 20.0) * 6.0
                - pl.when(
                    pl.col("field_posture_mode").is_in(
                        ["cluster_seeding", "duplicate_suppressed"]
                    )
                )
                .then(0.0)
                .otherwise(180.0)
                - pl.when(pl.col("config_family") == "congestion")
                .then(0.0)
                .otherwise(120.0)
                - pl.col("first_congestion_transition_round_mean").fill_null(20) * 16.0
                - pl.when(pl.col("duplicate_suppressed_rounds_mean") <= 0).then(160.0).otherwise(0.0)
                - pl.when(pl.col("bounded_state_mode") == "explosive").then(380.0).otherwise(0.0)
                - pl.when(pl.col("bounded_state_mode") == "collapse").then(620.0).otherwise(0.0)
            )
            .when(pl.col("field_regime") == "privacy")
            .then(
                pl.col("delivery_probability_mean") * 0.9
                + pl.col("coverage_mean") * 0.3
                - pl.col("observer_leakage_mean") * 1.2
                - pl.col("total_transmissions_mean") * 8.0
                + pl.col("expensive_transport_suppression_count_mean") * 8.0
                - pl.when(pl.col("field_posture_mode") == "privacy_conservative")
                .then(0.0)
                .otherwise(120.0)
                - pl.when(pl.col("config_family") == "privacy")
                .then(0.0)
                .otherwise(90.0)
            )
            .otherwise(
                pl.col("delivery_probability_mean") * 0.9
                + pl.col("coverage_mean") * 0.4
                - pl.col("total_transmissions_mean") * 9.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.12
                - pl.col("observer_leakage_mean") * 0.4
                - pl.col("generic_budget_used_mean") * 8.0
                - pl.when(pl.col("field_posture_mode") == "balanced")
                .then(0.0)
                .otherwise(80.0)
            )
            + _field_diffusion_regime_match_bonus_expr()
        ).alias("regime_fit_score")
    ).with_columns(
        pl.when(pl.col("field_regime") == "congestion")
        .then(
            (pl.col("bounded_state_mode") == "viable")
            & (pl.col("cluster_seed_usage_count_mean") > 0)
            & (pl.col("cluster_coverage_mean") >= 500.0)
            & (pl.col("cluster_coverage_starvation_count_mean") <= 6.0)
        )
        .otherwise(
            (pl.col("bounded_state_mode") != "collapse")
            & (pl.col("regime_fit_score") > 0.0)
        )
        .alias("acceptable_candidate"),
        (
            (pl.col("config_family") == pl.col("field_regime"))
            | ((pl.col("field_regime") == "balanced") & (pl.col("config_id") == "field"))
        )
        .alias("config_family_match")
    )
    ranked = scored.sort(
        [
            "field_regime",
            "acceptable_candidate",
            "config_family_match",
            "regime_fit_score",
            "config_id",
        ],
        descending=[False, True, True, True, False],
    )
    selected = ranked.group_by("field_regime").agg(
        pl.first("config_id").alias("best_attempt_config_id"),
        pl.first("acceptable_candidate").alias("acceptable_candidate"),
        pl.first("field_posture_mode").alias("field_posture_mode"),
        pl.first("delivery_probability_mean").alias("delivery_probability_mean"),
        pl.first("coverage_mean").alias("coverage_mean"),
        pl.first("cluster_coverage_mean").alias("cluster_coverage_mean"),
        pl.first("total_transmissions_mean").alias("total_transmissions_mean"),
        pl.first("observer_leakage_mean").alias("observer_leakage_mean"),
        pl.first("bounded_state_mode").alias("bounded_state_mode"),
        pl.first("field_posture_transition_count_mean").alias(
            "field_posture_transition_count_mean"
        ),
        pl.first("first_scarcity_transition_round_mean").alias(
            "first_scarcity_transition_round_mean"
        ),
        pl.first("first_congestion_transition_round_mean").alias(
            "first_congestion_transition_round_mean"
        ),
        pl.first("protected_budget_used_mean").alias("protected_budget_used_mean"),
        pl.first("generic_budget_used_mean").alias("generic_budget_used_mean"),
        pl.first("bridge_opportunity_count_mean").alias("bridge_opportunity_count_mean"),
        pl.first("protected_bridge_usage_count_mean").alias(
            "protected_bridge_usage_count_mean"
        ),
        pl.first("cluster_seed_opportunity_count_mean").alias(
            "cluster_seed_opportunity_count_mean"
        ),
        pl.first("cluster_seed_usage_count_mean").alias("cluster_seed_usage_count_mean"),
        pl.first("cluster_coverage_starvation_count_mean").alias(
            "cluster_coverage_starvation_count_mean"
        ),
        pl.first("cluster_seeding_rounds_mean").alias("cluster_seeding_rounds_mean"),
        pl.first("duplicate_suppressed_rounds_mean").alias(
            "duplicate_suppressed_rounds_mean"
        ),
        pl.first("redundant_forward_suppression_count_mean").alias(
            "redundant_forward_suppression_count_mean"
        ),
        pl.first("same_cluster_suppression_count_mean").alias(
            "same_cluster_suppression_count_mean"
        ),
        pl.first("expensive_transport_suppression_count_mean").alias(
            "expensive_transport_suppression_count_mean"
        ),
        pl.first("regime_fit_score").alias("regime_fit_score"),
    )
    return selected.with_columns(
        _field_diffusion_success_criteria_expr().alias("success_criteria"),
        pl.when(pl.col("acceptable_candidate"))
        .then(pl.col("best_attempt_config_id"))
        .otherwise(pl.lit("no-acceptable-field-candidate"))
        .alias("config_id"),
        pl.when(pl.col("acceptable_candidate"))
        .then(pl.lit("accepted"))
        .otherwise(pl.lit("no acceptable field candidate"))
        .alias("selection_status"),
    ).select(
        "field_regime",
        "success_criteria",
        "selection_status",
        "acceptable_candidate",
        "config_id",
        "best_attempt_config_id",
        "field_posture_mode",
        "delivery_probability_mean",
        "coverage_mean",
        "cluster_coverage_mean",
        "total_transmissions_mean",
        "observer_leakage_mean",
        "bounded_state_mode",
        "field_posture_transition_count_mean",
        "first_scarcity_transition_round_mean",
        "first_congestion_transition_round_mean",
        "protected_budget_used_mean",
        "generic_budget_used_mean",
        "bridge_opportunity_count_mean",
        "protected_bridge_usage_count_mean",
        "cluster_seed_opportunity_count_mean",
        "cluster_seed_usage_count_mean",
        "cluster_coverage_starvation_count_mean",
        "cluster_seeding_rounds_mean",
        "duplicate_suppressed_rounds_mean",
        "redundant_forward_suppression_count_mean",
        "same_cluster_suppression_count_mean",
        "expensive_transport_suppression_count_mean",
        "regime_fit_score",
    ).sort("field_regime")


def leading_recommendation_configs(
    recommendations: pl.DataFrame, limit_per_engine: int = 2
) -> pl.DataFrame:
    frames: list[pl.DataFrame] = []
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
            _run_route_presence_expr().mean().alias("route_present_mean"),
            _run_route_presence_expr()
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


def engine_family_label(engine_family: str) -> str:
    labels = {
        "batman-bellman": "BATMAN Bellman",
        "batman-classic": "BATMAN Classic",
        "babel": "Babel",
        "olsrv2": "OLSRv2",
        "scatter": "Scatter",
        "pathway": "Pathway",
        "field": "Field",
        "comparison": "Comparison",
    }
    return labels.get(engine_family, engine_family)


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
        lines.append(f"## {engine_family_label(engine_family)}")
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
        if engine_family == "scatter":
            lines.append(
                "Scatter runtime profile: "
                f"handoff={top['scatter_handoff_mean']:.1f}, "
                f"constrained={top['scatter_constrained_mean']:.1f}, "
                f"bridging={top['scatter_bridging_mean']:.1f}, "
                f"retained_peak={top['scatter_retained_peak_mean']:.1f}."
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
    aggregates = _ensure_optional_columns(aggregates)
    return (
        aggregates.filter(pl.col("engine_family") == "comparison")
        .sort(
            [
                "family_id",
                "route_present_total_window_permille_mean",
                "route_present_permille_mean",
            ],
            descending=[False, True, True],
        )
        .group_by("family_id")
        .agg(
            pl.first("config_id").alias("config_id"),
            pl.first("dominant_engine").alias("dominant_engine"),
            pl.first("activation_success_permille_mean").alias("activation_success_permille_mean"),
            pl.first("route_present_permille_mean").alias(
                "route_present_active_window_permille_mean"
            ),
            pl.first("route_present_total_window_permille_mean").alias(
                "route_present_total_window_permille_mean"
            ),
            pl.first("stress_score").alias("stress_score"),
        )
        .select(
            "family_id",
            "config_id",
            "dominant_engine",
            "activation_success_permille_mean",
            "route_present_active_window_permille_mean",
            "route_present_total_window_permille_mean",
            "stress_score",
        )
        .sort("family_id")
    )


def comparison_engine_round_breakdown_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    aggregates = _ensure_optional_columns(aggregates)
    return (
        aggregates.filter(pl.col("engine_family") == "comparison")
        .sort(
            [
                "family_id",
                "route_present_total_window_permille_mean",
                "route_present_permille_mean",
                "config_id",
            ],
            descending=[False, True, True, False],
        )
        .group_by("family_id")
        .agg(
            pl.first("config_id").alias("config_id"),
            pl.first("dominant_engine").alias("dominant_engine"),
            pl.first("route_present_permille_mean").alias(
                "route_present_active_window_permille_mean"
            ),
            pl.first("route_present_total_window_permille_mean").alias(
                "route_present_total_window_permille_mean"
            ),
            pl.first("engine_handoff_count_mean").alias("engine_handoff_count_mean"),
            pl.first("batman_bellman_selected_rounds_mean").alias(
                "batman_bellman_selected_rounds_mean"
            ),
            pl.first("batman_classic_selected_rounds_mean").alias(
                "batman_classic_selected_rounds_mean"
            ),
            pl.first("babel_selected_rounds_mean").alias("babel_selected_rounds_mean"),
            pl.first("olsrv2_selected_rounds_mean").alias("olsrv2_selected_rounds_mean"),
            pl.first("pathway_selected_rounds_mean").alias("pathway_selected_rounds_mean"),
            pl.first("scatter_selected_rounds_mean").alias("scatter_selected_rounds_mean"),
            pl.first("field_selected_rounds_mean").alias("field_selected_rounds_mean"),
        )
        .select(
            "family_id",
            "config_id",
            "dominant_engine",
            "route_present_active_window_permille_mean",
            "route_present_total_window_permille_mean",
            "engine_handoff_count_mean",
            "batman_bellman_selected_rounds_mean",
            "batman_classic_selected_rounds_mean",
            "babel_selected_rounds_mean",
            "olsrv2_selected_rounds_mean",
            "pathway_selected_rounds_mean",
            "scatter_selected_rounds_mean",
            "field_selected_rounds_mean",
        )
        .sort("family_id")
    )


def head_to_head_summary_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    aggregates = _ensure_optional_columns(aggregates)
    head_to_head_rows = aggregates.filter(
        (pl.col("engine_family") == "head-to-head")
        & pl.col("comparison_engine_set").is_not_null()
    )
    if head_to_head_rows.is_empty():
        return pl.DataFrame()
    engine_order = {engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)}
    return (
        head_to_head_rows
        .with_columns(
            pl.col("comparison_engine_set")
            .replace_strict(engine_order, default=len(engine_order))
            .alias("comparison_engine_order")
        )
        .select(
            "family_id",
            "config_id",
            "comparison_engine_set",
            "comparison_engine_order",
            "dominant_engine",
            "activation_success_permille_mean",
            pl.col("route_present_permille_mean").alias(
                "route_present_active_window_permille_mean"
            ),
            pl.col("route_present_total_window_permille_mean").alias(
                "route_present_total_window_permille_mean"
            ),
            "stress_score",
        )
        .sort(
            [
                "family_id",
                "route_present_total_window_permille_mean",
                "route_present_active_window_permille_mean",
                "activation_success_permille_mean",
                "comparison_engine_order",
            ],
            descending=[False, True, True, True, False],
        )
        .drop("comparison_engine_order")
    )


def comparison_config_sensitivity_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    if aggregates.is_empty():
        return pl.DataFrame()
    aggregates = _ensure_optional_columns(aggregates)
    rows: list[dict[str, object]] = []
    grouped: dict[tuple[str, str], list[dict[str, object]]] = {}
    for row in aggregates.iter_rows(named=True):
        engine_family = row.get("engine_family")
        if engine_family not in {"comparison", "head-to-head"}:
            continue
        key = (str(engine_family), str(row.get("family_id")))
        grouped.setdefault(key, []).append(row)
    topline_columns = [
        "activation_success_permille_mean",
        "route_present_total_window_permille_mean",
        "first_materialization_round_mean",
        "first_loss_round_mean",
        "recovery_success_permille_mean",
        "route_churn_count_mean",
    ]
    selection_columns = [
        "dominant_engine",
        "batman_bellman_selected_rounds_mean",
        "batman_classic_selected_rounds_mean",
        "babel_selected_rounds_mean",
        "olsrv2_selected_rounds_mean",
        "scatter_selected_rounds_mean",
        "pathway_selected_rounds_mean",
        "field_selected_rounds_mean",
    ]
    for (engine_family, family_id), family_rows in sorted(grouped.items()):
        config_ids = sorted(str(row.get("config_id")) for row in family_rows)
        topline_signatures = {
            tuple(row.get(column) for column in topline_columns) for row in family_rows
        }
        selection_signatures = {
            tuple(row.get(column) for column in selection_columns) for row in family_rows
        }
        topline_flat = len(topline_signatures) <= 1
        selection_flat = len(selection_signatures) <= 1
        if topline_flat and selection_flat:
            sensitivity_class = "flat-control"
        elif topline_flat:
            sensitivity_class = "selection-only"
        elif selection_flat:
            sensitivity_class = "topline-only"
        else:
            sensitivity_class = "topline-and-selection"
        rows.append(
            {
                "engine_family": engine_family,
                "family_id": family_id,
                "config_count": len(set(config_ids)),
                "topline_signature_count": len(topline_signatures),
                "selection_signature_count": len(selection_signatures),
                "topline_flat": topline_flat,
                "selection_flat": selection_flat,
                "sensitivity_class": sensitivity_class,
            }
        )
    return pl.from_dicts(rows, infer_schema_length=None) if rows else pl.DataFrame()


def benchmark_profile_audit_table(
    aggregates: pl.DataFrame, profile_recommendations: pl.DataFrame
) -> pl.DataFrame:
    if aggregates.is_empty():
        return pl.DataFrame()
    aggregates = _ensure_optional_columns(aggregates)
    head_to_head_rows = aggregates.filter(
        (pl.col("engine_family") == "head-to-head")
        & pl.col("comparison_engine_set").is_not_null()
    )
    if head_to_head_rows.is_empty():
        return pl.DataFrame()
    order = {engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)}
    representative = (
        head_to_head_rows
        .with_columns(
            pl.col("comparison_engine_set")
            .replace_strict(order, default=len(order))
            .alias("comparison_engine_order")
        )
        .select(
            pl.col("comparison_engine_set").alias("engine_set"),
            pl.col("config_id").alias("representative_config_id"),
            _aggregate_route_presence_expr().alias("representative_route_present_mean"),
            pl.col("activation_success_permille_mean").alias(
                "representative_activation_mean"
            ),
            pl.lit("fixed-representative").alias("representative_surface_kind"),
            "comparison_engine_order",
        )
        .unique(subset=["engine_set"], keep="first")
    )
    calibrated = (
        profile_recommendations.select(
            pl.col("engine_family").alias("engine_set"),
            pl.col("profile_id").alias("calibrated_profile_id"),
            pl.col("config_id").alias("calibrated_config_id"),
            pl.col("route_present_mean").alias("calibrated_route_present_mean"),
            pl.col("activation_success_mean").alias("calibrated_activation_mean"),
            pl.lit("calibrated-best").alias("calibrated_surface_kind"),
        )
        .filter(pl.col("engine_set") != "comparison")
    )
    return (
        representative.join(calibrated, on="engine_set", how="left")
        .with_columns(
            (
                pl.col("representative_config_id") == pl.col("calibrated_config_id")
            ).fill_null(False).alias("configs_match")
        )
        .sort("comparison_engine_order")
        .drop("comparison_engine_order")
    )


def _generic_diffusion_score_expr(weights: dict[str, float]) -> pl.Expr:
    return (
        pl.col("delivery_probability_permille_mean") * weights["delivery"]
        + pl.col("coverage_permille_mean") * weights["coverage"]
        + pl.col("cluster_coverage_permille_mean") * weights["cluster_coverage"]
        + pl.col("corridor_persistence_permille_mean") * weights["corridor_persistence"]
        - pl.col("delivery_latency_rounds_mean").fill_null(0) * weights["latency_penalty"]
        - pl.col("total_transmissions_mean") * weights["transmission_penalty"]
        - pl.col("energy_per_delivered_message_mean").fill_null(0)
        * weights["energy_penalty"]
        - pl.col("storage_utilization_permille_mean") * weights["storage_penalty"]
        - pl.col("estimated_reproduction_permille_mean") * weights["reproduction_penalty"]
        - pl.col("observer_leakage_permille_mean") * weights["observer_penalty"]
        - pl.when(pl.col("bounded_state_mode") == "explosive")
        .then(weights["explosive_penalty"])
        .otherwise(0.0)
        - pl.when(pl.col("bounded_state_mode") == "collapse")
        .then(weights["collapse_penalty"])
        .otherwise(0.0)
    )


_BALANCED_DIFFUSION_WEIGHTS = {
    "delivery": 1.0,
    "coverage": 0.6,
    "cluster_coverage": 0.35,
    "corridor_persistence": 0.15,
    "latency_penalty": 16.0,
    "transmission_penalty": 10.0,
    "energy_penalty": 0.18,
    "storage_penalty": 0.25,
    "reproduction_penalty": 0.15,
    "observer_penalty": 0.45,
    "explosive_penalty": 320.0,
    "collapse_penalty": 220.0,
}

_DELIVERY_HEAVY_DIFFUSION_WEIGHTS = {
    "delivery": 1.15,
    "coverage": 0.75,
    "cluster_coverage": 0.45,
    "corridor_persistence": 0.12,
    "latency_penalty": 12.0,
    "transmission_penalty": 8.0,
    "energy_penalty": 0.12,
    "storage_penalty": 0.18,
    "reproduction_penalty": 0.1,
    "observer_penalty": 0.3,
    "explosive_penalty": 240.0,
    "collapse_penalty": 180.0,
}

_BOUNDEDNESS_HEAVY_DIFFUSION_WEIGHTS = {
    "delivery": 0.85,
    "coverage": 0.5,
    "cluster_coverage": 0.35,
    "corridor_persistence": 0.2,
    "latency_penalty": 20.0,
    "transmission_penalty": 14.0,
    "energy_penalty": 0.25,
    "storage_penalty": 0.35,
    "reproduction_penalty": 0.22,
    "observer_penalty": 0.5,
    "explosive_penalty": 420.0,
    "collapse_penalty": 280.0,
}


def diffusion_baseline_audit_table(diffusion_aggregates: pl.DataFrame) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    for column in [
        "replication_budget",
        "message_horizon",
        "forward_probability_permille",
        "bridge_bias_permille",
    ]:
        if column not in diffusion_aggregates.columns:
            diffusion_aggregates = diffusion_aggregates.with_columns(
                pl.lit(None).cast(pl.Int64).alias(column)
            )
    baseline_ids = [
        "batman-classic",
        "batman-bellman",
        "babel",
        "olsrv2",
        "scatter",
        "pathway",
        "pathway-batman-bellman",
    ]
    return (
        diffusion_aggregates.filter(pl.col("config_id").is_in(baseline_ids))
        .group_by("config_id")
        .agg(
            pl.col("replication_budget").first().alias("replication_budget"),
            pl.col("message_horizon").first().alias("ttl_rounds"),
            pl.col("forward_probability_permille")
            .first()
            .alias("forward_probability_permille"),
            pl.col("bridge_bias_permille").first().alias("bridge_bias_permille"),
            pl.col("delivery_probability_permille_mean")
            .mean()
            .alias("delivery_probability_mean"),
            pl.col("coverage_permille_mean").mean().alias("coverage_mean"),
            pl.col("cluster_coverage_permille_mean")
            .mean()
            .alias("cluster_coverage_mean"),
            pl.col("observer_leakage_permille_mean")
            .mean()
            .alias("observer_leakage_mean"),
            _stable_mode_expr("bounded_state_mode"),
        )
        .sort("config_id")
    )


def diffusion_family_weight_sensitivity_table(
    diffusion_aggregates: pl.DataFrame,
) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()

    def winners_for(profile_id: str, weights: dict[str, float]) -> pl.DataFrame:
        return (
            diffusion_aggregates.with_columns(
                _generic_diffusion_score_expr(weights).alias("score")
            )
            .sort(["family_id", "score", "config_id"], descending=[False, True, False])
            .group_by("family_id")
            .agg(
                pl.first("config_id").alias(f"{profile_id}_winner_config_id"),
                pl.first("score").alias(f"{profile_id}_winner_score"),
            )
        )

    balanced = winners_for("balanced", _BALANCED_DIFFUSION_WEIGHTS)
    delivery_heavy = winners_for("delivery_heavy", _DELIVERY_HEAVY_DIFFUSION_WEIGHTS)
    boundedness_heavy = winners_for(
        "boundedness_heavy", _BOUNDEDNESS_HEAVY_DIFFUSION_WEIGHTS
    )
    return (
        balanced.join(delivery_heavy, on="family_id", how="inner")
        .join(boundedness_heavy, on="family_id", how="inner")
        .with_columns(
            (
                (
                    pl.col("balanced_winner_config_id")
                    == pl.col("delivery_heavy_winner_config_id")
                )
                & (
                    pl.col("balanced_winner_config_id")
                    == pl.col("boundedness_heavy_winner_config_id")
                )
            ).alias("winner_stable")
        )
        .sort("family_id")
    )


def diffusion_engine_summary_table(diffusion_aggregates: pl.DataFrame) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    scored = diffusion_aggregates.with_columns(
        _generic_diffusion_score_expr(_BALANCED_DIFFUSION_WEIGHTS).alias("score")
    )
    return (
        scored.sort(["family_id", "score"], descending=[False, True])
        .group_by("family_id")
        .agg(
            pl.first("config_id").alias("config_id"),
            pl.first("density").alias("density"),
            pl.first("mobility_model").alias("mobility_model"),
            pl.first("transport_mix").alias("transport_mix"),
            pl.first("pressure").alias("pressure"),
            pl.first("objective_regime").alias("objective_regime"),
            pl.first("stress_score").alias("stress_score"),
            pl.first("delivery_probability_permille_mean").alias("delivery_probability_permille_mean"),
            pl.first("coverage_permille_mean").alias("coverage_permille_mean"),
            pl.first("cluster_coverage_permille_mean").alias("cluster_coverage_permille_mean"),
            pl.first("delivery_latency_rounds_mean").alias("delivery_latency_rounds_mean"),
            pl.first("total_transmissions_mean").alias("total_transmissions_mean"),
            pl.first("energy_per_delivered_message_mean").alias("energy_per_delivered_message_mean"),
            pl.first("estimated_reproduction_permille_mean").alias("estimated_reproduction_permille_mean"),
            pl.first("observer_leakage_permille_mean").alias("observer_leakage_permille_mean"),
            pl.first("bounded_state_mode").alias("bounded_state_mode"),
            pl.first("score").alias("score"),
        )
        .sort("family_id")
    )


def diffusion_engine_comparison_table(diffusion_aggregates: pl.DataFrame) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    return diffusion_aggregates.with_columns(
        _generic_diffusion_score_expr(_BALANCED_DIFFUSION_WEIGHTS).alias("score")
    ).sort(["family_id", "score", "config_id"], descending=[False, True, False])


def diffusion_regime_engine_summary_table(
    diffusion_aggregates: pl.DataFrame,
) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    scored = _regime_scored_diffusion_candidates(diffusion_aggregates)
    return (
        scored.sort(
            ["diffusion_regime", "regime_score", "config_id"],
            descending=[False, True, False],
        )
        .group_by("diffusion_regime")
        .agg(
            pl.first("config_id").alias("config_id"),
            pl.first("delivery_probability_mean").alias("delivery_probability_mean"),
            pl.first("coverage_mean").alias("coverage_mean"),
            pl.first("cluster_coverage_mean").alias("cluster_coverage_mean"),
            pl.first("total_transmissions_mean").alias("total_transmissions_mean"),
            pl.first("observer_leakage_mean").alias("observer_leakage_mean"),
            pl.first("bounded_state_mode").alias("bounded_state_mode"),
            pl.first("regime_score").alias("regime_score"),
        )
        .sort("diffusion_regime")
    )


def field_vs_best_diffusion_alternative_table(
    diffusion_aggregates: pl.DataFrame,
    field_diffusion_regime_calibration: pl.DataFrame,
) -> pl.DataFrame:
    if diffusion_aggregates.is_empty() or field_diffusion_regime_calibration.is_empty():
        return pl.DataFrame()
    scored = _regime_scored_diffusion_candidates(diffusion_aggregates)
    alternatives = (
        scored.filter(~pl.col("config_id").str.starts_with("field"))
        .sort(
            ["diffusion_regime", "regime_score", "config_id"],
            descending=[False, True, False],
        )
        .group_by("diffusion_regime")
        .agg(
            pl.first("config_id").alias("alternative_config_id"),
            pl.first("delivery_probability_mean").alias("alternative_delivery_mean"),
            pl.first("coverage_mean").alias("alternative_coverage_mean"),
            pl.first("cluster_coverage_mean").alias("alternative_cluster_coverage_mean"),
            pl.first("total_transmissions_mean").alias("alternative_total_transmissions_mean"),
            pl.first("observer_leakage_mean").alias("alternative_observer_leakage_mean"),
            pl.first("bounded_state_mode").alias("alternative_bounded_state_mode"),
            pl.first("regime_score").alias("alternative_regime_score"),
        )
    )
    field_candidates = (
        scored.filter(pl.col("config_id").str.starts_with("field"))
        .rename(
            {
                "diffusion_regime": "field_regime",
                "config_id": "field_candidate_config_id",
                "delivery_probability_mean": "field_candidate_delivery_mean",
                "coverage_mean": "field_candidate_coverage_mean",
                "cluster_coverage_mean": "field_candidate_cluster_coverage_mean",
                "total_transmissions_mean": "field_candidate_total_transmissions_mean",
                "observer_leakage_mean": "field_candidate_observer_leakage_mean",
                "bounded_state_mode": "field_candidate_bounded_state_mode",
                "regime_score": "field_candidate_regime_score",
            }
        )
    )
    calibration = field_diffusion_regime_calibration.join(
        field_candidates,
        left_on=["field_regime", "best_attempt_config_id"],
        right_on=["field_regime", "field_candidate_config_id"],
        how="left",
    )
    return calibration.join(
        alternatives,
        left_on="field_regime",
        right_on="diffusion_regime",
        how="left",
    ).with_columns(
        (pl.col("field_candidate_delivery_mean") - pl.col("alternative_delivery_mean"))
        .round(1)
        .alias("delivery_delta"),
        (pl.col("field_candidate_coverage_mean") - pl.col("alternative_coverage_mean"))
        .round(1)
        .alias("coverage_delta"),
        (
            pl.col("field_candidate_cluster_coverage_mean")
            - pl.col("alternative_cluster_coverage_mean")
        )
        .round(1)
        .alias("cluster_coverage_delta"),
        (
            pl.col("field_candidate_total_transmissions_mean")
            - pl.col("alternative_total_transmissions_mean")
        )
        .round(1)
        .alias("tx_delta"),
        (pl.col("field_candidate_regime_score") - pl.col("alternative_regime_score"))
        .round(1)
        .alias("regime_score_delta"),
    ).select(
        "field_regime",
        "selection_status",
        "acceptable_candidate",
        "best_attempt_config_id",
        "field_candidate_bounded_state_mode",
        "alternative_config_id",
        "alternative_bounded_state_mode",
        "delivery_delta",
        "coverage_delta",
        "cluster_coverage_delta",
        "tx_delta",
        "regime_score_delta",
    ).sort("field_regime")



def _large_population_route_metadata() -> pl.DataFrame:
    return pl.from_dicts(LARGE_POPULATION_ROUTE_FAMILIES)


def _large_population_diffusion_metadata() -> pl.DataFrame:
    return pl.from_dicts(LARGE_POPULATION_DIFFUSION_FAMILIES)


def _routing_fitness_crossover_metadata() -> pl.DataFrame:
    return pl.from_dicts(ROUTING_FITNESS_CROSSOVER_FAMILIES)


def _routing_fitness_multi_flow_metadata() -> pl.DataFrame:
    return pl.from_dicts(ROUTING_FITNESS_MULTI_FLOW_FAMILIES)


def _routing_fitness_stale_metadata() -> pl.DataFrame:
    return pl.from_dicts(ROUTING_FITNESS_STALE_FAMILIES)


def large_population_route_summary_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    if aggregates.is_empty():
        return pl.DataFrame()
    aggregates = _ensure_optional_columns(aggregates)
    metadata = _large_population_route_metadata()
    engine_order = {engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)}
    families = metadata["family_id"].to_list()
    filtered = aggregates.filter(
        (pl.col("engine_family") == "head-to-head") & pl.col("family_id").is_in(families)
    )
    if filtered.is_empty():
        return pl.DataFrame()
    return (
        filtered.join(metadata, on="family_id", how="inner")
        .with_columns(
            pl.col("comparison_engine_set")
            .replace_strict(engine_order, default=len(engine_order))
            .alias("engine_order")
        )
        .group_by("topology_class", "topology_label", "comparison_engine_set", "engine_order")
        .agg(
            pl.when(pl.col("size_band") == "small")
            .then(pl.coalesce([pl.col("route_present_total_window_permille_mean"), pl.col("route_present_permille_mean")]))
            .otherwise(None)
            .max()
            .alias("small_route_present"),
            pl.when(pl.col("size_band") == "moderate")
            .then(pl.coalesce([pl.col("route_present_total_window_permille_mean"), pl.col("route_present_permille_mean")]))
            .otherwise(None)
            .max()
            .alias("moderate_route_present"),
            pl.when(pl.col("size_band") == "high")
            .then(pl.coalesce([pl.col("route_present_total_window_permille_mean"), pl.col("route_present_permille_mean")]))
            .otherwise(None)
            .max()
            .alias("high_route_present"),
            pl.when(pl.col("size_band") == "high")
            .then(pl.col("first_loss_round_mean"))
            .otherwise(None)
            .max()
            .alias("high_first_loss_round"),
            pl.when(pl.col("size_band") == "high")
            .then(pl.col("recovery_round_mean"))
            .otherwise(None)
            .max()
            .alias("high_recovery_round"),
            pl.when(pl.col("size_band") == "high")
            .then(pl.col("activation_success_permille_mean"))
            .otherwise(None)
            .max()
            .alias("high_activation_success"),
        )
        .with_columns(
            (pl.col("high_route_present") - pl.col("small_route_present")).alias(
                "small_to_high_route_delta"
            )
        )
        .sort(["topology_label", "engine_order"])
        .drop("engine_order")
    )


def routing_fitness_crossover_summary_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    if aggregates.is_empty():
        return pl.DataFrame()
    aggregates = _ensure_optional_columns(aggregates)
    metadata = _routing_fitness_crossover_metadata()
    families = metadata["family_id"].to_list()
    engine_order = {engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)}
    filtered = aggregates.filter(
        (pl.col("engine_family") == "head-to-head") & pl.col("family_id").is_in(families)
    )
    if filtered.is_empty():
        return pl.DataFrame()
    return (
        filtered.join(metadata, on="family_id", how="inner")
        .with_columns(
            pl.col("comparison_engine_set")
            .replace_strict(engine_order, default=len(engine_order))
            .alias("engine_order"),
            pl.coalesce(
                [
                    pl.col("route_present_total_window_permille_mean"),
                    pl.col("route_present_permille_mean"),
                ]
            ).alias("route_present_total_window_permille_mean"),
        )
        .select(
            "family_id",
            "question",
            "question_label",
            "band_label",
            "band_order",
            "comparison_engine_set",
            "engine_order",
            "route_present_total_window_permille_mean",
            "recovery_success_permille_mean",
            "first_loss_round_mean",
            "recovery_round_mean",
            "route_churn_count_mean",
            "active_route_hop_count_mean",
            "route_observation_count_mean",
        )
        .sort(["question_label", "band_order", "engine_order"])
        .drop("engine_order")
    )


def routing_fitness_multiflow_summary_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    if aggregates.is_empty():
        return pl.DataFrame()
    aggregates = _ensure_optional_columns(aggregates)
    metadata = _routing_fitness_multi_flow_metadata()
    families = metadata["family_id"].to_list()
    engine_order = {engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)}
    filtered = aggregates.filter(
        (pl.col("engine_family") == "head-to-head") & pl.col("family_id").is_in(families)
    )
    if filtered.is_empty():
        return pl.DataFrame()
    return (
        filtered.join(metadata, on="family_id", how="inner")
        .with_columns(
            pl.col("comparison_engine_set")
            .replace_strict(engine_order, default=len(engine_order))
            .alias("engine_order"),
            pl.coalesce(
                [
                    pl.col("route_present_total_window_permille_mean"),
                    pl.col("route_present_permille_mean"),
                ]
            ).alias("route_present_total_window_permille_mean"),
        )
        .with_columns(
            pl.when(
                pl.col("broker_participation_permille_mean").is_not_null()
                & pl.col("broker_concentration_permille_mean").is_not_null()
            )
            .then(pl.lit("attributed"))
            .when(pl.col("route_present_total_window_permille_mean").fill_null(0) == 0)
            .then(pl.lit("no-visible-route"))
            .otherwise(pl.lit("non-next-hop-route"))
            .alias("broker_metric_status")
        )
        .select(
            "family_id",
            "family_label",
            "family_order",
            "comparison_engine_set",
            "engine_order",
            "route_present_total_window_permille_mean",
            "objective_route_presence_min_permille_mean",
            "objective_route_presence_max_permille_mean",
            "objective_route_presence_spread_mean",
            "objective_starvation_count_mean",
            "concurrent_route_round_count_mean",
            "broker_participation_permille_mean",
            "broker_concentration_permille_mean",
            "broker_route_churn_count_mean",
            "broker_metric_status",
            "route_churn_count_mean",
            "active_route_hop_count_mean",
            "route_observation_count_mean",
        )
        .sort(["family_order", "engine_order"])
        .drop("engine_order")
    )


def routing_fitness_stale_repair_summary_table(aggregates: pl.DataFrame) -> pl.DataFrame:
    if aggregates.is_empty():
        return pl.DataFrame()
    aggregates = _ensure_optional_columns(aggregates)
    metadata = _routing_fitness_stale_metadata()
    families = metadata["family_id"].to_list()
    engine_order = {engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)}
    filtered = aggregates.filter(
        (pl.col("engine_family") == "head-to-head") & pl.col("family_id").is_in(families)
    )
    if filtered.is_empty():
        return pl.DataFrame()
    return (
        filtered.join(metadata, on="family_id", how="inner")
        .with_columns(
            pl.col("comparison_engine_set")
            .replace_strict(engine_order, default=len(engine_order))
            .alias("engine_order"),
            pl.coalesce(
                [
                    pl.col("route_present_total_window_permille_mean"),
                    pl.col("route_present_permille_mean"),
                ]
            ).alias("route_present_total_window_permille_mean"),
        )
        .select(
            "family_id",
            "family_label",
            "family_order",
            "comparison_engine_set",
            "engine_order",
            "route_present_total_window_permille_mean",
            "first_disruption_round_mean",
            "first_loss_round_mean",
            "stale_persistence_round_mean",
            "recovery_round_mean",
            "recovery_success_permille_mean",
            "unrecovered_after_loss_count_mean",
            "route_churn_count_mean",
            "route_observation_count_mean",
        )
        .sort(["family_order", "engine_order"])
        .drop("engine_order")
    )


def large_population_diffusion_state_points_table(
    diffusion_aggregates: pl.DataFrame,
) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    metadata = _large_population_diffusion_metadata()
    families = metadata["family_id"].to_list()
    filtered = diffusion_aggregates.filter(pl.col("family_id").is_in(families)).filter(
        pl.col("bounded_state_mode").is_not_null()
    )
    if filtered.is_empty():
        return pl.DataFrame()
    return (
        filtered.join(metadata, on="family_id", how="inner")
        .sort(
            [
                "question_label",
                "size_order",
                "bounded_state_mode",
                "delivery_probability_permille_mean",
                "coverage_permille_mean",
                "cluster_coverage_permille_mean",
                "total_transmissions_mean",
                "config_id",
            ],
            descending=[False, False, False, True, True, True, False, False],
        )
        .group_by("family_id", "question", "question_label", "family_label", "size_band", "size_order", "bounded_state_mode")
        .agg(
            pl.first("config_id").alias("config_id"),
            pl.first("delivery_probability_permille_mean").alias(
                "delivery_probability_permille_mean"
            ),
            pl.first("coverage_permille_mean").alias("coverage_permille_mean"),
            pl.first("cluster_coverage_permille_mean").alias(
                "cluster_coverage_permille_mean"
            ),
            pl.first("total_transmissions_mean").alias("total_transmissions_mean"),
            pl.first("estimated_reproduction_permille_mean").alias(
                "estimated_reproduction_permille_mean"
            ),
        )
        .sort(["question_label", "size_order", "bounded_state_mode"])
    )


def large_population_diffusion_transition_table(
    diffusion_aggregates: pl.DataFrame,
) -> pl.DataFrame:
    points = large_population_diffusion_state_points_table(diffusion_aggregates)
    if points.is_empty():
        return pl.DataFrame()
    metadata = _large_population_diffusion_metadata()
    collapse = points.filter(pl.col("bounded_state_mode") == "collapse").rename(
        {"config_id": "collapse_config_id"}
    ).select("family_id", "collapse_config_id")
    viable = points.filter(pl.col("bounded_state_mode") == "viable").rename(
        {"config_id": "viable_config_id"}
    ).select("family_id", "viable_config_id")
    explosive = points.filter(pl.col("bounded_state_mode") == "explosive").rename(
        {"config_id": "explosive_config_id"}
    ).select("family_id", "explosive_config_id")
    states = (
        points.group_by("family_id")
        .agg(pl.col("bounded_state_mode").sort().str.join(", ").alias("observed_states"))
    )
    return (
        metadata.join(states, on="family_id", how="left")
        .join(collapse, on="family_id", how="left")
        .join(viable, on="family_id", how="left")
        .join(explosive, on="family_id", how="left")
        .select(
            "family_id",
            "question",
            "question_label",
            "family_label",
            "size_band",
            "size_order",
            "observed_states",
            "collapse_config_id",
            "viable_config_id",
            "explosive_config_id",
        )
        .sort(["question_label", "size_order"])
    )



def diffusion_boundary_table(diffusion_boundaries: pl.DataFrame) -> pl.DataFrame:
    if diffusion_boundaries.is_empty():
        return pl.DataFrame()
    return diffusion_boundaries.sort(
        ["viable_family_count", "config_id"], descending=[True, False]
    )
