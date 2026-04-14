"""Plot dimensions, per-engine color palettes, heuristic line styles, and recommendation-profile scoring weights."""

from __future__ import annotations

PLOT_SPECS = {
    "batman_transition_stability": (7.4, 4.8),
    "batman_transition_loss": (7.4, 4.8),
    "pathway_budget_route_presence": (7.4, 4.8),
    "pathway_budget_activation": (7.4, 4.8),
    "field_budget_route_presence": (7.4, 4.8),
    "field_budget_reconfiguration": (7.4, 4.8),
    "comparison_dominant_engine": (7.4, 4.8),
    "head_to_head_route_presence": (7.6, 5.0),
}

BATMAN_FAMILY_COLORS = {
    "batman-decay-window-pressure": "#0072B2",
    "batman-partition-recovery": "#009E73",
    "batman-asymmetry-relink-transition": "#D55E00",
}

PATHWAY_FAMILY_COLORS = {
    "pathway-search-budget-pressure": "#0072B2",
    "pathway-high-fanout-budget-pressure": "#E69F00",
    "pathway-bridge-failure-service": "#009E73",
}

FIELD_FAMILY_COLORS = {
    "field-partial-observability-bridge": "#0072B2",
    "field-reconfiguration-recovery": "#009E73",
    "field-asymmetric-envelope-shift": "#D55E00",
    "field-uncertain-service-fanout": "#CC79A7",
    "field-bridge-anti-entropy-continuity": "#56B4E9",
    "field-bootstrap-upgrade-window": "#F0E442",
}

COMPARISON_ENGINE_COLORS = {
    "batman": "#0072B2",
    "pathway": "#009E73",
    "field": "#CC79A7",
    "none": "#999999",
}

HEAD_TO_HEAD_SET_COLORS = {
    "batman": "#0072B2",
    "pathway": "#009E73",
    "field": "#CC79A7",
    "pathway-batman": "#E69F00",
}

HEURISTIC_LINESTYLES = {
    "zero": "solid",
    "hop-lower-bound": "dashed",
}

RECOMMENDATION_PROFILES = {
    "balanced": {
        "activation_weight": 3.0,
        "route_weight": 1.0,
        "stability_weight": 0.05,
        "stress_weight": 5.0,
        "materialization_weight": 0.0,
        "recovery_weight": 0.0,
        "churn_penalty": 40.0,
        "maintenance_penalty": 100.0,
        "reachability_penalty": 120.0,
        "degraded_penalty": 60.0,
    },
    "conservative": {
        "activation_weight": 4.0,
        "route_weight": 1.5,
        "stability_weight": 0.06,
        "stress_weight": 6.0,
        "materialization_weight": 0.0,
        "recovery_weight": 0.0,
        "churn_penalty": 55.0,
        "maintenance_penalty": 140.0,
        "reachability_penalty": 150.0,
        "degraded_penalty": 90.0,
    },
    "aggressive": {
        "activation_weight": 2.0,
        "route_weight": 2.0,
        "stability_weight": 0.03,
        "stress_weight": 6.5,
        "materialization_weight": 8.0,
        "recovery_weight": 4.0,
        "churn_penalty": 25.0,
        "maintenance_penalty": 70.0,
        "reachability_penalty": 90.0,
        "degraded_penalty": 40.0,
    },
    "degraded-network": {
        "activation_weight": 4.0,
        "route_weight": 1.5,
        "stability_weight": 0.06,
        "stress_weight": 8.0,
        "materialization_weight": 0.0,
        "recovery_weight": 6.0,
        "churn_penalty": 35.0,
        "maintenance_penalty": 120.0,
        "reachability_penalty": 150.0,
        "degraded_penalty": 90.0,
    },
    "service-heavy": {
        "activation_weight": 3.0,
        "route_weight": 2.0,
        "stability_weight": 0.02,
        "stress_weight": 5.0,
        "materialization_weight": 6.0,
        "recovery_weight": 5.0,
        "churn_penalty": 35.0,
        "maintenance_penalty": 100.0,
        "reachability_penalty": 120.0,
        "degraded_penalty": 60.0,
    },
}

