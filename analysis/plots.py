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
    PLOT_SPECS,
)

PLOT_TEXT_COLOR = "#2f3437"
PLOT_MUTED_TEXT_COLOR = "#4b5563"
DIFFUSION_ENGINE_SETS = [
    "batman-bellman",
    "batman-classic",
    "babel",
    "pathway",
    "field",
    "pathway-batman-bellman",
]
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


def render_batman_bellman_transition_stability(ax, aggregates: pl.DataFrame) -> None:
    batman_bellman_families = [
        "batman-bellman-decay-window-pressure",
        "batman-bellman-partition-recovery",
        "batman-bellman-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-bellman") & pl.col("family_id").is_in(batman_bellman_families)
    )
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, 3, wspace=0.18)
    for index, family_id in enumerate(batman_bellman_families):
        panel = fig.add_subplot(grid[0, index])
        rows = data.filter(pl.col("family_id") == family_id).sort("batman_bellman_stale_after_ticks")
        xs = rows["batman_bellman_stale_after_ticks"].to_list()
        ys = rows["stability_total_mean"].to_list()
        panel.plot(
            xs,
            ys,
            marker="o",
            color=BATMAN_BELLMAN_FAMILY_COLORS.get(family_id, "#0072B2"),
            linewidth=1.9,
            markersize=5.5,
            markeredgecolor="white",
            markeredgewidth=0.7,
            zorder=3,
        )
        for x, y, refresh in zip(
            xs, ys, rows["batman_bellman_next_refresh_within_ticks"].to_list(), strict=False
        ):
            panel.annotate(
                refresh_annotation(refresh),
                (x, y),
                textcoords="offset points",
                xytext=(0, 6),
                ha="center",
                fontsize=7.2,
                color=PLOT_MUTED_TEXT_COLOR,
            )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Stale ticks")
        if index == 0:
            panel.set_ylabel("Stability")
        panel.set_xticks(xs)
        panel.set_ylim(0, 2500)
        style_plot_axes(panel)


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
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, 3, wspace=0.18)
    panels = []
    for index, family_id in enumerate(batman_bellman_families):
        panel = fig.add_subplot(grid[0, index])
        panels.append(panel)
        rows = data.filter(pl.col("family_id") == family_id).sort("batman_bellman_stale_after_ticks")
        xs = rows["batman_bellman_stale_after_ticks"].to_list()
        ys = rows["first_loss_round_mean"].to_list()
        panel.plot(
            xs,
            ys,
            marker="o",
            color=BATMAN_BELLMAN_FAMILY_COLORS.get(family_id, "#0072B2"),
            linewidth=1.9,
            markersize=5.5,
            markeredgecolor="white",
            markeredgewidth=0.7,
            zorder=3,
        )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Stale ticks")
        if index == 0:
            panel.set_ylabel("First loss")
        panel.set_xticks(xs)
        style_plot_axes(panel)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


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
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, 3, wspace=0.18)
    panels = []
    for index, family_id in enumerate(batman_classic_families):
        panel = fig.add_subplot(grid[0, index])
        panels.append(panel)
        rows = data.filter(pl.col("family_id") == family_id).sort(
            "batman_classic_stale_after_ticks"
        )
        xs = rows["batman_classic_stale_after_ticks"].to_list()
        ys = rows["stability_total_mean"].to_list()
        panel.plot(
            xs,
            ys,
            marker="o",
            color=BATMAN_CLASSIC_FAMILY_COLORS.get(family_id, "#56B4E9"),
            linewidth=1.9,
            markersize=5.5,
            markeredgecolor="white",
            markeredgewidth=0.7,
            zorder=3,
        )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Stale ticks")
        if index == 0:
            panel.set_ylabel("Stability")
        panel.set_xticks(xs)
        style_plot_axes(panel)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


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
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, 3, wspace=0.18)
    panels = []
    for index, family_id in enumerate(batman_classic_families):
        panel = fig.add_subplot(grid[0, index])
        panels.append(panel)
        rows = data.filter(pl.col("family_id") == family_id).sort(
            "batman_classic_stale_after_ticks"
        )
        xs = rows["batman_classic_stale_after_ticks"].to_list()
        ys = rows["first_loss_round_mean"].to_list()
        panel.plot(
            xs,
            ys,
            marker="s",
            color=BATMAN_CLASSIC_FAMILY_COLORS.get(family_id, "#56B4E9"),
            linewidth=1.6,
            markersize=5.5,
            markeredgecolor="white",
            markeredgewidth=0.7,
            linestyle="dashed",
            zorder=3,
        )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Stale ticks")
        if index == 0:
            panel.set_ylabel("First loss round")
        panel.set_xticks(xs)
        style_plot_axes(panel)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


