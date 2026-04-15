"""Plot dimensions, per-engine color palettes, heuristic line styles, and recommendation-profile scoring weights."""

from __future__ import annotations

PLOT_SPECS = {
    "batman_bellman_transition_stability": (11.0, 4.8),
    "batman_bellman_transition_loss": (11.0, 4.8),
    "pathway_budget_route_presence": (11.0, 4.8),
    "pathway_budget_activation": (11.0, 4.8),
    "field_budget_route_presence": (11.0, 8.4),
    "field_budget_reconfiguration": (11.0, 8.4),
    "batman_classic_transition_stability": (11.0, 4.8),
    "batman_classic_transition_loss": (11.0, 4.8),
    "babel_decay_stability": (11.0, 4.8),
    "babel_decay_loss": (11.0, 4.8),
    "olsrv2_decay_stability": (11.0, 6.2),
    "olsrv2_decay_loss": (11.0, 6.2),
    "scatter_profile_route_presence": (11.0, 6.8),
    "scatter_profile_startup": (11.0, 6.8),
    "comparison_dominant_engine": (8.1, 4.8),
    "head_to_head_route_presence": (9.2, 5.0),
    "head_to_head_timing_profile": (12.2, 5.2),
    "recommended_engine_robustness": (9.2, 5.2),
    "mixed_vs_standalone_divergence": (10.0, 5.2),
    "diffusion_delivery_coverage": (18.0, 20.0),
    "diffusion_resource_boundedness": (18.0, 20.0),
}

BATMAN_BELLMAN_FAMILY_COLORS = {
    "batman-bellman-decay-window-pressure": "#0072B2",
    "batman-bellman-partition-recovery": "#009E73",
    "batman-bellman-asymmetry-relink-transition": "#D55E00",
}

PATHWAY_FAMILY_COLORS = {
    "pathway-search-budget-pressure": "#0072B2",
    "pathway-high-fanout-budget-pressure": "#E69F00",
    "pathway-bridge-failure-service": "#009E73",
}

BATMAN_CLASSIC_FAMILY_COLORS = {
    "batman-classic-decay-window-pressure": "#1D4ED8",
    "batman-classic-partition-recovery": "#B45309",
    "batman-classic-asymmetry-relink-transition": "#059669",
}

BABEL_FAMILY_COLORS = {
    "babel-decay-window-pressure": "#7A1F4D",
    "babel-asymmetry-cost-penalty": "#A23B72",
    "babel-partition-feasibility-recovery": "#8A6F00",
}

OLSRV2_FAMILY_COLORS = {
    "olsrv2-topology-propagation-latency": "#0F766E",
    "olsrv2-partition-recovery": "#0EA5A4",
    "olsrv2-mpr-flooding-stability": "#0891B2",
    "olsrv2-asymmetric-relink-transition": "#155E75",
}

SCATTER_FAMILY_COLORS = {
    "scatter-connected-low-loss": "#C2410C",
    "scatter-connected-high-loss": "#EA580C",
    "scatter-bridge-transition": "#9A3412",
    "scatter-partial-observability-bridge": "#B45309",
    "scatter-concurrent-mixed": "#F97316",
    "scatter-corridor-continuity-uncertainty": "#C2410C",
    "scatter-medium-bridge-repair": "#7C2D12",
}

SCATTER_PROFILE_COLORS = {
    "balanced": "#C2410C",
    "conservative": "#9A3412",
    "degraded-network": "#F97316",
}

FIELD_FAMILY_COLORS = {
    "field-partial-observability-bridge": "#0072B2",
    "field-reconfiguration-recovery": "#009E73",
    "field-asymmetric-envelope-shift": "#D55E00",
    "field-uncertain-service-fanout": "#CC79A7",
    "field-service-overlap-reselection": "#7C3AED",
    "field-service-freshness-inversion": "#C2410C",
    "field-service-publication-pressure": "#0F766E",
    "field-bridge-anti-entropy-continuity": "#56B4E9",
    "field-bootstrap-upgrade-window": "#F0E442",
}

COMPARISON_ENGINE_COLORS = {
    "batman-bellman": "#0072B2",
    "batman-classic": "#56B4E9",
    "babel": "#882255",
    "olsrv2": "#0F766E",
    "pathway": "#009E73",
    "scatter": "#C2410C",
    "field": "#CC79A7",
    "tie": "#6B7280",
    "none": "#999999",
}

HEAD_TO_HEAD_SET_COLORS = {
    "batman-bellman": "#0072B2",
    "batman-classic": "#56B4E9",
    "babel": "#882255",
    "olsrv2": "#0F766E",
    "pathway": "#009E73",
    "scatter": "#C2410C",
    "field": "#CC79A7",
    "pathway-batman-bellman": "#E69F00",
}

ROUTE_VISIBLE_ENGINE_SET_ORDER = [
    "batman-classic",
    "batman-bellman",
    "babel",
    "olsrv2",
    "pathway",
    "scatter",
    "pathway-batman-bellman",
    "field",
]

DIFFUSION_BOUND_STATE_COLORS = {
    "viable": "#0F766E",
    "collapse": "#B45309",
    "explosive": "#B91C1C",
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
        "field_service_reward": 0.003,
        "field_shift_penalty": 32.0,
        "field_narrow_reward": 14.0,
        "field_degraded_round_penalty": 0.14,
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
    "field-stable-service": {
        "activation_weight": 2.5,
        "route_weight": 1.4,
        "stability_weight": 0.03,
        "stress_weight": 5.0,
        "materialization_weight": 0.0,
        "recovery_weight": 2.0,
        "churn_penalty": 35.0,
        "maintenance_penalty": 110.0,
        "reachability_penalty": 120.0,
        "degraded_penalty": 75.0,
        "field_service_reward": 0.012,
        "field_shift_penalty": 18.0,
        "field_narrow_penalty": 10.0,
        "field_degraded_round_penalty": 0.15,
    },
    "field-low-churn": {
        "activation_weight": 2.0,
        "route_weight": 1.2,
        "stability_weight": 0.03,
        "stress_weight": 4.5,
        "materialization_weight": 0.0,
        "recovery_weight": 2.0,
        "churn_penalty": 45.0,
        "maintenance_penalty": 120.0,
        "reachability_penalty": 130.0,
        "degraded_penalty": 80.0,
        "field_service_reward": 0.004,
        "field_shift_penalty": 45.0,
        "field_narrow_reward": 18.0,
        "field_degraded_round_penalty": 0.18,
    },
    "field-broad-reselection": {
        "activation_weight": 2.0,
        "route_weight": 1.3,
        "stability_weight": 0.02,
        "stress_weight": 4.5,
        "materialization_weight": 0.0,
        "recovery_weight": 1.0,
        "churn_penalty": 22.0,
        "maintenance_penalty": 90.0,
        "reachability_penalty": 110.0,
        "degraded_penalty": 60.0,
        "field_service_reward": 0.02,
        "field_shift_reward": 18.0,
        "field_narrow_penalty": 20.0,
        "field_degraded_round_penalty": 0.08,
    },
    "field-conservative-publication": {
        "activation_weight": 2.3,
        "route_weight": 1.2,
        "stability_weight": 0.03,
        "stress_weight": 5.0,
        "materialization_weight": 0.0,
        "recovery_weight": 2.0,
        "churn_penalty": 40.0,
        "maintenance_penalty": 120.0,
        "reachability_penalty": 130.0,
        "degraded_penalty": 85.0,
        "field_service_penalty": 0.005,
        "field_shift_penalty": 35.0,
        "field_narrow_reward": 32.0,
        "field_degraded_round_penalty": 0.2,
    },
}
