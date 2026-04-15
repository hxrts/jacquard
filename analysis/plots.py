"""Matplotlib render functions for each analysis figure and a shared save-plot-artifact helper."""

from __future__ import annotations

from pathlib import Path

import matplotlib.pyplot as plt
import polars as pl

from .constants import (
    BABEL_FAMILY_COLORS,
    BATMAN_BELLMAN_FAMILY_COLORS,
    BATMAN_CLASSIC_FAMILY_COLORS,
    COMPARISON_ENGINE_COLORS,
    DIFFUSION_BOUND_STATE_COLORS,
    FIELD_FAMILY_COLORS,
    HEAD_TO_HEAD_SET_COLORS,
    OLSRV2_FAMILY_COLORS,
    PLOT_SPECS,
    ROUTE_VISIBLE_ENGINE_SET_ORDER,
    SCATTER_FAMILY_COLORS,
)

PLOT_TEXT_COLOR = "#2f3437"
PLOT_MUTED_TEXT_COLOR = "#4b5563"
DIFFUSION_FIGURE_FAMILIES = [
    "diffusion-partitioned-clusters",
    "diffusion-sparse-long-delay",
    "diffusion-adversarial-observation",
    "diffusion-bridge-drought",
    "diffusion-energy-starved-relay",
    "diffusion-congestion-cascade",
]


def family_label(family_id: str) -> str:
    return (
        family_id.replace("batman-bellman-", "")
        .replace("pathway-", "")
        .replace("field-", "")
        .replace("olsrv2-", "")
        .replace("scatter-", "")
        .replace("comparison-", "")
        .replace("head-to-head-", "")
        .replace("-", " ")
    )