def render_babel_decay_stability(ax, aggregates: pl.DataFrame) -> None:
    babel_families = [
        "babel-decay-window-pressure",
        "babel-asymmetry-cost-penalty",
        "babel-partition-feasibility-recovery",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "babel") & pl.col("family_id").is_in(babel_families)
    )
    if data.is_empty():
        return
    present_families = [
        fid for fid in babel_families if not data.filter(pl.col("family_id") == fid).is_empty()
    ]
    if not present_families:
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, len(present_families), wspace=0.18)
    panels = []
    for index, family_id in enumerate(present_families):
        panel = fig.add_subplot(grid[0, index])
        panels.append(panel)
        rows = data.filter(pl.col("family_id") == family_id).sort("babel_stale_after_ticks")
        xs = rows["babel_stale_after_ticks"].to_list()
        ys = rows["stability_total_mean"].to_list()
        panel.plot(
            xs,
            ys,
            marker="o",
            color=BABEL_FAMILY_COLORS.get(family_id, "#882255"),
            linewidth=2.3,
            markersize=6.2,
            markeredgecolor="white",
            markeredgewidth=0.9,
            zorder=3,
        )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Stale ticks")
        if index == 0:
            panel.set_ylabel("Stability")
        panel.set_xticks(xs)
        style_plot_axes(panel)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


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
    if data.is_empty():
        return
    present_families = [
        fid for fid in babel_families if not data.filter(pl.col("family_id") == fid).is_empty()
    ]
    if not present_families:
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, len(present_families), wspace=0.18)
    panels = []
    for index, family_id in enumerate(present_families):
        panel = fig.add_subplot(grid[0, index])
        panels.append(panel)
        rows = data.filter(pl.col("family_id") == family_id).sort("babel_stale_after_ticks")
        xs = rows["babel_stale_after_ticks"].to_list()
        ys = rows["first_loss_round_mean"].to_list()
        panel.plot(
            xs,
            ys,
            marker="s",
            color=BABEL_FAMILY_COLORS.get(family_id, "#882255"),
            linewidth=2.0,
            markersize=6.0,
            markeredgecolor="white",
            markeredgewidth=0.9,
            linestyle="dashed",
            zorder=3,
        )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Stale ticks")
        if index == 0:
            panel.set_ylabel("First loss round")
        panel.set_xticks(xs)
        style_plot_axes(panel)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


