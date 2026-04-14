"""Score expressions, recommendation tables, transition and boundary metrics, baseline comparison, and recommendations text output."""

from __future__ import annotations

import json
from pathlib import Path

import polars as pl

from .constants import RECOMMENDATION_PROFILES


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
    return (
        pl.col("activation_success_permille_mean") * profile["activation_weight"]
        + pl.col("route_present_permille_mean") * profile["route_weight"]
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
    )


_OPTIONAL_FLOAT_COLUMNS = [
    "field_degraded_steady_entry_permille_mean",
    "field_degraded_steady_recovery_permille_mean",
    "field_degraded_to_bootstrap_permille_mean",
    "field_degraded_steady_round_permille_mean",
    "field_service_retention_carry_forward_permille_mean",
    "field_asymmetric_shift_success_permille_mean",
    "field_route_bound_reconfiguration_count_mean",
    "field_continuation_shift_count_mean",
    "field_corridor_narrow_count_mean",
]

_OPTIONAL_STR_COLUMNS = [
    "field_continuity_band_mode",
    "field_commitment_resolution_mode",
    "field_last_outcome_mode",
    "field_last_continuity_transition_mode",
]


def _ensure_optional_columns(df: pl.DataFrame) -> pl.DataFrame:
    for col in _OPTIONAL_FLOAT_COLUMNS:
        if col not in df.columns:
            df = df.with_columns(pl.lit(None).cast(pl.Float64).alias(col))
    for col in _OPTIONAL_STR_COLUMNS:
        if col not in df.columns:
            df = df.with_columns(pl.lit(None).cast(pl.String).alias(col))
    return df


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
            pl.col("field_continuity_band_mode")
            .drop_nulls()
            .mode()
            .first()
            .alias("field_continuity_band_mode"),
            pl.col("field_commitment_resolution_mode")
            .drop_nulls()
            .mode()
            .first()
            .alias("field_commitment_resolution_mode"),
            pl.col("field_last_outcome_mode")
            .drop_nulls()
            .mode()
            .first()
            .alias("field_last_outcome_mode"),
            pl.col("field_last_continuity_transition_mode")
            .drop_nulls()
            .mode()
            .first()
            .alias("field_last_continuity_transition_mode"),
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
        "batman-bellman": ["conservative", "aggressive", "degraded-network"],
        "batman-classic": ["conservative", "aggressive", "degraded-network"],
        "babel": ["conservative", "aggressive", "degraded-network"],
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
        pl.col("route_present_permille_mean").mean().alias("route_present_mean"),
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
        pl.col("field_continuity_band_mode")
        .drop_nulls()
        .mode()
        .first()
        .alias("field_continuity_band_mode"),
        pl.col("field_last_continuity_transition_mode")
        .drop_nulls()
        .mode()
        .first()
        .alias("field_last_continuity_transition_mode"),
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
                "stay continuity-biased long enough to exploit rare opportunities without overspread"
            )
        )
        .when(pl.col("field_regime") == "scarcity")
        .then(
            pl.lit(
                "enter scarcity early enough to cut energy and spread pressure before explosiveness"
            )
        )
        .when(pl.col("field_regime") == "congestion")
        .then(
            pl.lit(
                "enter congestion suppression early enough to bound redundant spread"
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


def _field_diffusion_regime_match_bonus_expr() -> pl.Expr:
    return (
        pl.when(
            (pl.col("field_regime") == "balanced") & (pl.col("config_id") == "field")
        )
        .then(40.0)
        .when(
            (pl.col("field_regime") == "continuity")
            & (pl.col("config_id") == "field-continuity")
        )
        .then(40.0)
        .when(
            (pl.col("field_regime") == "scarcity")
            & (pl.col("config_id") == "field-scarcity")
        )
        .then(40.0)
        .when(
            (pl.col("field_regime") == "congestion")
            & (pl.col("config_id") == "field-congestion")
        )
        .then(40.0)
        .when(
            (pl.col("field_regime") == "privacy")
            & (pl.col("config_id") == "field-privacy")
        )
        .then(40.0)
        .otherwise(0.0)
    )


def field_diffusion_regime_calibration_table(
    diffusion_aggregates: pl.DataFrame,
) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    field_rows = diffusion_aggregates.filter(
        pl.col("config_id").str.starts_with("field")
    ).with_columns(_field_diffusion_regime_expr().alias("field_regime"))
    grouped = field_rows.group_by("field_regime", "config_id").agg(
        pl.col("field_posture_mode").drop_nulls().mode().first().alias("field_posture_mode"),
        pl.col("delivery_probability_permille_mean").mean().alias("delivery_probability_mean"),
        pl.col("coverage_permille_mean").mean().alias("coverage_mean"),
        pl.col("total_transmissions_mean").mean().alias("total_transmissions_mean"),
        pl.col("energy_per_delivered_message_mean").mean().alias(
            "energy_per_delivered_message_mean"
        ),
        pl.col("storage_utilization_permille_mean").mean().alias(
            "storage_utilization_mean"
        ),
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
        pl.col("bounded_state_mode").drop_nulls().mode().first().alias("bounded_state_mode"),
    )
    scored = grouped.with_columns(
        (
            pl.when(pl.col("field_regime") == "continuity")
            .then(
                pl.col("delivery_probability_mean") * 1.0
                + pl.col("coverage_mean") * 0.4
                + pl.col("corridor_persistence_mean") * 0.45
                - pl.col("total_transmissions_mean") * 8.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.05
                - pl.when(pl.col("field_posture_mode") == "continuity_biased")
                .then(0.0)
                .otherwise(140.0)
                - pl.col("field_posture_transition_count_mean") * 20.0
            )
            .when(pl.col("field_regime") == "scarcity")
            .then(
                pl.col("delivery_probability_mean") * 0.85
                + pl.col("coverage_mean") * 0.3
                - pl.col("total_transmissions_mean") * 18.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.35
                - pl.col("storage_utilization_mean") * 0.35
                - pl.col("estimated_reproduction_mean") * 0.22
                - pl.when(pl.col("field_posture_mode") == "scarcity_conservative")
                .then(0.0)
                .otherwise(180.0)
                - pl.col("first_scarcity_transition_round_mean").fill_null(20) * 10.0
                - pl.when(pl.col("bounded_state_mode") == "explosive").then(320.0).otherwise(0.0)
            )
            .when(pl.col("field_regime") == "congestion")
            .then(
                pl.col("delivery_probability_mean") * 0.7
                + pl.col("coverage_mean") * 0.4
                - pl.col("total_transmissions_mean") * 16.0
                - pl.col("storage_utilization_mean") * 0.42
                - pl.col("estimated_reproduction_mean") * 0.28
                - pl.when(pl.col("field_posture_mode") == "congestion_suppressed")
                .then(0.0)
                .otherwise(180.0)
                - pl.col("first_congestion_transition_round_mean").fill_null(20) * 12.0
                - pl.when(pl.col("bounded_state_mode") == "explosive").then(320.0).otherwise(0.0)
            )
            .when(pl.col("field_regime") == "privacy")
            .then(
                pl.col("delivery_probability_mean") * 0.9
                + pl.col("coverage_mean") * 0.3
                - pl.col("observer_leakage_mean") * 1.2
                - pl.col("total_transmissions_mean") * 8.0
                - pl.when(pl.col("field_posture_mode") == "privacy_conservative")
                .then(0.0)
                .otherwise(120.0)
            )
            .otherwise(
                pl.col("delivery_probability_mean") * 0.9
                + pl.col("coverage_mean") * 0.4
                - pl.col("total_transmissions_mean") * 9.0
                - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.12
                - pl.col("observer_leakage_mean") * 0.4
                - pl.when(pl.col("field_posture_mode") == "balanced")
                .then(0.0)
                .otherwise(80.0)
            )
            + _field_diffusion_regime_match_bonus_expr()
        ).alias("regime_fit_score")
    )
    return (
        scored.sort(
            ["field_regime", "regime_fit_score", "config_id"],
            descending=[False, True, False],
        )
        .group_by("field_regime")
        .agg(
            pl.first("config_id").alias("config_id"),
            pl.first("field_posture_mode").alias("field_posture_mode"),
            pl.first("delivery_probability_mean").alias("delivery_probability_mean"),
            pl.first("coverage_mean").alias("coverage_mean"),
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
            pl.first("regime_fit_score").alias("regime_fit_score"),
        )
        .with_columns(_field_diffusion_success_criteria_expr().alias("success_criteria"))
        .select(
            "field_regime",
            "success_criteria",
            "config_id",
            "field_posture_mode",
            "delivery_probability_mean",
            "coverage_mean",
            "total_transmissions_mean",
            "observer_leakage_mean",
            "bounded_state_mode",
            "field_posture_transition_count_mean",
            "first_scarcity_transition_round_mean",
            "first_congestion_transition_round_mean",
            "regime_fit_score",
        )
        .sort("field_regime")
    )


def leading_recommendation_configs(
    recommendations: pl.DataFrame, limit_per_engine: int = 2
) -> pl.DataFrame:
    frames: list[pl.DataFrame] = []
    for engine_family in ["batman-bellman", "batman-classic", "babel", "pathway", "field", "comparison"]:
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
    for engine_family in ["batman-bellman", "batman-classic", "babel", "pathway", "field", "comparison"]:
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
    for engine_family in ["batman-bellman", "batman-classic", "babel", "pathway", "field", "comparison"]:
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
        .sort(["family_id", "route_present_permille_mean"], descending=[False, True])
        .group_by("family_id")
        .agg(
            pl.first("config_id").alias("config_id"),
            pl.first("dominant_engine").alias("dominant_engine"),
            pl.first("activation_success_permille_mean").alias("activation_success_permille_mean"),
            pl.first("route_present_permille_mean").alias("route_present_permille_mean"),
            pl.first("stress_score").alias("stress_score"),
        )
        .select(
            "family_id",
            "config_id",
            "dominant_engine",
            "activation_success_permille_mean",
            "route_present_permille_mean",
            "stress_score",
        )
        .sort("family_id")
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


def diffusion_engine_summary_table(diffusion_aggregates: pl.DataFrame) -> pl.DataFrame:
    if diffusion_aggregates.is_empty():
        return pl.DataFrame()
    scored = diffusion_aggregates.with_columns(
        (
            pl.col("delivery_probability_permille_mean") * 1.0
            + pl.col("coverage_permille_mean") * 0.6
            + pl.col("corridor_persistence_permille_mean") * 0.15
            - pl.col("delivery_latency_rounds_mean").fill_null(0) * 16.0
            - pl.col("total_transmissions_mean") * 10.0
            - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.18
            - pl.col("storage_utilization_permille_mean") * 0.25
            - pl.col("estimated_reproduction_permille_mean") * 0.15
            - pl.col("observer_leakage_permille_mean") * 0.45
            - pl.when(pl.col("bounded_state_mode") == "explosive").then(320.0).otherwise(0.0)
            - pl.when(pl.col("bounded_state_mode") == "collapse").then(220.0).otherwise(0.0)
        ).alias("score")
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
        (
            pl.col("delivery_probability_permille_mean") * 1.0
            + pl.col("coverage_permille_mean") * 0.6
            + pl.col("corridor_persistence_permille_mean") * 0.15
            - pl.col("delivery_latency_rounds_mean").fill_null(0) * 16.0
            - pl.col("total_transmissions_mean") * 10.0
            - pl.col("energy_per_delivered_message_mean").fill_null(0) * 0.18
            - pl.col("storage_utilization_permille_mean") * 0.25
            - pl.col("estimated_reproduction_permille_mean") * 0.15
            - pl.col("observer_leakage_permille_mean") * 0.45
            - pl.when(pl.col("bounded_state_mode") == "explosive").then(320.0).otherwise(0.0)
            - pl.when(pl.col("bounded_state_mode") == "collapse").then(220.0).otherwise(0.0)
        ).alias("score")
    ).sort(["family_id", "score", "config_id"], descending=[False, True, False])


def diffusion_boundary_table(diffusion_boundaries: pl.DataFrame) -> pl.DataFrame:
    if diffusion_boundaries.is_empty():
        return pl.DataFrame()
    return diffusion_boundaries.sort(
        ["viable_family_count", "config_id"], descending=[True, False]
    )