def break_tick_label(label: str) -> str:
    parts = label.split("-")
    if len(parts) < 2:
        return label
    midpoint = max(1, len(parts) // 2)
    return "-".join(parts[:midpoint]) + "\n" + "-".join(parts[midpoint:])


def heuristic_label(label: str | None) -> str:
    if label == "hop-lower-bound":
        return "hop-lb"
    return label or "zero"


def refresh_annotation(refresh: int | None) -> str:
    return f"r{refresh}" if refresh is not None else ""


def diffusion_config_label(config_id: str) -> str:
    return break_tick_label(config_id)


def diffusion_config_color(config_id: str) -> str:
    if config_id in HEAD_TO_HEAD_SET_COLORS:
        return HEAD_TO_HEAD_SET_COLORS[config_id]
    if config_id.startswith("field-"):
        if config_id == "field":
            return "#D16D9E"
        if config_id.startswith("field-continuity"):
            return {"field-continuity": "#CC79A7"}.get(config_id, "#B85F97")
        if config_id.startswith("field-scarcity"):
            return {"field-scarcity": "#B35C8C"}.get(config_id, "#8F4E72")
        if config_id.startswith("field-congestion"):
            return {"field-congestion": "#9D4EDD"}.get(config_id, "#5B21B6")
        if config_id.startswith("field-privacy"):
            return {"field-privacy": "#7A1F4D"}.get(config_id, "#5C173A")
        return "#CC79A7"
    return "#64748b"


def diffusion_engine_sets(diffusion_engine_comparison: pl.DataFrame) -> list[str]:
    preferred_order = [
        "batman-bellman",
        "batman-classic",
        "babel",
        "scatter",
        "pathway",
        "field",
        "field-continuity",
        "field-scarcity",
        "field-congestion",
        "field-privacy",
        "pathway-batman-bellman",
    ]
    available = set(diffusion_engine_comparison["config_id"].unique().to_list())
    ordered = [engine for engine in preferred_order if engine in available]
    remaining = sorted(available.difference(preferred_order))
    return [*ordered, *remaining]


def style_plot_axes(ax) -> None:
    ax.grid(alpha=0.22, color="#cbd5e1", linewidth=0.8)
    ax.set_facecolor("#fbfdff")
    for spine in ax.spines.values():
        spine.set_color("#94a3b8")
        spine.set_linewidth(0.8)
    ax.tick_params(axis="both", colors=PLOT_TEXT_COLOR, labelcolor=PLOT_TEXT_COLOR)
    ax.xaxis.label.set_color(PLOT_TEXT_COLOR)
    ax.yaxis.label.set_color(PLOT_TEXT_COLOR)
    ax.title.set_color(PLOT_TEXT_COLOR)


def style_legend(legend) -> None:
    if legend is None:
        return
    legend.get_title().set_color(PLOT_TEXT_COLOR)
    for text in legend.get_texts():
        text.set_color(PLOT_TEXT_COLOR)


OUTCOME_LINE_STYLE = {
    "marker": "o",
    "linestyle": "-",
    "linewidth": 2.0,
    "markersize": 5.8,
    "markeredgecolor": "white",
    "markeredgewidth": 0.8,
    "zorder": 3,
}

FRAGILITY_LINE_STYLE = {
    "marker": "s",
    "linestyle": "dashed",
    "linewidth": 1.8,
    "markersize": 5.6,
    "markeredgecolor": "white",
    "markeredgewidth": 0.8,
    "zorder": 3,
}

ENGINE_SECTION_COLORS = {
    "batman-classic": "#56B4E9",
    "batman-bellman": "#0072B2",
    "babel": "#882255",
    "olsrv2": "#0F766E",
    "scatter": "#C2410C",
}

HEURISTIC_COLORS = {"zero": "#0072B2", "hop-lower-bound": "#D55E00"}
HEURISTIC_MARKERS = {"zero": "o", "hop-lower-bound": "s"}


def _route_presence_percent(value: int | float | None) -> float | None:
    if value is None:
        return None
    return float(value) / 10000.0


def _activation_percent(value: int | float | None) -> float | None:
    if value is None:
        return None
    return float(value) / 10.0


def _sync_panel_ylim(panels, minimum_upper: float = 1.0) -> None:
    if not panels:
        return
    y_max = max(panel.get_ylim()[1] for panel in panels)
    y_max = max(minimum_upper, y_max)
    for panel in panels:
        panel.set_ylim(0, y_max)


def _render_single_series_family_sweep(
    ax,
    data: pl.DataFrame,
    families: list[str],
    x_column: str,
    y_column: str,
    xlabel: str,
    ylabel: str,
    color: str,
    line_style: dict[str, object],
    value_transform=None,
    tick_formatter=None,
    annotation_column: str | None = None,
    annotation_formatter=None,
    columns: int | None = None,
) -> None:
    if data.is_empty():
        return
    present_families = [
        family for family in families if not data.filter(pl.col("family_id") == family).is_empty()
    ]
    if not present_families:
        return
    fig = ax.figure
    fig.set_layout_engine(None)
    subplotspec = ax.get_subplotspec()
    ax.remove()
    columns_count = columns or len(present_families)
    rows_count = (len(present_families) + columns_count - 1) // columns_count
    grid = subplotspec.subgridspec(rows_count, columns_count, hspace=0.42, wspace=0.26)
    panels = []
    for index, family_id in enumerate(present_families):
        panel = fig.add_subplot(grid[index // columns_count, index % columns_count])
        panels.append(panel)
        rows = data.filter(pl.col("family_id") == family_id).sort(x_column)
        raw_xs = rows[x_column].to_list()
        ys_raw = rows[y_column].to_list()
        ys = [value_transform(value) if value_transform else value for value in ys_raw]
        categorical_x = bool(raw_xs) and not isinstance(raw_xs[0], (int, float))
        xs = list(range(len(raw_xs))) if categorical_x else raw_xs
        tick_labels = [tick_formatter(value) if tick_formatter else value for value in raw_xs]
        panel.plot(xs, ys, color=color, **line_style)
        if annotation_column is not None and annotation_formatter is not None:
            annotations = rows[annotation_column].to_list()
            for x_value, y_value, annotation_value in zip(
                xs, ys, annotations, strict=False
            ):
                panel.annotate(
                    annotation_formatter(annotation_value),
                    (x_value, y_value),
                    textcoords="offset points",
                    xytext=(0, 6),
                    ha="center",
                    fontsize=7.2,
                    color=PLOT_MUTED_TEXT_COLOR,
                )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel(xlabel)
        if index == 0:
            panel.set_ylabel(ylabel)
        panel.set_xticks(xs, tick_labels)
        style_plot_axes(panel)
    for index in range(len(present_families), rows_count * columns_count):
        panel = fig.add_subplot(grid[index // columns_count, index % columns_count])
        panel.axis("off")
    _sync_panel_ylim(
        panels,
        minimum_upper=100.0 if ylabel.endswith("(%)") else 1.0,
    )


def _render_multi_series_family_sweep(
    ax,
    data: pl.DataFrame,
    families: list[str],
    x_column: str,
    variant_column: str,
    variant_order: list[str],
    xlabel: str,
    ylabel: str,
    line_style: dict[str, object],
    y_selector,
    value_transform=None,
    columns: int = 3,
) -> None:
    if data.is_empty():
        return
    fig = ax.figure
    fig.set_layout_engine(None)
    subplotspec = ax.get_subplotspec()
    ax.remove()
    rows_count = (len(families) + columns - 1) // columns
    grid = subplotspec.subgridspec(rows_count, columns, hspace=0.42, wspace=0.24)
    panels = []
    legend_handles = []
    legend_labels = []
    for index, family_id in enumerate(families):
        panel = fig.add_subplot(grid[index // columns, index % columns])
        panels.append(panel)
        family_rows = data.filter(pl.col("family_id") == family_id)
        x_ticks: list[int] = []
        for variant in variant_order:
            rows = family_rows.filter(pl.col(variant_column) == variant).sort(x_column)
            if rows.is_empty():
                continue
            xs = rows[x_column].to_list()
            raw_values = y_selector(rows)
            ys = [value_transform(value) if value_transform else value for value in raw_values]
            series_style = {
                key: value
                for key, value in line_style.items()
                if key not in {"color", "marker", "label"}
            }
            line, = panel.plot(
                xs,
                ys,
                color=HEURISTIC_COLORS.get(variant or "zero", "#475569"),
                marker=HEURISTIC_MARKERS.get(variant or "zero", "o"),
                label=heuristic_label(variant),
                **series_style,
            )
            if not legend_labels or heuristic_label(variant) not in legend_labels:
                legend_handles.append(line)
                legend_labels.append(heuristic_label(variant))
            x_ticks.extend(xs)
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel(xlabel)
        if index % columns == 0:
            panel.set_ylabel(ylabel)
        panel.set_xticks(sorted(set(x_ticks)))
        style_plot_axes(panel)
    for index in range(len(families), rows_count * columns):
        panel = fig.add_subplot(grid[index // columns, index % columns])
        panel.axis("off")
    _sync_panel_ylim(
        panels,
        minimum_upper=100.0 if ylabel.endswith("(%)") else 1.0,
    )
    legend = fig.legend(
        legend_handles,
        legend_labels,
        loc="lower center",
        bbox_to_anchor=(0.5, 0.02),
        ncol=max(2, len(legend_labels)),
        frameon=False,
        fontsize=8,
        title="Heuristic",
        title_fontsize=8,
    )
    style_legend(legend)
    fig.subplots_adjust(bottom=0.12)


def render_batman_bellman_transition_stability(ax, aggregates: pl.DataFrame) -> None:
    batman_bellman_families = [
        "batman-bellman-decay-window-pressure",
        "batman-bellman-partition-recovery",
        "batman-bellman-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-bellman") & pl.col("family_id").is_in(batman_bellman_families)
    )
    _render_single_series_family_sweep(
        ax,
        data,
        batman_bellman_families,
        "batman_bellman_stale_after_ticks",
        "stability_total_mean",
        "Stale ticks",
        "Stability score",
        ENGINE_SECTION_COLORS["batman-bellman"],
        OUTCOME_LINE_STYLE,
        annotation_column="batman_bellman_next_refresh_within_ticks",
        annotation_formatter=refresh_annotation,
    )


def render_batman_bellman_transition_loss(ax, aggregates: pl.DataFrame) -> None:
    batman_bellman_families = [
        "batman-bellman-decay-window-pressure",
        "batman-bellman-partition-recovery",
        "batman-bellman-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-bellman")
        & pl.col("family_id").is_in(batman_bellman_families)
        & pl.col("first_loss_round_mean").is_not_null()
    )
    _render_single_series_family_sweep(
        ax,
        data,
        batman_bellman_families,
        "batman_bellman_stale_after_ticks",
        "first_loss_round_mean",
        "Stale ticks",
        "First loss round",
        ENGINE_SECTION_COLORS["batman-bellman"],
        FRAGILITY_LINE_STYLE,
    )


def render_batman_classic_transition_stability(ax, aggregates: pl.DataFrame) -> None:
    batman_classic_families = [
        "batman-classic-decay-window-pressure",
        "batman-classic-partition-recovery",
        "batman-classic-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-classic")
        & pl.col("family_id").is_in(batman_classic_families)
    )
    _render_single_series_family_sweep(
        ax,
        data,
        batman_classic_families,
        "batman_classic_stale_after_ticks",
        "stability_total_mean",
        "Stale ticks",
        "Stability score",
        ENGINE_SECTION_COLORS["batman-classic"],
        OUTCOME_LINE_STYLE,
    )


def render_batman_classic_transition_loss(ax, aggregates: pl.DataFrame) -> None:
    batman_classic_families = [
        "batman-classic-decay-window-pressure",
        "batman-classic-partition-recovery",
        "batman-classic-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-classic")
        & pl.col("family_id").is_in(batman_classic_families)
        & pl.col("first_loss_round_mean").is_not_null()
    )
    _render_single_series_family_sweep(
        ax,
        data,
        batman_classic_families,
        "batman_classic_stale_after_ticks",
        "first_loss_round_mean",
        "Stale ticks",
        "First loss round",
        ENGINE_SECTION_COLORS["batman-classic"],
        FRAGILITY_LINE_STYLE,
    )


def render_babel_decay_stability(ax, aggregates: pl.DataFrame) -> None:
    babel_families = [
        "babel-decay-window-pressure",
        "babel-asymmetry-cost-penalty",
        "babel-partition-feasibility-recovery",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "babel") & pl.col("family_id").is_in(babel_families)
    )
    _render_single_series_family_sweep(
        ax,
        data,
        babel_families,
        "babel_stale_after_ticks",
        "stability_total_mean",
        "Stale ticks",
        "Stability score",
        ENGINE_SECTION_COLORS["babel"],
        OUTCOME_LINE_STYLE,
    )


def render_babel_decay_loss(ax, aggregates: pl.DataFrame) -> None:
    babel_families = [
        "babel-decay-window-pressure",
        "babel-asymmetry-cost-penalty",
        "babel-partition-feasibility-recovery",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "babel")
        & pl.col("family_id").is_in(babel_families)
        & pl.col("first_loss_round_mean").is_not_null()
    )
    _render_single_series_family_sweep(
        ax,
        data,
        babel_families,
        "babel_stale_after_ticks",
        "first_loss_round_mean",
        "Stale ticks",
        "First loss round",
        ENGINE_SECTION_COLORS["babel"],
        FRAGILITY_LINE_STYLE,
    )


def render_olsrv2_decay_stability(ax, aggregates: pl.DataFrame) -> None:
    olsrv2_families = [
        "olsrv2-topology-propagation-latency",
        "olsrv2-partition-recovery",
        "olsrv2-mpr-flooding-stability",
        "olsrv2-asymmetric-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "olsrv2") & pl.col("family_id").is_in(olsrv2_families)
    )
    _render_single_series_family_sweep(
        ax,
        data,
        olsrv2_families,
        "olsrv2_stale_after_ticks",
        "stability_total_mean",
        "Stale ticks",
        "Stability score",
        ENGINE_SECTION_COLORS["olsrv2"],
        OUTCOME_LINE_STYLE,
        columns=2,
    )


def render_olsrv2_decay_loss(ax, aggregates: pl.DataFrame) -> None:
    olsrv2_families = [
        "olsrv2-topology-propagation-latency",
        "olsrv2-partition-recovery",
        "olsrv2-mpr-flooding-stability",
        "olsrv2-asymmetric-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "olsrv2")
        & pl.col("family_id").is_in(olsrv2_families)
        & pl.col("first_loss_round_mean").is_not_null()
    )
    _render_single_series_family_sweep(
        ax,
        data,
        olsrv2_families,
        "olsrv2_stale_after_ticks",
        "first_loss_round_mean",
        "Stale ticks",
        "First loss round",
        ENGINE_SECTION_COLORS["olsrv2"],
        FRAGILITY_LINE_STYLE,
        columns=2,
    )


SCATTER_FIGURE_FAMILIES = [
    "scatter-connected-low-loss",
    "scatter-bridge-transition",
    "scatter-partial-observability-bridge",
    "scatter-corridor-continuity-uncertainty",
    "scatter-concurrent-mixed",
    "scatter-connected-high-loss",
    "scatter-medium-bridge-repair",
]

SCATTER_PROFILE_ORDER = ["balanced", "conservative", "degraded-network"]


def render_scatter_profile_route_presence(ax, aggregates: pl.DataFrame) -> None:
    present_families = [
        family
        for family in SCATTER_FIGURE_FAMILIES
        if not aggregates.filter(
            (pl.col("engine_family") == "scatter") & (pl.col("family_id") == family)
        ).is_empty()
    ]
    data = aggregates.filter(pl.col("engine_family") == "scatter")
    _render_single_series_family_sweep(
        ax,
        data,
        SCATTER_FIGURE_FAMILIES,
        "scatter_profile_id",
        "route_present_permille_mean",
        "Profile",
        "Active route presence (%)",
        ENGINE_SECTION_COLORS["scatter"],
        OUTCOME_LINE_STYLE,
        value_transform=_route_presence_percent,
        tick_formatter=lambda value: {
            "balanced": "balanced",
            "conservative": "conservative",
            "degraded-network": "degraded",
        }.get(str(value), str(value)),
        columns=4,
    )


def render_scatter_profile_startup(ax, aggregates: pl.DataFrame) -> None:
    data = aggregates.filter(pl.col("engine_family") == "scatter")
    _render_single_series_family_sweep(
        ax,
        data,
        SCATTER_FIGURE_FAMILIES,
        "scatter_profile_id",
        "first_materialization_round_mean",
        "Profile",
        "First materialization round",
        ENGINE_SECTION_COLORS["scatter"],
        FRAGILITY_LINE_STYLE,
        tick_formatter=lambda value: {
            "balanced": "balanced",
            "conservative": "conservative",
            "degraded-network": "degraded",
        }.get(str(value), str(value)),
        columns=4,
    )


def render_pathway_budget_route_presence(ax, aggregates: pl.DataFrame) -> None:
    pathway_families = [
        "pathway-search-budget-pressure",
        "pathway-high-fanout-budget-pressure",
        "pathway-bridge-failure-service",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "pathway") & pl.col("family_id").is_in(pathway_families)
    )
    _render_multi_series_family_sweep(
        ax,
        data,
        pathway_families,
        "pathway_query_budget",
        "pathway_heuristic_mode",
        ["zero", "hop-lower-bound"],
        "Query budget",
        "Active route presence (%)",
        OUTCOME_LINE_STYLE,
        y_selector=lambda rows: rows["route_present_permille_mean"].to_list(),
        value_transform=_route_presence_percent,
        columns=3,
    )


def render_pathway_budget_activation(ax, aggregates: pl.DataFrame) -> None:
    pathway_families = [
        "pathway-search-budget-pressure",
        "pathway-high-fanout-budget-pressure",
        "pathway-bridge-failure-service",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "pathway") & pl.col("family_id").is_in(pathway_families)
    )
    _render_multi_series_family_sweep(
        ax,
        data,
        pathway_families,
        "pathway_query_budget",
        "pathway_heuristic_mode",
        ["zero", "hop-lower-bound"],
        "Query budget",
        "Activation (%)",
        FRAGILITY_LINE_STYLE,
        y_selector=lambda rows: rows["activation_success_permille_mean"].to_list(),
        value_transform=_activation_percent,
        columns=3,
    )


def render_field_budget_route_presence(ax, aggregates: pl.DataFrame) -> None:
    field_families = [
        "field-partial-observability-bridge",
        "field-reconfiguration-recovery",
        "field-asymmetric-envelope-shift",
        "field-uncertain-service-fanout",
        "field-service-overlap-reselection",
        "field-service-freshness-inversion",
        "field-service-publication-pressure",
        "field-bridge-anti-entropy-continuity",
        "field-bootstrap-upgrade-window",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "field") & pl.col("family_id").is_in(field_families)
    )
    _render_multi_series_family_sweep(
        ax,
        data,
        field_families,
        "field_query_budget",
        "field_heuristic_mode",
        ["zero", "hop-lower-bound"],
        "Query budget",
        "Active route presence (%)",
        OUTCOME_LINE_STYLE,
        y_selector=lambda rows: rows["route_present_permille_mean"].to_list(),
        value_transform=_route_presence_percent,
        columns=3,
    )