def render_pathway_budget_route_presence(ax, aggregates: pl.DataFrame) -> None:
    pathway_families = [
        "pathway-search-budget-pressure",
        "pathway-high-fanout-budget-pressure",
        "pathway-bridge-failure-service",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "pathway") & pl.col("family_id").is_in(pathway_families)
    )
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, 3, wspace=0.18)
    heuristic_colors = {"zero": "#0072B2", "hop-lower-bound": "#D55E00"}
    heuristic_markers = {"zero": "o", "hop-lower-bound": "s"}
    panels = []
    for index, family_id in enumerate(pathway_families):
        panel = fig.add_subplot(grid[0, index])
        panels.append(panel)
        family_rows = data.filter(pl.col("family_id") == family_id)
        heuristics = (
            family_rows.select("pathway_heuristic_mode")
            .unique()
            .sort("pathway_heuristic_mode")
            .to_series()
            .to_list()
        )
        for heuristic in heuristics:
            rows = family_rows.filter(pl.col("pathway_heuristic_mode") == heuristic).sort(
                "pathway_query_budget"
            )
            panel.plot(
                rows["pathway_query_budget"].to_list(),
                rows["route_present_permille_mean"].to_list(),
                color=heuristic_colors.get(heuristic or "zero", "#475569"),
                marker=heuristic_markers.get(heuristic or "zero", "o"),
                linestyle="-",
                linewidth=1.8,
                markersize=5.5,
                markeredgecolor="white",
                markeredgewidth=0.7,
                label=heuristic_label(heuristic),
                zorder=3,
            )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Query budget")
        if index == 0:
            panel.set_ylabel("Route presence")
        panel.set_xticks(sorted(set(family_rows["pathway_query_budget"].to_list())))
        style_plot_axes(panel)
        legend = panel.legend(fontsize="x-small", title="Heuristic", title_fontsize="x-small")
        style_legend(legend)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