def render_field_budget_reconfiguration(ax, aggregates: pl.DataFrame) -> None:
    field_families = [
        "field-partial-observability-bridge",
        "field-reconfiguration-recovery",
        "field-asymmetric-envelope-shift",
        "field-uncertain-service-fanout",
        "field-service-overlap-reselection",
        "field-service-freshness-inversion",
        "field-service-publication-pressure",
        "field-bridge-anti-entropy-continuity",
        "field-bootstrap-upgrade-window",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "field") & pl.col("family_id").is_in(field_families)
    )
    _render_multi_series_family_sweep(
        ax,
        data,
        field_families,
        "field_query_budget",
        "field_heuristic_mode",
        ["zero", "hop-lower-bound"],
        "Query budget",
        "Reconfiguration load",
        FRAGILITY_LINE_STYLE,
        y_selector=lambda rows: (
            rows["field_continuation_shift_count_mean"]
            + rows["field_search_reconfiguration_rounds_mean"]
        ).to_list(),
        columns=3,
    )


def render_comparison_summary(ax, aggregates: pl.DataFrame) -> None:
    data = aggregates.filter(pl.col("engine_family") == "comparison")
    if data.is_empty():
        return
    summary = (
        data.sort(["family_id", "route_present_permille_mean", "config_id"], descending=[False, True, False])
        .group_by("family_id")
        .agg(
            pl.first("dominant_engine").alias("dominant_engine"),
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
        .sort("family_id")
    )
    families = summary["family_id"].to_list()
    engine_columns = {
        "batman-bellman": "batman_bellman_selected_rounds_mean",
        "batman-classic": "batman_classic_selected_rounds_mean",
        "babel": "babel_selected_rounds_mean",
        "olsrv2": "olsrv2_selected_rounds_mean",
        "pathway": "pathway_selected_rounds_mean",
        "scatter": "scatter_selected_rounds_mean",
        "field": "field_selected_rounds_mean",
    }
    engine_sets = [
        engine_set for engine_set in ROUTE_VISIBLE_ENGINE_SET_ORDER if engine_set in engine_columns
    ]
    if not engine_sets:
        return
    y_positions = list(range(len(families)))
    ax.set_facecolor("#fbfdff")
    for spine in ax.spines.values():
        spine.set_color("#94a3b8")
        spine.set_linewidth(0.8)
    ax.tick_params(axis="y", colors=PLOT_TEXT_COLOR, labelcolor=PLOT_TEXT_COLOR)
    ax.tick_params(axis="x", length=0, labelbottom=False, colors=PLOT_TEXT_COLOR)
    ax.grid(False)
    for family_index, y_position in enumerate(y_positions):
        selected_rounds = {
            engine_set: int(summary[engine_columns[engine_set]][family_index] or 0)
            for engine_set in engine_sets
        }
        total_selected_rounds = sum(selected_rounds.values())
        dominant_engine = summary["dominant_engine"][family_index] or "none"
        dominant_rounds = selected_rounds.get(dominant_engine, 0)
        dominant_share = (
            (dominant_rounds * 100.0) / total_selected_rounds
            if total_selected_rounds > 0
            else 0.0
        )
        ax.barh(
            [y_position],
            [1.0],
            height=0.72,
            color=COMPARISON_ENGINE_COLORS.get(dominant_engine, "#94a3b8"),
            edgecolor="#334155",
            linewidth=0.8,
        )
        ax.text(
            0.04,
            y_position - 0.12,
            f"{dominant_engine} {dominant_share:.0f}%",
            ha="left",
            va="center",
            fontsize=8.5,
            color="#ffffff",
            fontweight="bold",
        )
    ax.set_yticks(y_positions)
    ax.set_yticklabels([break_tick_label(family) for family in families], fontsize=8.5)
    ax.set_xlabel("")
    ax.invert_yaxis()
    ax.set_xlim(0, 1)
    ax.set_xticks([])
    legend_handles = [
        plt.Rectangle(
            (0, 0),
            1,
            1,
            color=COMPARISON_ENGINE_COLORS.get(engine_set, "#94a3b8"),
            linewidth=0,
        )
        for engine_set in engine_sets
    ]
    legend_labels = engine_sets
    legend = ax.legend(
        legend_handles,
        legend_labels,
        loc="upper center",
        bbox_to_anchor=(0.5, -0.14),
        ncol=4,
        fontsize=8,
        frameon=False,
        handlelength=1.4,
        columnspacing=1.2,
    )
    style_legend(legend)


def render_head_to_head_route_presence(ax, aggregates: pl.DataFrame) -> None:
    data = aggregates.filter(pl.col("engine_family") == "head-to-head")
    if data.is_empty():
        return
    route_presence_column = (
        "route_present_total_window_permille_mean"
        if "route_present_total_window_permille_mean" in data.columns
        else "route_present_permille_mean"
    )
    families = data["family_id"].unique().sort().to_list()
    preferred_order = ROUTE_VISIBLE_ENGINE_SET_ORDER
    available = set(data["comparison_engine_set"].drop_nulls().unique().to_list())
    engine_sets = [engine_set for engine_set in preferred_order if engine_set in available]
    engine_sets.extend(sorted(available.difference(engine_sets)))
    if not engine_sets:
        return
    y_positions = list(range(len(families)))
    ax.set_facecolor("#fbfdff")
    for spine in ax.spines.values():
        spine.set_color("#94a3b8")
        spine.set_linewidth(0.8)
    ax.tick_params(axis="y", colors=PLOT_TEXT_COLOR, labelcolor=PLOT_TEXT_COLOR)
    ax.tick_params(axis="x", colors=PLOT_TEXT_COLOR, labelcolor=PLOT_TEXT_COLOR)
    ax.grid(axis="x", color="#cbd5e1", linewidth=0.7, alpha=0.7)
    for family_index, y_position in enumerate(y_positions):
        family_rows = data.filter(pl.col("family_id") == families[family_index]).sort(
            [route_presence_column, "comparison_engine_set"],
            descending=[True, False],
        )
        best_row = family_rows.head(1)
        best_engine = best_row["comparison_engine_set"].item() or "none"
        best_value = int(best_row[route_presence_column].item() or 0)
        distinct_lower = next(
            (
                int(value or 0)
                for value in family_rows[route_presence_column].to_list()
                if int(value or 0) < best_value
            ),
            0,
        )
        gap = max(best_value - distinct_lower, 0)
        best_percent = best_value / 10.0
        ax.barh(
            [y_position],
            [best_percent],
            height=0.72,
            color=HEAD_TO_HEAD_SET_COLORS.get(best_engine, "#94a3b8"),
            edgecolor="#334155",
            linewidth=0.8,
        )
        label_x = min(best_percent + 1.2, 98.5)
        label_color = PLOT_TEXT_COLOR
        label_ha = "left"
        if best_percent >= 28.0:
            label_x = best_percent - 1.6
            label_color = "#ffffff"
            label_ha = "right"
        ax.text(
            label_x,
            y_position - 0.12,
            f"{best_engine} {best_percent:.1f}%",
            ha=label_ha,
            va="center",
            fontsize=8.5,
            color=label_color,
            fontweight="bold",
        )
        ax.text(
            label_x,
            y_position + 0.16,
            f"next lower gap={gap / 10.0:.1f} pts",
            ha=label_ha,
            va="center",
            fontsize=7.6,
            color=label_color,
        )
    ax.set_yticks(y_positions)
    ax.set_yticklabels([break_tick_label(family) for family in families], fontsize=8.2)
    ax.set_xlabel("Total-window route presence (%)")
    ax.set_xlim(0, 100)
    ax.invert_yaxis()
    ax.set_xticks([0, 25, 50, 75, 100])
    legend_handles = [
        plt.Rectangle((0, 0), 1, 1, color=HEAD_TO_HEAD_SET_COLORS.get(e, "#94a3b8"), linewidth=0)
        for e in engine_sets
    ]
    legend_labels = [e for e in engine_sets]
    legend = ax.legend(
        legend_handles,
        legend_labels,
        loc="upper center",
        bbox_to_anchor=(0.5, -0.12),
        ncol=3,
        fontsize=8,
        frameon=False,
    )
    style_legend(legend)


def _style_grid_panel(panel) -> None:
    panel.set_facecolor("#fbfdff")
    for spine in panel.spines.values():
        spine.set_color("#94a3b8")
        spine.set_linewidth(0.8)
    panel.tick_params(axis="both", colors=PLOT_TEXT_COLOR, labelcolor=PLOT_TEXT_COLOR)
    panel.title.set_color(PLOT_TEXT_COLOR)


def _draw_metric_grid(
    panel,
    families: list[str],
    engine_sets: list[str],
    values: list[list[float | None]],
    title: str,
    cmap_name: str,
    reverse_scale: bool,
) -> None:
    _style_grid_panel(panel)
    panel.grid(False)
    present_values = [value for row in values for value in row if value is not None]
    low = min(present_values) if present_values else 0.0
    high = max(present_values) if present_values else 1.0
    scale = high - low if high != low else 1.0
    cmap = plt.get_cmap(cmap_name)
    for row_index, row in enumerate(values):
        for col_index, value in enumerate(row):
            if value is None:
                facecolor = "#e5e7eb"
                text = "–"
                text_color = PLOT_TEXT_COLOR
            else:
                normalized = (value - low) / scale
                if reverse_scale:
                    normalized = 1.0 - normalized
                facecolor = cmap(0.25 + 0.65 * normalized)
                text = str(int(value))
                text_color = "#ffffff" if normalized > 0.45 else PLOT_TEXT_COLOR
            panel.add_patch(
                plt.Rectangle(
                    (col_index, row_index),
                    1.0,
                    1.0,
                    facecolor=facecolor,
                    edgecolor="#cbd5e1",
                    linewidth=0.8,
                )
            )
            panel.text(
                col_index + 0.5,
                row_index + 0.5,
                text,
                ha="center",
                va="center",
                fontsize=7.5,
                color=text_color,
                fontweight="bold" if value is not None else "normal",
            )
    panel.set_xlim(0, len(engine_sets))
    panel.set_ylim(len(families), 0)
    panel.set_xticks([index + 0.5 for index in range(len(engine_sets))])
    panel.set_xticklabels([break_tick_label(engine) for engine in engine_sets], fontsize=7.3)
    panel.set_yticks([index + 0.5 for index in range(len(families))])
    panel.set_yticklabels([break_tick_label(family) for family in families], fontsize=7.6)
    panel.set_title(title, fontsize=9.2)


def render_head_to_head_timing_profile(ax, aggregates: pl.DataFrame) -> None:
    data = aggregates.filter(pl.col("engine_family") == "head-to-head")
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, 2, wspace=0.18)
    families = data["family_id"].unique().sort().to_list()
    available = set(data["comparison_engine_set"].drop_nulls().unique().to_list())
    engine_sets = [engine_set for engine_set in ROUTE_VISIBLE_ENGINE_SET_ORDER if engine_set in available]
    engine_sets.extend(sorted(available.difference(engine_sets)))
    materialization_values: list[list[float | None]] = []
    loss_values: list[list[float | None]] = []
    for family in families:
        family_rows = data.filter(pl.col("family_id") == family)
        materialization_row: list[float | None] = []
        loss_row: list[float | None] = []
        for engine_set in engine_sets:
            row = family_rows.filter(pl.col("comparison_engine_set") == engine_set).head(1)
            materialization_row.append(
                row["first_materialization_round_mean"].item()
                if not row.is_empty() and row["first_materialization_round_mean"].item() is not None
                else None
            )
            loss_row.append(
                row["first_loss_round_mean"].item()
                if not row.is_empty() and row["first_loss_round_mean"].item() is not None
                else None
            )
        materialization_values.append(materialization_row)
        loss_values.append(loss_row)
    left = fig.add_subplot(grid[0, 0])
    right = fig.add_subplot(grid[0, 1])
    _draw_metric_grid(
        left,
        families,
        engine_sets,
        materialization_values,
        "First materialization",
        "Blues",
        True,
    )
    _draw_metric_grid(
        right,
        families,
        engine_sets,
        loss_values,
        "First loss",
        "Greens",
        False,
    )
    left.set_ylabel("Regime")
    right.set_yticklabels([])


def render_recommended_engine_robustness(ax, data: pl.DataFrame) -> None:
    data = data.filter(
        pl.col("route_present_mean_permille").is_not_null()
        & pl.col("route_present_stddev_permille").is_not_null()
    )
    if data.is_empty():
        return
    style_plot_axes(ax)
    ax.set_xlabel("Route presence (permille)")
    ax.set_ylabel("Route variability (permille stddev)")
    ax.set_xlim(0, 1000)
    max_stddev = max(
        [
            float(value)
            for value in data["route_present_stddev_permille"].to_list()
            if value is not None
        ],
        default=1.0,
    )
    y_top = max(5.0, max_stddev + 20.0)
    ax.set_ylim(0, y_top)

    entries = []
    for row in data.iter_rows(named=True):
        entries.append({
            "engine": row["engine_family"],
            "x": float(row["route_present_mean_permille"]),
            "y": float(row["route_present_stddev_permille"]),
            "loss": row["first_loss_median"],
            "stress": row["max_sustained_stress_score"],
        })

    for e in entries:
        ax.scatter(
            [e["x"]],
            [e["y"]],
            color=COMPARISON_ENGINE_COLORS.get(e["engine"], "#94a3b8"),
            s=64,
            edgecolors="#334155",
            linewidths=0.8,
            zorder=3,
        )

    # Adjust label y positions so nearby labels don't overlap.
    label_y = [e["y"] for e in entries]
    min_sep = max(y_top * 0.14, 3.5)
    for _ in range(30):
        moved = False
        for i in range(len(entries)):
            for j in range(i + 1, len(entries)):
                if abs(entries[i]["x"] - entries[j]["x"]) > 250:
                    continue
                dy = label_y[j] - label_y[i]
                if abs(dy) < min_sep:
                    nudge = (min_sep - abs(dy)) / 2 + 0.1
                    if dy >= 0:
                        label_y[i] -= nudge
                        label_y[j] += nudge
                    else:
                        label_y[i] += nudge
                        label_y[j] -= nudge
                    moved = True
        if not moved:
            break
    for i in range(len(label_y)):
        label_y[i] = max(min_sep * 0.4, min(y_top - min_sep * 0.4, label_y[i]))

    for idx, e in enumerate(entries):
        ly = label_y[idx]
        ax.text(
            min(e["x"] + 16, 985),
            ly + 1.2,
            f"{e['engine']}",
            ha="left",
            va="bottom",
            fontsize=8.2,
            color=PLOT_TEXT_COLOR,
            fontweight="bold",
        )
        ax.text(
            min(e["x"] + 16, 985),
            ly - 1.4,
            f"stress={e['stress']} loss={int(e['loss']) if e['loss'] is not None else '–'}",
            ha="left",
            va="top",
            fontsize=7.2,
            color=PLOT_MUTED_TEXT_COLOR,
        )