def render_pathway_budget_activation(ax, aggregates: pl.DataFrame) -> None:
    pathway_families = [
        "pathway-search-budget-pressure",
        "pathway-high-fanout-budget-pressure",
        "pathway-bridge-failure-service",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "pathway") & pl.col("family_id").is_in(pathway_families)
    )
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    grid = subplotspec.subgridspec(1, 3, wspace=0.18)
    heuristic_colors = {"zero": "#0072B2", "hop-lower-bound": "#D55E00"}
    heuristic_markers = {"zero": "o", "hop-lower-bound": "s"}
    panels = []
    for index, family_id in enumerate(pathway_families):
        panel = fig.add_subplot(grid[0, index])
        panels.append(panel)
        family_rows = data.filter(pl.col("family_id") == family_id)
        heuristics = (
            family_rows.select("pathway_heuristic_mode")
            .unique()
            .sort("pathway_heuristic_mode")
            .to_series()
            .to_list()
        )
        for heuristic in heuristics:
            rows = family_rows.filter(pl.col("pathway_heuristic_mode") == heuristic).sort(
                "pathway_query_budget"
            )
            panel.plot(
                rows["pathway_query_budget"].to_list(),
                rows["activation_success_permille_mean"].to_list(),
                color=heuristic_colors.get(heuristic or "zero", "#475569"),
                marker=heuristic_markers.get(heuristic or "zero", "o"),
                linestyle="-",
                linewidth=1.8,
                markersize=5.5,
                markeredgecolor="white",
                markeredgewidth=0.7,
                label=heuristic_label(heuristic),
                zorder=3,
            )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Query budget")
        if index == 0:
            panel.set_ylabel("Activation")
        panel.set_xticks(sorted(set(family_rows["pathway_query_budget"].to_list())))
        style_plot_axes(panel)
        legend = panel.legend(fontsize="x-small", title="Heuristic", title_fontsize="x-small")
        style_legend(legend)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


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
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    columns = 3
    rows_count = (len(field_families) + columns - 1) // columns
    grid = subplotspec.subgridspec(rows_count, columns, hspace=0.42, wspace=0.16)
    heuristic_colors = {"zero": "#0072B2", "hop-lower-bound": "#D55E00"}
    heuristic_markers = {"zero": "o", "hop-lower-bound": "s"}
    panels = []
    for index, family_id in enumerate(field_families):
        panel = fig.add_subplot(grid[index // columns, index % columns])
        panels.append(panel)
        family_rows = data.filter(pl.col("family_id") == family_id)
        heuristics = (
            family_rows.select("field_heuristic_mode")
            .unique()
            .sort("field_heuristic_mode")
            .to_series()
            .to_list()
        )
        for heuristic in heuristics:
            rows = family_rows.filter(pl.col("field_heuristic_mode") == heuristic).sort(
                "field_query_budget"
            )
            panel.plot(
                rows["field_query_budget"].to_list(),
                rows["route_present_permille_mean"].to_list(),
                color=heuristic_colors.get(heuristic or "zero", "#475569"),
                marker=heuristic_markers.get(heuristic or "zero", "o"),
                linestyle="-",
                linewidth=1.8,
                markersize=5.5,
                markeredgecolor="white",
                markeredgewidth=0.7,
                label=heuristic_label(heuristic),
                zorder=3,
            )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Query budget")
        if index % columns == 0:
            panel.set_ylabel("Route presence")
        panel.set_xticks(sorted(set(family_rows["field_query_budget"].to_list())))
        style_plot_axes(panel)
        legend = panel.legend(fontsize="x-small", title="Heuristic", title_fontsize="x-small")
        style_legend(legend)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


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
    if data.is_empty():
        return
    fig = ax.figure
    subplotspec = ax.get_subplotspec()
    ax.remove()
    columns = 3
    rows_count = (len(field_families) + columns - 1) // columns
    grid = subplotspec.subgridspec(rows_count, columns, hspace=0.42, wspace=0.16)
    heuristic_colors = {"zero": "#0072B2", "hop-lower-bound": "#D55E00"}
    heuristic_markers = {"zero": "o", "hop-lower-bound": "s"}
    panels = []
    for index, family_id in enumerate(field_families):
        panel = fig.add_subplot(grid[index // columns, index % columns])
        panels.append(panel)
        family_rows = data.filter(pl.col("family_id") == family_id)
        heuristics = (
            family_rows.select("field_heuristic_mode")
            .unique()
            .sort("field_heuristic_mode")
            .to_series()
            .to_list()
        )
        for heuristic in heuristics:
            rows = family_rows.filter(pl.col("field_heuristic_mode") == heuristic).sort(
                "field_query_budget"
            )
            reconfiguration_load = (
                rows["field_continuation_shift_count_mean"]
                + rows["field_search_reconfiguration_rounds_mean"]
            ).to_list()
            panel.plot(
                rows["field_query_budget"].to_list(),
                reconfiguration_load,
                color=heuristic_colors.get(heuristic or "zero", "#475569"),
                marker=heuristic_markers.get(heuristic or "zero", "o"),
                linestyle="-",
                linewidth=1.8,
                markersize=5.5,
                markeredgecolor="white",
                markeredgewidth=0.7,
                label=heuristic_label(heuristic),
                zorder=3,
            )
        panel.set_title(family_label(family_id), fontsize=9.5)
        panel.set_xlabel("Query budget")
        if index % columns == 0:
            panel.set_ylabel("Reconfiguration load")
        panel.set_xticks(sorted(set(family_rows["field_query_budget"].to_list())))
        style_plot_axes(panel)
        legend = panel.legend(fontsize="x-small", title="Heuristic", title_fontsize="x-small")
        style_legend(legend)
    y_max = max(panel.get_ylim()[1] for panel in panels)
    for panel in panels:
        panel.set_ylim(0, y_max)


def render_comparison_summary(ax, aggregates: pl.DataFrame) -> None:
    data = aggregates.filter(pl.col("engine_family") == "comparison")
    if data.is_empty():
        return
    summary = (
        data.with_columns(pl.col("dominant_engine").fill_null("none"))
        .sort(["family_id", "route_present_permille_mean"], descending=[False, True])
        .group_by("family_id")
        .agg(
            pl.first("dominant_engine").alias("dominant_engine"),
            pl.first("route_present_permille_mean").alias("route_present_permille_mean"),
            pl.first("activation_success_permille_mean").alias("activation_success_permille_mean"),
            pl.first("stress_score").alias("stress_score"),
        )
        .sort("family_id")
    )
    families = summary["family_id"].to_list()
    engines = summary["dominant_engine"].to_list()
    route_presence = summary["route_present_permille_mean"].to_list()
    activation = summary["activation_success_permille_mean"].to_list()
    stress = summary["stress_score"].to_list()
    y_positions = list(range(len(families)))
    bars = ax.barh(
        y_positions,
        route_presence,
        color=[COMPARISON_ENGINE_COLORS.get(engine, "#999999") for engine in engines],
        edgecolor="#334155",
        linewidth=0.8,
        height=0.64,
    )
    ax.set_yticks(y_positions)
    ax.set_yticklabels([break_tick_label(family) for family in families], fontsize=8.5)
    ax.set_xlabel("Route presence")
    ax.invert_yaxis()
    style_plot_axes(ax)
    ax.set_xlim(0, 1000)
    for bar, engine, act, stress_level in zip(
        bars, engines, activation, stress, strict=False
    ):
        y = bar.get_y() + bar.get_height() / 2
        x = bar.get_width()
        note = f"`{engine}` act={act} stress={stress_level}"
        if x == 0:
            ax.scatter(
                [8],
                [y],
                marker="x",
                color=PLOT_MUTED_TEXT_COLOR,
                s=28,
                linewidths=1.2,
                zorder=4,
            )
            ax.text(
                18,
                y,
                "no route",
                va="center",
                ha="left",
                fontsize=8.2,
                color=PLOT_MUTED_TEXT_COLOR,
            )
        else:
            ax.text(
                min(x + 12, 985),
                y,
                note,
                va="center",
                ha="left",
                fontsize=8.2,
                color=PLOT_TEXT_COLOR,
            )
    legend_handles = [
        plt.Rectangle((0, 0), 1, 1, color=color, linewidth=0)
        for engine, color in COMPARISON_ENGINE_COLORS.items()
        if engine != "none"
    ]
    legend_labels = [
        f"`{engine}`" for engine in COMPARISON_ENGINE_COLORS if engine != "none"
    ]
    legend = ax.legend(
        legend_handles,
        legend_labels,
        loc="upper center",
        bbox_to_anchor=(0.5, -0.12),
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
    families = data["family_id"].unique().sort().to_list()
    engine_sets = [
        "batman-bellman",
        "batman-classic",
        "babel",
        "pathway",
        "field",
        "pathway-batman-bellman",
    ]
    y_positions = list(range(len(families)))
    height = 0.12
    offsets = [-2.5 * height, -1.5 * height, -0.5 * height, 0.5 * height, 1.5 * height, 2.5 * height]
    for engine_set, offset in zip(engine_sets, offsets, strict=False):
        rows = data.filter(pl.col("comparison_engine_set") == engine_set)
        values = []
        for family in families:
            family_row = rows.filter(pl.col("family_id") == family).head(1)
            values.append(
                family_row["route_present_permille_mean"].item() if not family_row.is_empty() else 0
            )
        ax.barh(
            [position + offset for position in y_positions],
            values,
            height=height,
            label=f"`{engine_set}`",
            color=HEAD_TO_HEAD_SET_COLORS.get(engine_set, "#94a3b8"),
            edgecolor="#334155",
            linewidth=0.7,
        )
    ax.set_yticks(y_positions)
    ax.set_yticklabels([break_tick_label(family) for family in families], fontsize=8.2)
    ax.set_xlabel("Route presence")
    ax.set_xlim(0, 1000)
    ax.invert_yaxis()
    style_plot_axes(ax)
    legend_handles = [
        plt.Rectangle((0, 0), 1, 1, color=HEAD_TO_HEAD_SET_COLORS.get(e, "#94a3b8"), linewidth=0)
        for e in engine_sets
    ]
    legend_labels = [f"`{e}`" for e in engine_sets]
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
    cols = 3
    rows = (len(families) + cols - 1) // cols
    grid = subplotspec.subgridspec(rows, cols, wspace=0.16, hspace=0.32)
    panels = []
    y_positions = list(range(len(DIFFUSION_ENGINE_SETS)))
    for index, family in enumerate(families):
        panel = fig.add_subplot(grid[index // cols, index % cols])
        panels.append(panel)
        family_rows = diffusion_engine_comparison.filter(pl.col("family_id") == family)
        delivery = []
        coverage = []
        colors = []
        for engine_set in DIFFUSION_ENGINE_SETS:
            row = family_rows.filter(pl.col("config_id") == engine_set).head(1)
            delivery.append(
                row["delivery_probability_permille_mean"].item() if not row.is_empty() else 0
            )
            coverage.append(
                row["coverage_permille_mean"].item() if not row.is_empty() else 0
            )
            colors.append(HEAD_TO_HEAD_SET_COLORS.get(engine_set, "#64748b"))
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
                [break_tick_label(engine) for engine in DIFFUSION_ENGINE_SETS],
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
        plt.Rectangle((0, 0), 1, 1, color=HEAD_TO_HEAD_SET_COLORS.get(engine, "#64748b"), linewidth=0)
        for engine in DIFFUSION_ENGINE_SETS
    ]
    engine_labels = [f"`{engine}`" for engine in DIFFUSION_ENGINE_SETS]
    coverage_handle = plt.Line2D([0], [0], color="#111827", marker="o", linewidth=1.0, markersize=4.0)
    legend = panels[0].legend(
        [*engine_handles, coverage_handle],
        [*engine_labels, "coverage"],
        loc="lower center",
        bbox_to_anchor=(0.5, 0.0),
        ncol=3,
        frameon=False,
        fontsize=8,
    )
    style_legend(legend)
    legend.remove()
    figure_legend = fig.legend(
        [*engine_handles, coverage_handle],
        [*engine_labels, "coverage"],
        loc="lower center",
        bbox_to_anchor=(0.5, 0.005),
        ncol=3,
        frameon=False,
        fontsize=8,
    )
    style_legend(figure_legend)
    fig.subplots_adjust(bottom=0.22)


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
    cols = 3
    rows = (len(families) + cols - 1) // cols
    grid = subplotspec.subgridspec(rows, cols, wspace=0.16, hspace=0.32)
    panels = []
    max_tx = diffusion_engine_comparison["total_transmissions_mean"].max()
    y_positions = list(range(len(DIFFUSION_ENGINE_SETS)))
    for index, family in enumerate(families):
        panel = fig.add_subplot(grid[index // cols, index % cols])
        panels.append(panel)
        family_rows = diffusion_engine_comparison.filter(pl.col("family_id") == family)
        transmissions = []
        reproduction = []
        bounded_states = []
        colors = []
        for engine_set in DIFFUSION_ENGINE_SETS:
            row = family_rows.filter(pl.col("config_id") == engine_set).head(1)
            transmissions.append(row["total_transmissions_mean"].item() if not row.is_empty() else 0)
            reproduction.append(
                row["estimated_reproduction_permille_mean"].item() if not row.is_empty() else 0
            )
            bounded_states.append(row["bounded_state_mode"].item() if not row.is_empty() else "none")
            colors.append(HEAD_TO_HEAD_SET_COLORS.get(engine_set, "#64748b"))
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
                [break_tick_label(engine) for engine in DIFFUSION_ENGINE_SETS],
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
        plt.Rectangle((0, 0), 1, 1, color=HEAD_TO_HEAD_SET_COLORS.get(engine, "#64748b"), linewidth=0)
        for engine in DIFFUSION_ENGINE_SETS
    ]
    legend_labels = [f"`{engine}`" for engine in DIFFUSION_ENGINE_SETS]
    legend = panels[0].legend(
        legend_handles,
        legend_labels,
        loc="lower center",
        bbox_to_anchor=(0.5, 0.0),
        ncol=3,
        frameon=False,
        fontsize=8,
    )
    style_legend(legend)
    legend.remove()
    figure_legend = fig.legend(
        legend_handles,
        legend_labels,
        loc="lower center",
        bbox_to_anchor=(0.5, 0.005),
        ncol=3,
        frameon=False,
        fontsize=8,
    )
    style_legend(figure_legend)
    fig.subplots_adjust(bottom=0.22)


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