def render_mixed_vs_standalone_divergence(ax, aggregates: pl.DataFrame) -> None:
    comparison_rows = (
        aggregates.filter(pl.col("engine_family") == "comparison")
        .sort(
            ["family_id", "route_present_total_window_permille_mean", "config_id"],
            descending=[False, True, False],
        )
        .group_by("family_id")
        .agg(
            pl.first("dominant_engine").alias("mixed_engine"),
            pl.first("route_present_total_window_permille_mean").alias("mixed_route_presence"),
        )
        .sort("family_id")
    )
    head_rows = aggregates.filter(pl.col("engine_family") == "head-to-head")
    if comparison_rows.is_empty() or head_rows.is_empty():
        return
    preferred_order = {engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)}
    rows: list[dict[str, object]] = []
    for row in comparison_rows.iter_rows(named=True):
        suffix = row["family_id"].replace("comparison-", "")
        family_id = f"head-to-head-{suffix}"
        family_head = head_rows.filter(pl.col("family_id") == family_id).with_columns(
            pl.col("comparison_engine_set")
            .replace_strict(preferred_order, default=len(preferred_order))
            .alias("engine_order")
        ).sort(
            ["route_present_total_window_permille_mean", "activation_success_permille_mean", "engine_order"],
            descending=[True, True, False],
        )
        if family_head.is_empty():
            continue
        best = family_head.head(1)
        best_engine = best["comparison_engine_set"].item()
        best_route_presence = best["route_present_total_window_permille_mean"].item()
        mixed_route_presence = row["mixed_route_presence"]
        rows.append(
            {
                "family_id": row["family_id"],
                "mixed_engine": row["mixed_engine"],
                "best_engine": best_engine,
                "delta": best_route_presence - mixed_route_presence,
                "mixed_route_presence": mixed_route_presence,
                "best_route_presence": best_route_presence,
            }
        )
    if not rows:
        return
    y_positions = list(range(len(rows)))
    deltas = [float(row["delta"]) / 10.0 for row in rows]
    colors = [
        HEAD_TO_HEAD_SET_COLORS.get(str(row["best_engine"]), "#94a3b8")
        for row in rows
    ]
    ax.set_facecolor("#fbfdff")
    for spine in ax.spines.values():
        spine.set_color("#94a3b8")
        spine.set_linewidth(0.8)
    ax.tick_params(axis="y", colors=PLOT_TEXT_COLOR, labelcolor=PLOT_TEXT_COLOR)
    ax.tick_params(axis="x", colors=PLOT_TEXT_COLOR, labelcolor=PLOT_TEXT_COLOR)
    ax.grid(axis="x", color="#cbd5e1", linewidth=0.7, alpha=0.7)
    bars = ax.barh(
        y_positions,
        deltas,
        color=colors,
        edgecolor="#334155",
        linewidth=0.7,
        height=0.62,
    )
    ax.axvline(0, color="#475569", linewidth=1.0)
    ax.set_yticks(y_positions)
    ax.set_yticklabels([break_tick_label(str(row["family_id"])) for row in rows], fontsize=8.2)
    ax.set_xlabel("Standalone advantage over mixed (pts)")
    ax.invert_yaxis()
    min_delta = min(deltas)
    max_delta = max(deltas)
    ax.set_xlim(min(-2.0, min_delta - 3.0), max(2.0, max_delta + 11.0))
    for bar, row in zip(bars, rows, strict=False):
        x = bar.get_width()
        y = bar.get_y() + bar.get_height() / 2
        label_x = x + (0.6 if x >= 0 else -0.6)
        label_ha = "left" if x >= 0 else "right"
        label_color = PLOT_TEXT_COLOR
        if abs(x) >= 10.0:
            label_x = x - 0.8 if x >= 0 else x + 0.8
            label_ha = "right" if x >= 0 else "left"
            label_color = "#ffffff"
        ax.text(
            label_x,
            y - 0.11,
            f"{row['mixed_engine']} -> {row['best_engine']}",
            ha=label_ha,
            va="center",
            fontsize=7.8,
            color=label_color,
            fontweight="bold",
        )
        ax.text(
            label_x,
            y + 0.15,
            f"{x:.1f} pts",
            ha=label_ha,
            va="center",
            fontsize=7.3,
            color=label_color,
        )


def render_diffusion_delivery_coverage(ax, diffusion_engine_comparison: pl.DataFrame) -> None:
    if diffusion_engine_comparison.is_empty():
        return
    fig = ax.figure
    fig.set_layout_engine(None)
    subplotspec = ax.get_subplotspec()
    ax.remove()
    families = [
        family
        for family in DIFFUSION_FIGURE_FAMILIES
        if not diffusion_engine_comparison.filter(pl.col("family_id") == family).is_empty()
    ]
    engine_sets = diffusion_engine_sets(diffusion_engine_comparison)
    cols = 3
    rows = (len(families) + cols - 1) // cols
    grid = subplotspec.subgridspec(rows, cols, wspace=0.16, hspace=0.25)
    panels = []
    y_positions = list(range(len(engine_sets)))
    for index, family in enumerate(families):
        panel = fig.add_subplot(grid[index // cols, index % cols])
        panels.append(panel)
        family_rows = diffusion_engine_comparison.filter(pl.col("family_id") == family)
        delivery = []
        coverage = []
        colors = []
        for engine_set in engine_sets:
            row = family_rows.filter(pl.col("config_id") == engine_set).head(1)
            delivery.append(
                row["delivery_probability_permille_mean"].item() if not row.is_empty() else 0
            )
            coverage.append(
                row["coverage_permille_mean"].item() if not row.is_empty() else 0
            )
            colors.append(diffusion_config_color(engine_set))
        bars = panel.barh(
            y_positions,
            delivery,
            color=colors,
            edgecolor="#334155",
            linewidth=0.6,
            height=0.6,
            zorder=2,
        )
        panel.scatter(
            coverage,
            y_positions,
            color="#111827",
            marker="o",
            s=18,
            zorder=4,
        )
        for bar, cov in zip(bars, coverage, strict=False):
            y = bar.get_y() + bar.get_height() / 2
            panel.plot([bar.get_width(), cov], [y, y], color="#111827", linewidth=0.9, alpha=0.8, zorder=3)
        panel.set_title(family_label(family), fontsize=9.2)
        panel.set_xlim(0, 1000)
        panel.set_xlabel("Delivery / coverage")
        panel.set_yticks(y_positions)
        if index % cols == 0:
            panel.set_ylabel("Engine set")
            panel.set_yticklabels(
                [break_tick_label(engine) for engine in engine_sets],
                fontsize=7.4,
            )
        else:
            panel.set_yticklabels([])
        panel.invert_yaxis()
        style_plot_axes(panel)
    for empty_index in range(len(families), rows * cols):
        empty = fig.add_subplot(grid[empty_index // cols, empty_index % cols])
        empty.axis("off")
    engine_handles = [
        plt.Rectangle((0, 0), 1, 1, color=diffusion_config_color(engine), linewidth=0)
        for engine in engine_sets
    ]
    engine_labels = [engine for engine in engine_sets]
    coverage_handle = plt.Line2D([0], [0], color="#111827", marker="o", linewidth=1.0, markersize=4.0)
    legend = panels[0].legend(
        [*engine_handles, coverage_handle],
        [*engine_labels, "coverage"],
        loc="lower center",
        bbox_to_anchor=(0.5, 0.0),
        ncol=min(4, max(2, len(engine_sets) // 3 + 1)),
        frameon=False,
        fontsize=8,
    )
    style_legend(legend)
    legend.remove()
    figure_legend = fig.legend(
        [*engine_handles, coverage_handle],
        [*engine_labels, "coverage"],
        loc="lower center",
        bbox_to_anchor=(0.5, 0.03),
        ncol=min(4, max(2, len(engine_sets) // 3 + 1)),
        frameon=False,
        fontsize=8,
    )
    style_legend(figure_legend)
    fig.subplots_adjust(bottom=0.16)


def render_diffusion_resource_boundedness(ax, diffusion_engine_comparison: pl.DataFrame) -> None:
    if diffusion_engine_comparison.is_empty():
        return
    fig = ax.figure
    fig.set_layout_engine(None)
    subplotspec = ax.get_subplotspec()
    ax.remove()
    families = [
        family
        for family in DIFFUSION_FIGURE_FAMILIES
        if not diffusion_engine_comparison.filter(pl.col("family_id") == family).is_empty()
    ]
    engine_sets = diffusion_engine_sets(diffusion_engine_comparison)
    cols = 3
    rows = (len(families) + cols - 1) // cols
    grid = subplotspec.subgridspec(rows, cols, wspace=0.16, hspace=0.25)
    panels = []
    max_tx = diffusion_engine_comparison["total_transmissions_mean"].max()
    y_positions = list(range(len(engine_sets)))
    for index, family in enumerate(families):
        panel = fig.add_subplot(grid[index // cols, index % cols])
        panels.append(panel)
        family_rows = diffusion_engine_comparison.filter(pl.col("family_id") == family)
        transmissions = []
        reproduction = []
        bounded_states = []
        colors = []
        for engine_set in engine_sets:
            row = family_rows.filter(pl.col("config_id") == engine_set).head(1)
            transmissions.append(row["total_transmissions_mean"].item() if not row.is_empty() else 0)
            reproduction.append(
                row["estimated_reproduction_permille_mean"].item() if not row.is_empty() else 0
            )
            bounded_states.append(row["bounded_state_mode"].item() if not row.is_empty() else "none")
            colors.append(diffusion_config_color(engine_set))
        bars = panel.barh(
            y_positions,
            transmissions,
            color=colors,
            edgecolor="#334155",
            linewidth=0.6,
            height=0.6,
            zorder=2,
        )
        panel.set_title(family_label(family), fontsize=9.2)
        panel.set_xlim(0, max(16, (max_tx or 0) + 8))
        panel.set_xlabel("Tx mean")
        panel.set_yticks(y_positions)
        if index % cols == 0:
            panel.set_ylabel("Engine set")
            panel.set_yticklabels(
                [break_tick_label(engine) for engine in engine_sets],
                fontsize=7.4,
            )
        else:
            panel.set_yticklabels([])
        panel.invert_yaxis()
        style_plot_axes(panel)
        x_max = max(16, (max_tx or 0) + 8)
        for bar, r_value, state in zip(bars, reproduction, bounded_states, strict=False):
            x = bar.get_width()
            y = bar.get_y() + bar.get_height() / 2
            panel.text(
                min(x + 0.8, x_max - 0.4),
                y - 0.12,
                f"R={r_value}",
                ha="left",
                va="center",
                fontsize=7.1,
                color=PLOT_TEXT_COLOR,
            )
            panel.text(
                min(x + 0.8, x_max - 0.4),
                y + 0.16,
                state,
                ha="left",
                va="center",
                fontsize=6.8,
                color=DIFFUSION_BOUND_STATE_COLORS.get(state, "#64748b"),
            )
    for empty_index in range(len(families), rows * cols):
        empty = fig.add_subplot(grid[empty_index // cols, empty_index % cols])
        empty.axis("off")
    legend_handles = [
        plt.Rectangle((0, 0), 1, 1, color=diffusion_config_color(engine), linewidth=0)
        for engine in engine_sets
    ]
    legend_labels = [engine for engine in engine_sets]
    legend = panels[0].legend(
        legend_handles,
        legend_labels,
        loc="lower center",
        bbox_to_anchor=(0.5, 0.0),
        ncol=min(4, max(2, len(engine_sets) // 3 + 1)),
        frameon=False,
        fontsize=8,
    )
    style_legend(legend)
    legend.remove()
    figure_legend = fig.legend(
        legend_handles,
        legend_labels,
        loc="lower center",
        bbox_to_anchor=(0.5, 0.03),
        ncol=min(4, max(2, len(engine_sets) // 3 + 1)),
        frameon=False,
        fontsize=8,
    )
    style_legend(figure_legend)
    fig.subplots_adjust(bottom=0.16)


def save_plot_artifact(
    report_dir: Path,
    key: str,
    render_fn,
    aggregates: pl.DataFrame,
) -> None:
    width_inches, height_inches = PLOT_SPECS[key]
    fig, ax = plt.subplots(figsize=(width_inches, height_inches), layout="constrained")
    render_fn(ax, aggregates)
    fig.savefig(report_dir / f"{key}.svg", format="svg")
    fig.savefig(report_dir / f"{key}.pdf", format="pdf")
    fig.savefig(report_dir / f"{key}.png", format="png", dpi=300)
    plt.close(fig)
