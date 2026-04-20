"""Altair render functions for each analysis figure and a shared save helper."""

from __future__ import annotations

import math
from collections import defaultdict
from pathlib import Path

import altair as alt
import polars as pl
from reportlab.graphics import renderPDF
from svglib.svglib import svg2rlg

from .constants import (
    COMPARISON_ENGINE_COLORS,
    DIFFUSION_BOUND_STATE_COLORS,
    ENGINE_COLORS,
    HEAD_TO_HEAD_SET_COLORS,
    HEURISTIC_COLORS,
    LARGE_POPULATION_STATE_ORDER,
    PLOT_SPECS,
    ROUTING_FITNESS_CROSSOVER_FAMILIES,
    ROUTING_FITNESS_MULTI_FLOW_FAMILIES,
    ROUTING_FITNESS_STALE_FAMILIES,
    ROUTE_VISIBLE_ENGINE_SET_ORDER,
)

alt.data_transformers.disable_max_rows()

PLOT_TEXT_COLOR = "#2f3437"
PLOT_MUTED_TEXT_COLOR = "#4b5563"
PLOT_BACKGROUND_COLOR = "#fbfdff"
PLOT_BORDER_COLOR = "#94a3b8"
PLOT_GRID_COLOR = "#cbd5e1"
PLOT_MISSING_COLOR = "#e5e7eb"
PLOT_FONT = "Helvetica"

PIXELS_PER_INCH = 72

OUTCOME_SERIES_STYLE = {
    "shape": "circle",
    "stroke_dash": [1, 0],
}

FRAGILITY_SERIES_STYLE = {
    "shape": "square",
    "stroke_dash": [7, 4],
}

DIFFUSION_FIGURE_FAMILIES = [
    "diffusion-partitioned-clusters",
    "diffusion-sparse-long-delay",
    "diffusion-adversarial-observation",
    "diffusion-bridge-drought",
    "diffusion-energy-starved-relay",
    "diffusion-congestion-cascade",
]

SCATTER_FIGURE_FAMILIES = [
    "scatter-connected-low-loss",
    "scatter-bridge-transition",
    "scatter-partial-observability-bridge",
    "scatter-corridor-continuity-uncertainty",
    "scatter-concurrent-mixed",
    "scatter-connected-high-loss",
    "scatter-medium-bridge-repair",
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


def wrapped_family_label(family_id: str) -> str:
    words = family_label(family_id).split()
    if len(words) <= 2:
        return " ".join(words)
    midpoint = max(1, len(words) // 2)
    return " ".join(words[:midpoint]) + "\n" + " ".join(words[midpoint:])


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


def engine_display_label(engine_id: str | None) -> str:
    canonical = {
        "batman-bellman": "BATMAN Bellman",
        "batman-classic": "BATMAN Classic",
        "babel": "Babel",
        "olsrv2": "OLSRv2",
        "pathway": "Pathway",
        "scatter": "Scatter",
        "field": "Field",
        "pathway-batman-bellman": "Pathway + BATMAN Bellman",
        "tie": "Tie",
        "none": "None",
        "field-continuity": "Field continuity",
        "field-scarcity": "Field scarcity",
        "field-congestion": "Field congestion",
        "field-privacy": "Field privacy",
        "transition-tight": "Transition tight",
        "transition-balanced": "Transition balanced",
        "transition-bridge-biased": "Transition bridge-biased",
        "transition-broad": "Transition broad",
    }
    if engine_id in canonical:
        return canonical[engine_id]
    if engine_id and engine_id.startswith("field-") and "-search-" in engine_id:
        tokens = engine_id.split("-")
        regime = tokens[1]
        search_id = tokens[-1]
        return f"Field {regime} s{search_id}"
    if not engine_id:
        return "None"
    return " ".join(part.upper() if part == "mpr" else part.capitalize() for part in engine_id.split("-"))


def compact_engine_label(engine_id: str | None) -> str:
    canonical = {
        "batman-bellman": "BB",
        "batman-classic": "BC",
        "babel": "Babel",
        "olsrv2": "OLSRv2",
        "pathway": "Pathway",
        "scatter": "Scatter",
        "field": "Field",
        "pathway-batman-bellman": "Pathway\n+ BB",
        "tie": "Tie",
        "none": "None",
        "field-continuity": "Field\ncontinuity",
        "field-scarcity": "Field\nscarcity",
        "field-congestion": "Field\ncongestion",
        "field-privacy": "Field\nprivacy",
    }
    if engine_id in canonical:
        return canonical[engine_id]
    if engine_id and engine_id.startswith("field-") and "-search-" in engine_id:
        tokens = engine_id.split("-")
        regime = tokens[1].replace("continuity", "cont.")
        search_id = tokens[-1]
        return f"{regime}\ns{search_id}"
    if not engine_id:
        return "None"
    return break_tick_label(engine_display_label(engine_id).replace(" ", "-"))


def refresh_annotation(refresh: int | None) -> str:
    return f"r{refresh}" if refresh is not None else ""


def diffusion_config_label(config_id: str) -> str:
    return compact_engine_label(config_id)


def diffusion_config_color(config_id: str) -> str:
    if config_id in HEAD_TO_HEAD_SET_COLORS:
        return HEAD_TO_HEAD_SET_COLORS[config_id]
    if config_id.startswith("field-"):
        return {
            "field": ENGINE_COLORS["field"],
            "field-continuity": "#D986AA",
            "field-scarcity": "#B85A85",
            "field-congestion": "#9E4671",
            "field-privacy": "#7E3259",
        }.get(config_id, ENGINE_COLORS["field"])
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


def _plot_pixels(key: str) -> tuple[int, int]:
    width_inches, height_inches = PLOT_SPECS[key]
    return int(width_inches * PIXELS_PER_INCH), int(height_inches * PIXELS_PER_INCH)


def _panel_dimensions(
    total_width: int,
    total_height: int,
    panel_count: int,
    columns: int,
    *,
    legend: bool = False,
) -> tuple[int, int]:
    rows = max(1, math.ceil(panel_count / max(1, columns)))
    horizontal_spacing = 18
    vertical_spacing = 20
    legend_height = 58 if legend else 14
    panel_width = max(
        118,
        int((total_width - horizontal_spacing * max(0, columns - 1)) / max(1, columns)),
    )
    panel_height = max(
        108,
        int(
            (
                total_height
                - vertical_spacing * max(0, rows - 1)
                - legend_height
            )
            / rows
        ),
    )
    return panel_width, panel_height


def _configure_chart(chart: alt.TopLevelMixin) -> alt.TopLevelMixin:
    return (
        chart.configure(
            background=PLOT_BACKGROUND_COLOR,
            padding=8,
        )
        .configure_view(
            fill=PLOT_BACKGROUND_COLOR,
            stroke=PLOT_BORDER_COLOR,
            strokeWidth=1,
        )
        .configure_axis(
            labelColor=PLOT_TEXT_COLOR,
            titleColor=PLOT_TEXT_COLOR,
            domainColor=PLOT_BORDER_COLOR,
            tickColor=PLOT_BORDER_COLOR,
            gridColor=PLOT_GRID_COLOR,
            gridOpacity=0.35,
            labelFont=PLOT_FONT,
            titleFont=PLOT_FONT,
            labelFontSize=10,
            titleFontSize=10,
        )
        .configure_header(
            labelColor=PLOT_TEXT_COLOR,
            titleColor=PLOT_TEXT_COLOR,
            labelFont=PLOT_FONT,
            titleFont=PLOT_FONT,
            labelFontSize=11,
            titleFontSize=11,
        )
        .configure_legend(
            labelColor=PLOT_TEXT_COLOR,
            titleColor=PLOT_TEXT_COLOR,
            labelFont=PLOT_FONT,
            titleFont=PLOT_FONT,
            labelFontSize=10,
            titleFontSize=10,
            orient="bottom",
            direction="horizontal",
            symbolStrokeWidth=1,
        )
        .configure_title(
            color=PLOT_TEXT_COLOR,
            font=PLOT_FONT,
            fontSize=13,
            anchor="start",
        )
        .configure_concat(spacing=16)
        .configure_facet(spacing=18)
    )


def _placeholder_chart(width: int, height: int, message: str) -> alt.Chart:
    chart = (
        alt.Chart(
            alt.InlineData(values=[{"message": message, "x": 0.5, "y": 0.5}])
        )
        .mark_text(
            color=PLOT_MUTED_TEXT_COLOR,
            font=PLOT_FONT,
            fontSize=12,
            align="center",
            baseline="middle",
        )
        .encode(
            x=alt.X("x:Q", axis=None),
            y=alt.Y("y:Q", axis=None),
            text="message:N",
        )
        .properties(width=width, height=height)
    )
    return _configure_chart(chart)


def _engine_color_scale(
    domain: list[str],
    palette: dict[str, str],
    *,
    field: str = "engine_key:N",
    field_domain: list[str] | None = None,
    legend_title: str | None = "Engine",
) -> alt.Color:
    return alt.Color(
        field,
        scale=alt.Scale(
            domain=field_domain or domain,
            range=[palette[key] for key in domain],
        ),
        legend=alt.Legend(title=legend_title),
    )


def _heuristic_color_scale(domain: list[str]) -> alt.Color:
    color_map = {
        "zero": HEURISTIC_COLORS["zero"],
        "hop-lb": HEURISTIC_COLORS["hop-lower-bound"],
    }
    return alt.Color(
        "variant_label:N",
        scale=alt.Scale(domain=domain, range=[color_map[label] for label in domain]),
        legend=alt.Legend(title="Heuristic"),
    )


def _shape_scale(domain: list[str]) -> alt.Scale:
    shapes = {
        "zero": "circle",
        "hop-lb": "diamond",
    }
    return alt.Scale(domain=domain, range=[shapes[label] for label in domain])


def _route_presence_percent(value: int | float | None) -> float | None:
    if value is None:
        return None
    return float(value) / 10000.0


def _activation_percent(value: int | float | None) -> float | None:
    if value is None:
        return None
    return float(value) / 10.0


def _single_series_family_sweep(
    data: pl.DataFrame,
    families: list[str],
    total_width: int,
    total_height: int,
    *,
    x_column: str,
    y_column: str,
    xlabel: str,
    ylabel: str,
    color: str,
    series_style: dict[str, object],
    value_transform=None,
    tick_formatter=None,
    annotation_column: str | None = None,
    annotation_formatter=None,
    columns: int | None = None,
) -> alt.TopLevelMixin | None:
    present_families = [
        family for family in families if not data.filter(pl.col("family_id") == family).is_empty()
    ]
    if not present_families:
        return None

    rows: list[dict[str, object]] = []
    x_order: list[str] = []
    x_is_numeric = True
    for family_id in present_families:
        family_rows = data.filter(pl.col("family_id") == family_id).sort(x_column)
        for row in family_rows.iter_rows(named=True):
            raw_y = row.get(y_column)
            if raw_y is None:
                continue
            raw_x = row.get(x_column)
            x_is_numeric = x_is_numeric and isinstance(raw_x, (int, float))
            x_label = tick_formatter(raw_x) if tick_formatter else str(raw_x)
            if x_label not in x_order:
                x_order.append(x_label)
            annotation_label = ""
            if annotation_column is not None and annotation_formatter is not None:
                annotation_label = annotation_formatter(row.get(annotation_column))
            rows.append(
                {
                    "family_label": family_label(family_id),
                    "x_value": raw_x,
                    "x_label": x_label,
                    "y_value": value_transform(raw_y) if value_transform else raw_y,
                    "annotation_label": annotation_label,
                }
            )
    if not rows:
        return None

    y_top = max(
        100.0 if ylabel.endswith("(%)") else 1.0,
        max(float(row["y_value"]) for row in rows if row["y_value"] is not None),
    )
    columns_count = columns or len(present_families)
    panel_width, panel_height = _panel_dimensions(
        total_width, total_height, len(present_families), columns_count
    )
    dataset = alt.InlineData(values=rows)
    x_encoding = (
        alt.X(
            "x_value:Q",
            title=xlabel,
            axis=alt.Axis(labelAngle=0, tickMinStep=1),
        )
        if x_is_numeric
        else alt.X(
            "x_label:N",
            title=xlabel,
            sort=x_order,
            axis=alt.Axis(labelAngle=0),
        )
    )
    y_encoding = alt.Y(
        "y_value:Q",
        title=ylabel,
        scale=alt.Scale(domain=[0, y_top]),
    )
    tooltip = [
        alt.Tooltip("family_label:N", title="Family"),
        alt.Tooltip("x_label:N", title=xlabel),
        alt.Tooltip("y_value:Q", title=ylabel, format=".2f"),
    ]
    base = alt.Chart(dataset).encode(x=x_encoding, y=y_encoding, tooltip=tooltip)
    line = base.mark_line(
        color=color,
        strokeWidth=2,
        strokeDash=series_style["stroke_dash"],
        point=alt.OverlayMarkDef(
            filled=True,
            fill=color,
            shape=series_style["shape"],
            size=70,
            stroke="white",
            strokeWidth=1,
        ),
    )
    layers: list[alt.Chart] = [line]
    if annotation_column is not None and annotation_formatter is not None:
        layers.append(
            base.transform_filter("datum.annotation_label != ''").mark_text(
                color=PLOT_MUTED_TEXT_COLOR,
                font=PLOT_FONT,
                fontSize=9,
                dy=-10,
            ).encode(text="annotation_label:N")
        )
    chart = (
        alt.layer(*layers)
        .properties(width=panel_width, height=panel_height)
        .facet(
            facet=alt.Facet(
                "family_label:N",
                sort=[family_label(family) for family in present_families],
                header=alt.Header(title=None, labelOrient="bottom"),
            ),
            columns=columns_count,
        )
        .resolve_scale(y="shared")
    )
    return _configure_chart(chart)


def _multi_series_family_sweep(
    data: pl.DataFrame,
    families: list[str],
    total_width: int,
    total_height: int,
    *,
    x_column: str,
    variant_column: str,
    variant_order: list[str],
    xlabel: str,
    ylabel: str,
    series_style: dict[str, object],
    y_selector,
    value_transform=None,
    columns: int = 3,
) -> alt.TopLevelMixin | None:
    present_families = [
        family for family in families if not data.filter(pl.col("family_id") == family).is_empty()
    ]
    if not present_families:
        return None

    rows: list[dict[str, object]] = []
    variant_labels = [heuristic_label(variant) for variant in variant_order]
    for family_id in present_families:
        family_rows = data.filter(pl.col("family_id") == family_id)
        for variant in variant_order:
            rows_for_variant = family_rows.filter(pl.col(variant_column) == variant).sort(x_column)
            if rows_for_variant.is_empty():
                continue
            values = y_selector(rows_for_variant)
            for raw_x, raw_y in zip(rows_for_variant[x_column].to_list(), values, strict=False):
                if raw_y is None:
                    continue
                rows.append(
                    {
                        "family_label": family_label(family_id),
                        "x_value": raw_x,
                        "variant_label": heuristic_label(variant),
                        "y_value": value_transform(raw_y) if value_transform else raw_y,
                    }
                )
    if not rows:
        return None

    y_top = max(
        100.0 if ylabel.endswith("(%)") else 1.0,
        max(float(row["y_value"]) for row in rows if row["y_value"] is not None),
    )
    panel_width, panel_height = _panel_dimensions(
        total_width, total_height, len(present_families), columns, legend=True
    )
    dataset = alt.InlineData(values=rows)
    base = alt.Chart(dataset).encode(
        x=alt.X("x_value:Q", title=xlabel, axis=alt.Axis(labelAngle=0, tickMinStep=1)),
        y=alt.Y("y_value:Q", title=ylabel, scale=alt.Scale(domain=[0, y_top])),
        tooltip=[
            alt.Tooltip("family_label:N", title="Family"),
            alt.Tooltip("variant_label:N", title="Heuristic"),
            alt.Tooltip("x_value:Q", title=xlabel),
            alt.Tooltip("y_value:Q", title=ylabel, format=".2f"),
        ],
    )
    line = base.mark_line(
        strokeWidth=2,
        strokeDash=series_style["stroke_dash"],
    ).encode(
        color=_heuristic_color_scale(variant_labels),
    )
    points = base.mark_point(
        filled=True,
        size=70,
        stroke="white",
        strokeWidth=1,
        opacity=1,
    ).encode(
        color=alt.Color(
            "variant_label:N",
            scale=alt.Scale(
                domain=variant_labels,
                range=[
                    HEURISTIC_COLORS["zero"],
                    HEURISTIC_COLORS["hop-lower-bound"],
                ],
            ),
            legend=None,
        ),
        shape=alt.Shape(
            "variant_label:N",
            scale=_shape_scale(variant_labels),
            legend=None,
        ),
    )
    chart = (
        alt.layer(line, points)
        .properties(width=panel_width, height=panel_height)
        .facet(
            facet=alt.Facet(
                "family_label:N",
                sort=[family_label(family) for family in present_families],
                header=alt.Header(title=None, labelOrient="bottom"),
            ),
            columns=columns,
        )
        .resolve_scale(y="shared")
    )
    return _configure_chart(chart)


def render_batman_bellman_transition_stability(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "batman-bellman-decay-window-pressure",
        "batman-bellman-partition-recovery",
        "batman-bellman-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-bellman")
        & pl.col("family_id").is_in(families)
    )
    return _single_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="batman_bellman_stale_after_ticks",
        y_column="stability_total_mean",
        xlabel="Stale ticks",
        ylabel="Stability score",
        color=ENGINE_COLORS["batman-bellman"],
        series_style=OUTCOME_SERIES_STYLE,
        annotation_column="batman_bellman_next_refresh_within_ticks",
        annotation_formatter=refresh_annotation,
    )


def render_batman_bellman_transition_loss(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "batman-bellman-decay-window-pressure",
        "batman-bellman-partition-recovery",
        "batman-bellman-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-bellman")
        & pl.col("family_id").is_in(families)
        & pl.col("first_loss_round_mean").is_not_null()
    )
    return _single_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="batman_bellman_stale_after_ticks",
        y_column="first_loss_round_mean",
        xlabel="Stale ticks",
        ylabel="First loss round",
        color=ENGINE_COLORS["batman-bellman"],
        series_style=FRAGILITY_SERIES_STYLE,
    )


def render_batman_classic_transition_stability(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "batman-classic-decay-window-pressure",
        "batman-classic-partition-recovery",
        "batman-classic-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-classic")
        & pl.col("family_id").is_in(families)
    )
    return _single_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="batman_classic_stale_after_ticks",
        y_column="stability_total_mean",
        xlabel="Stale ticks",
        ylabel="Stability score",
        color=ENGINE_COLORS["batman-classic"],
        series_style=OUTCOME_SERIES_STYLE,
    )


def render_batman_classic_transition_loss(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "batman-classic-decay-window-pressure",
        "batman-classic-partition-recovery",
        "batman-classic-asymmetry-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "batman-classic")
        & pl.col("family_id").is_in(families)
        & pl.col("first_loss_round_mean").is_not_null()
    )
    return _single_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="batman_classic_stale_after_ticks",
        y_column="first_loss_round_mean",
        xlabel="Stale ticks",
        ylabel="First loss round",
        color=ENGINE_COLORS["batman-classic"],
        series_style=FRAGILITY_SERIES_STYLE,
    )


def render_babel_decay_stability(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "babel-decay-window-pressure",
        "babel-asymmetry-cost-penalty",
        "babel-partition-feasibility-recovery",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "babel") & pl.col("family_id").is_in(families)
    )
    return _single_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="babel_stale_after_ticks",
        y_column="stability_total_mean",
        xlabel="Stale ticks",
        ylabel="Stability score",
        color=ENGINE_COLORS["babel"],
        series_style=OUTCOME_SERIES_STYLE,
    )


def render_babel_decay_loss(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "babel-decay-window-pressure",
        "babel-asymmetry-cost-penalty",
        "babel-partition-feasibility-recovery",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "babel")
        & pl.col("family_id").is_in(families)
        & pl.col("first_loss_round_mean").is_not_null()
    )
    return _single_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="babel_stale_after_ticks",
        y_column="first_loss_round_mean",
        xlabel="Stale ticks",
        ylabel="First loss round",
        color=ENGINE_COLORS["babel"],
        series_style=FRAGILITY_SERIES_STYLE,
    )


def render_olsrv2_decay_stability(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "olsrv2-topology-propagation-latency",
        "olsrv2-partition-recovery",
        "olsrv2-mpr-flooding-stability",
        "olsrv2-asymmetric-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "olsrv2") & pl.col("family_id").is_in(families)
    )
    return _single_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="olsrv2_stale_after_ticks",
        y_column="stability_total_mean",
        xlabel="Stale ticks",
        ylabel="Stability score",
        color=ENGINE_COLORS["olsrv2"],
        series_style=OUTCOME_SERIES_STYLE,
        columns=2,
    )


def render_olsrv2_decay_loss(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "olsrv2-topology-propagation-latency",
        "olsrv2-partition-recovery",
        "olsrv2-mpr-flooding-stability",
        "olsrv2-asymmetric-relink-transition",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "olsrv2")
        & pl.col("family_id").is_in(families)
        & pl.col("first_loss_round_mean").is_not_null()
    )
    return _single_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="olsrv2_stale_after_ticks",
        y_column="first_loss_round_mean",
        xlabel="Stale ticks",
        ylabel="First loss round",
        color=ENGINE_COLORS["olsrv2"],
        series_style=FRAGILITY_SERIES_STYLE,
        columns=2,
    )


def render_scatter_profile_route_presence(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    data = aggregates.filter(pl.col("engine_family") == "scatter")
    return _single_series_family_sweep(
        data,
        SCATTER_FIGURE_FAMILIES,
        total_width,
        total_height,
        x_column="scatter_profile_id",
        y_column="route_present_permille_mean",
        xlabel="Profile",
        ylabel="Active route presence (%)",
        color=ENGINE_COLORS["scatter"],
        series_style=OUTCOME_SERIES_STYLE,
        value_transform=_route_presence_percent,
        tick_formatter=lambda value: {
            "balanced": "balanced",
            "conservative": "conservative",
            "degraded-network": "degraded",
        }.get(str(value), str(value)),
        columns=4,
    )


def render_scatter_profile_startup(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    data = aggregates.filter(pl.col("engine_family") == "scatter")
    return _single_series_family_sweep(
        data,
        SCATTER_FIGURE_FAMILIES,
        total_width,
        total_height,
        x_column="scatter_profile_id",
        y_column="first_materialization_round_mean",
        xlabel="Profile",
        ylabel="First materialization round",
        color=ENGINE_COLORS["scatter"],
        series_style=FRAGILITY_SERIES_STYLE,
        tick_formatter=lambda value: {
            "balanced": "balanced",
            "conservative": "conservative",
            "degraded-network": "degraded",
        }.get(str(value), str(value)),
        columns=4,
    )


def render_pathway_budget_route_presence(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "pathway-search-budget-pressure",
        "pathway-high-fanout-budget-pressure",
        "pathway-bridge-failure-service",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "pathway") & pl.col("family_id").is_in(families)
    )
    return _multi_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="pathway_query_budget",
        variant_column="pathway_heuristic_mode",
        variant_order=["zero", "hop-lower-bound"],
        xlabel="Query budget",
        ylabel="Active route presence (%)",
        series_style=OUTCOME_SERIES_STYLE,
        y_selector=lambda rows: rows["route_present_permille_mean"].to_list(),
        value_transform=_route_presence_percent,
    )


def render_pathway_budget_activation(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
        "pathway-search-budget-pressure",
        "pathway-high-fanout-budget-pressure",
        "pathway-bridge-failure-service",
    ]
    data = aggregates.filter(
        (pl.col("engine_family") == "pathway") & pl.col("family_id").is_in(families)
    )
    return _multi_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="pathway_query_budget",
        variant_column="pathway_heuristic_mode",
        variant_order=["zero", "hop-lower-bound"],
        xlabel="Query budget",
        ylabel="Activation (%)",
        series_style=FRAGILITY_SERIES_STYLE,
        y_selector=lambda rows: rows["activation_success_permille_mean"].to_list(),
        value_transform=_activation_percent,
    )


def render_field_budget_route_presence(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
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
        (pl.col("engine_family") == "field") & pl.col("family_id").is_in(families)
    )
    return _multi_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="field_query_budget",
        variant_column="field_heuristic_mode",
        variant_order=["zero", "hop-lower-bound"],
        xlabel="Query budget",
        ylabel="Active route presence (%)",
        series_style=OUTCOME_SERIES_STYLE,
        y_selector=lambda rows: rows["route_present_permille_mean"].to_list(),
        value_transform=_route_presence_percent,
    )


def render_field_budget_reconfiguration(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    families = [
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
        (pl.col("engine_family") == "field") & pl.col("family_id").is_in(families)
    )
    return _multi_series_family_sweep(
        data,
        families,
        total_width,
        total_height,
        x_column="field_query_budget",
        variant_column="field_heuristic_mode",
        variant_order=["zero", "hop-lower-bound"],
        xlabel="Query budget",
        ylabel="Reconfiguration load",
        series_style=FRAGILITY_SERIES_STYLE,
        y_selector=lambda rows: (
            rows["field_continuation_shift_count_mean"]
            + rows["field_search_reconfiguration_rounds_mean"]
        ).to_list(),
    )


def render_comparison_summary(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    data = aggregates.filter(pl.col("engine_family") == "comparison")
    if data.is_empty():
        return None
    summary = (
        data.sort(
            ["family_id", "route_present_permille_mean", "config_id"],
            descending=[False, True, False],
        )
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
    engine_columns = {
        "batman-bellman": "batman_bellman_selected_rounds_mean",
        "batman-classic": "batman_classic_selected_rounds_mean",
        "babel": "babel_selected_rounds_mean",
        "olsrv2": "olsrv2_selected_rounds_mean",
        "pathway": "pathway_selected_rounds_mean",
        "scatter": "scatter_selected_rounds_mean",
        "field": "field_selected_rounds_mean",
    }
    rows: list[dict[str, object]] = []
    present_engines: set[str] = set()
    for row in summary.iter_rows(named=True):
        selected_rounds = {
            engine: int(row.get(column) or 0)
            for engine, column in engine_columns.items()
        }
        total_selected_rounds = sum(selected_rounds.values())
        dominant_engine = row["dominant_engine"] or "none"
        dominant_share = (
            selected_rounds.get(dominant_engine, 0) * 100.0 / total_selected_rounds
            if total_selected_rounds > 0
            else 0.0
        )
        engine_key = dominant_engine if dominant_engine in COMPARISON_ENGINE_COLORS else "none"
        present_engines.add(engine_key)
        rows.append(
            {
                "family_label": wrapped_family_label(str(row["family_id"])),
                "engine_key": engine_key,
                "engine_legend": engine_display_label(engine_key),
                "share": dominant_share,
                "label_inside": dominant_share >= 82.0,
                "label_x": dominant_share - 1.8 if dominant_share >= 82.0 else min(dominant_share + 1.4, 103.0),
                "label": f"{compact_engine_label(dominant_engine)} {dominant_share:.0f}%",
            }
        )
    if not rows:
        return None
    engine_domain = [
        engine for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER if engine in present_engines
    ]
    if "none" in present_engines:
        engine_domain.append("none")
    if "tie" in present_engines:
        engine_domain.append("tie")
    legend_domain = [engine_display_label(engine) for engine in engine_domain]

    dataset = alt.InlineData(values=rows)
    height = max(180, min(total_height, 30 * len(rows) + 48))
    base = alt.Chart(dataset).encode(
        y=alt.Y("family_label:N", sort=[row["family_label"] for row in rows], title=None)
    )
    chart = alt.layer(
        base.mark_bar(height=22, cornerRadiusEnd=2).encode(
            x=alt.X(
                "share:Q",
                title="Leader share of active-route rounds (%)",
                scale=alt.Scale(domain=[0, 106]),
                axis=alt.Axis(values=[0, 25, 50, 75, 100]),
            ),
            color=_engine_color_scale(
                engine_domain,
                COMPARISON_ENGINE_COLORS,
                field="engine_legend:N",
                field_domain=legend_domain,
                legend_title="Engine",
            ),
        ),
        base.transform_filter("datum.label_inside == false").mark_text(
            align="left",
            baseline="middle",
            dx=6,
            color=PLOT_TEXT_COLOR,
            font=PLOT_FONT,
            fontSize=9,
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 106])),
            text="label:N",
        ),
        base.transform_filter("datum.label_inside == true").mark_text(
            align="right",
            baseline="middle",
            dx=-6,
            color="#ffffff",
            font=PLOT_FONT,
            fontSize=9,
            fontWeight="bold",
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 106])),
            text="label:N",
        ),
    ).properties(width=total_width - 18, height=height)
    return _configure_chart(chart)


def render_head_to_head_route_presence(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    data = aggregates.filter(pl.col("engine_family") == "head-to-head")
    if data.is_empty():
        return None
    route_presence_column = (
        "route_present_total_window_permille_mean"
        if "route_present_total_window_permille_mean" in data.columns
        else "route_present_permille_mean"
    )
    rows: list[dict[str, object]] = []
    engine_domain: list[str] = []
    for family in data["family_id"].unique().sort().to_list():
        family_rows = data.filter(pl.col("family_id") == family).sort(
            [route_presence_column, "comparison_engine_set"],
            descending=[True, False],
        )
        best_row = family_rows.head(1)
        best_engine = best_row["comparison_engine_set"].item() or "none"
        best_value = float(best_row[route_presence_column].item() or 0)
        distinct_lower = next(
            (
                float(value or 0)
                for value in family_rows[route_presence_column].to_list()
                if float(value or 0) < best_value
            ),
            0.0,
        )
        best_percent = best_value / 10.0
        gap_percent = max(best_value - distinct_lower, 0.0) / 10.0
        engine_key = best_engine if best_engine in HEAD_TO_HEAD_SET_COLORS else "none"
        if engine_key not in engine_domain:
            engine_domain.append(engine_key)
        rows.append(
            {
                "family_label": wrapped_family_label(family),
                "engine_key": engine_key,
                "engine_legend": engine_display_label(engine_key),
                "best_percent": best_percent,
                "label_inside": best_percent >= 80.0,
                "label_x": best_percent - 1.6 if best_percent >= 80.0 else min(best_percent + 1.5, 103.5),
                "engine_label": f"{compact_engine_label(best_engine)} {best_percent:.1f}%",
                "gap_label": f"next lower gap={gap_percent:.1f} pts",
            }
        )
    if not rows:
        return None
    legend_domain = [engine_display_label(engine) for engine in engine_domain]

    dataset = alt.InlineData(values=rows)
    height = max(180, min(total_height, 34 * len(rows) + 40))
    base = alt.Chart(dataset).encode(
        y=alt.Y("family_label:N", sort=[row["family_label"] for row in rows], title=None)
    )
    chart = alt.layer(
        base.mark_bar(height=24, cornerRadiusEnd=2).encode(
            x=alt.X(
                "best_percent:Q",
                title="Total-window route presence (%)",
                scale=alt.Scale(domain=[0, 106]),
                axis=alt.Axis(values=[0, 25, 50, 75, 100]),
            ),
            color=_engine_color_scale(
                engine_domain,
                HEAD_TO_HEAD_SET_COLORS,
                field="engine_legend:N",
                field_domain=legend_domain,
                legend_title="Engine set",
            ),
        ),
        base.transform_filter("datum.label_inside == false").mark_text(
            align="left",
            baseline="bottom",
            dx=6,
            dy=-2,
            color=PLOT_TEXT_COLOR,
            font=PLOT_FONT,
            fontSize=9,
            fontWeight="bold",
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 106])),
            text="engine_label:N",
        ),
        base.transform_filter("datum.label_inside == false").mark_text(
            align="left",
            baseline="top",
            dx=6,
            dy=2,
            color=PLOT_MUTED_TEXT_COLOR,
            font=PLOT_FONT,
            fontSize=8,
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 106])),
            text="gap_label:N",
        ),
        base.transform_filter("datum.label_inside == true").mark_text(
            align="right",
            baseline="bottom",
            dx=-6,
            dy=-2,
            color="#ffffff",
            font=PLOT_FONT,
            fontSize=9,
            fontWeight="bold",
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 106])),
            text="engine_label:N",
        ),
        base.transform_filter("datum.label_inside == true").mark_text(
            align="right",
            baseline="top",
            dx=-6,
            dy=2,
            color="#f8fafc",
            font=PLOT_FONT,
            fontSize=8,
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 106])),
            text="gap_label:N",
        ),
    ).properties(width=total_width - 18, height=height)
    return _configure_chart(chart)


def _metric_heatmap(
    rows: list[dict[str, object]],
    total_width: int,
    total_height: int,
    *,
    title: str,
    engine_domain: list[str],
    family_domain: list[str],
    range_colors: list[str],
    reverse_scale: bool,
    hide_y_axis: bool = False,
) -> alt.Chart:
    valid_values = [
        float(row["value"])
        for row in rows
        if row["value"] is not None
    ]
    low = min(valid_values) if valid_values else 0.0
    high = max(valid_values) if valid_values else 1.0
    if high == low:
        high = low + 1.0
    if reverse_scale:
        range_colors = list(reversed(range_colors))
    dataset = alt.InlineData(values=rows)
    base = alt.Chart(dataset).encode(
        x=alt.X(
            "engine_label:N",
            sort=engine_domain,
            title=None,
            axis=alt.Axis(labelAngle=0, labelLimit=120),
        ),
        y=alt.Y(
            "family_label:N",
            sort=family_domain,
            title="Regime" if not hide_y_axis else None,
            axis=None if hide_y_axis else alt.Axis(labelLimit=200),
        ),
    )
    chart = alt.layer(
        base.mark_rect(color=PLOT_MISSING_COLOR),
        base.transform_filter("datum.value != null").mark_rect().encode(
            color=alt.Color(
                "value:Q",
                scale=alt.Scale(domain=[low, high], range=range_colors),
                legend=None,
            )
        ),
        base.mark_text(
            color=PLOT_TEXT_COLOR,
            font=PLOT_FONT,
            fontSize=8,
        ).encode(text="display:N"),
    ).properties(width=total_width, height=total_height, title=title)
    return chart


def render_head_to_head_timing_profile(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    data = aggregates.filter(pl.col("engine_family") == "head-to-head")
    if data.is_empty():
        return None
    families = [
        wrapped_family_label(family)
        for family in data["family_id"].unique().sort().to_list()
    ]
    available = set(data["comparison_engine_set"].drop_nulls().unique().to_list())
    engine_sets = [engine for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER if engine in available]
    engine_sets.extend(sorted(available.difference(engine_sets)))
    engine_labels = [compact_engine_label(engine) for engine in engine_sets]
    materialization_rows: list[dict[str, object]] = []
    loss_rows: list[dict[str, object]] = []
    for family_id in data["family_id"].unique().sort().to_list():
        family_rows = data.filter(pl.col("family_id") == family_id)
        family_display = wrapped_family_label(family_id)
        for engine_set, engine_label in zip(engine_sets, engine_labels, strict=False):
            row = family_rows.filter(pl.col("comparison_engine_set") == engine_set).head(1)
            materialization = (
                row["first_materialization_round_mean"].item()
                if not row.is_empty()
                else None
            )
            loss = row["first_loss_round_mean"].item() if not row.is_empty() else None
            materialization_rows.append(
                {
                    "family_label": family_display,
                    "engine_label": engine_label,
                    "value": materialization,
                    "display": (
                        str(int(materialization))
                        if materialization is not None
                        else "–"
                    ),
                }
            )
            loss_rows.append(
                {
                    "family_label": family_display,
                    "engine_label": engine_label,
                    "value": loss,
                    "display": str(int(loss)) if loss is not None else "–",
                }
            )
    panel_width = max(240, total_width // 2 - 24)
    panel_height = max(180, total_height - 24)
    left = _metric_heatmap(
        materialization_rows,
        panel_width,
        panel_height,
        title="First materialization",
        engine_domain=engine_labels,
        family_domain=families,
        range_colors=["#dbeafe", "#1d4ed8"],
        reverse_scale=True,
    )
    right = _metric_heatmap(
        loss_rows,
        panel_width,
        panel_height,
        title="First loss",
        engine_domain=engine_labels,
        family_domain=families,
        range_colors=["#dcfce7", "#15803d"],
        reverse_scale=False,
        hide_y_axis=True,
    )
    return _configure_chart(left | right)


def render_recommended_engine_robustness(
    data: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    filtered = data.filter(
        pl.col("engine_family").is_in(ROUTE_VISIBLE_ENGINE_SET_ORDER)
        & pl.col("route_present_mean_permille").is_not_null()
        & pl.col("route_present_stddev_permille").is_not_null()
    )
    if filtered.is_empty():
        return None

    entries: list[dict[str, object]] = []
    for row in filtered.iter_rows(named=True):
        entries.append(
            {
                "engine_key": row["engine_family"],
                "engine_label": engine_display_label(row["engine_family"]),
                "x": float(row["route_present_mean_permille"]),
                "y": float(row["route_present_stddev_permille"]),
                "loss": row["first_loss_median"],
                "stress": row["max_sustained_stress_score"],
            }
        )
    max_stddev = max(float(entry["y"]) for entry in entries)
    y_top = max(5.0, max_stddev + 20.0)
    plot_height = max(total_height - 16, 220)
    units_per_pixel = y_top / plot_height
    label_block_height = units_per_pixel * 22.0
    label_y = [float(entry["y"]) for entry in entries]
    min_sep = max(label_block_height * 1.15, 14.0)
    lower_bound = label_block_height * 0.9
    upper_bound = y_top - label_block_height * 0.6
    for _ in range(40):
        moved = False
        for left_index in range(len(entries)):
            for right_index in range(left_index + 1, len(entries)):
                if abs(float(entries[left_index]["x"]) - float(entries[right_index]["x"])) > 220:
                    continue
                delta = label_y[right_index] - label_y[left_index]
                if abs(delta) < min_sep:
                    nudge = (min_sep - abs(delta)) / 2 + 0.1
                    if delta >= 0:
                        label_y[left_index] -= nudge
                        label_y[right_index] += nudge
                    else:
                        label_y[left_index] += nudge
                        label_y[right_index] -= nudge
                    moved = True
        for index, value in enumerate(label_y):
            label_y[index] = max(lower_bound, min(upper_bound, value))
        if not moved:
            break
    ordered = sorted(range(len(entries)), key=lambda index: label_y[index])
    for left_index, right_index in zip(ordered, ordered[1:], strict=False):
        if abs(float(entries[left_index]["x"]) - float(entries[right_index]["x"])) > 220:
            continue
        if label_y[right_index] - label_y[left_index] < min_sep:
            label_y[right_index] = min(upper_bound, label_y[left_index] + min_sep)
    for left_index, right_index in zip(reversed(ordered[1:]), reversed(ordered[:-1]), strict=False):
        if abs(float(entries[left_index]["x"]) - float(entries[right_index]["x"])) > 220:
            continue
        if label_y[left_index] - label_y[right_index] < min_sep:
            label_y[right_index] = max(lower_bound, label_y[left_index] - min_sep)
    for entry, y_value in zip(entries, label_y, strict=False):
        entry["label_y"] = y_value
        entry["label_x"] = min(float(entry["x"]) + 42.0, 1115.0)
        entry["sub_label"] = (
            f"stress={entry['stress']} "
            f"loss={int(entry['loss']) if entry['loss'] is not None else '–'}"
        )
    engine_domain = [engine for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER if engine in {entry["engine_key"] for entry in entries}]
    dataset = alt.InlineData(values=entries)
    base = alt.Chart(dataset)
    chart = alt.layer(
        base.mark_rule(color=PLOT_GRID_COLOR, strokeWidth=1).encode(
            x=alt.X("x:Q", scale=alt.Scale(domain=[0, 1180])),
            x2="label_x:Q",
            y=alt.Y(
                "y:Q",
                title="Route variability (permille stddev)",
                scale=alt.Scale(domain=[0, y_top]),
            ),
            y2="label_y:Q",
        ),
        base.mark_point(
            filled=True,
            size=90,
            stroke="white",
            strokeWidth=1,
        ).encode(
            x=alt.X(
                "x:Q",
                title="Route presence (permille)",
                scale=alt.Scale(domain=[0, 1180]),
            ),
            y=alt.Y(
                "y:Q",
                title="Route variability (permille stddev)",
                scale=alt.Scale(domain=[0, y_top]),
            ),
            color=_engine_color_scale(
                engine_domain,
                COMPARISON_ENGINE_COLORS,
                field="engine_label:N",
                field_domain=[engine_display_label(engine) for engine in engine_domain],
            ),
            tooltip=[
                alt.Tooltip("engine_label:N", title="Engine"),
                alt.Tooltip("x:Q", title="Route presence", format=".1f"),
                alt.Tooltip("y:Q", title="Stddev", format=".1f"),
                alt.Tooltip("sub_label:N", title="Detail"),
            ],
        ),
        base.mark_text(
            align="left",
            baseline="bottom",
            dx=6,
            dy=-1,
            font=PLOT_FONT,
            fontSize=9,
            color=PLOT_TEXT_COLOR,
            fontWeight="bold",
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 1180])),
            y="label_y:Q",
            text="engine_label:N",
        ),
        base.mark_text(
            align="left",
            baseline="top",
            dx=6,
            dy=1,
            font=PLOT_FONT,
            fontSize=8,
            color=PLOT_MUTED_TEXT_COLOR,
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 1180])),
            y="label_y:Q",
            text="sub_label:N",
        ),
    ).properties(width=total_width - 18, height=total_height - 16)
    return _configure_chart(chart)


def render_mixed_vs_standalone_divergence(
    aggregates: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
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
        return None
    preferred_order = {
        engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)
    }
    rows: list[dict[str, object]] = []
    for row in comparison_rows.iter_rows(named=True):
        suffix = row["family_id"].replace("comparison-", "")
        family_id = f"head-to-head-{suffix}"
        family_head = (
            head_rows.filter(pl.col("family_id") == family_id)
            .with_columns(
                pl.col("comparison_engine_set")
                .replace_strict(preferred_order, default=len(preferred_order))
                .alias("engine_order")
            )
            .sort(
                [
                    "route_present_total_window_permille_mean",
                    "activation_success_permille_mean",
                    "engine_order",
                ],
                descending=[True, True, False],
            )
        )
        if family_head.is_empty():
            continue
        best = family_head.head(1)
        best_engine = best["comparison_engine_set"].item() or "none"
        best_route_presence = float(best["route_present_total_window_permille_mean"].item() or 0)
        mixed_route_presence = float(row["mixed_route_presence"] or 0)
        delta = (best_route_presence - mixed_route_presence) / 10.0
        matched_best = best_route_presence == mixed_route_presence
        rows.append(
            {
                "family_label": wrapped_family_label(str(row["family_id"])),
                "engine_key": (
                    "tie"
                    if matched_best
                    else best_engine if best_engine in HEAD_TO_HEAD_SET_COLORS else "none"
                ),
                "engine_legend": (
                    "Matched best" if matched_best else engine_display_label(best_engine)
                ),
                "delta": delta,
                "transition_label": (
                    "Matched best"
                    if matched_best
                    else (
                        f"{compact_engine_label(row['mixed_engine'])}"
                        f" -> {compact_engine_label(best_engine)}"
                    )
                ),
                "value_label": f"{delta:.1f} pts",
            }
        )
    if not rows:
        return None

    max_delta = max(float(row["delta"]) for row in rows)
    x_min = 0.0
    x_max = max(2.0, max_delta + 12.0)
    for row in rows:
        delta = float(row["delta"])
        row["positive"] = delta >= 0
        row["label_x"] = delta
    engine_domain = [
        engine
        for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER
        if engine in {row["engine_key"] for row in rows}
    ]
    dataset = alt.InlineData(values=rows)
    base = alt.Chart(dataset).encode(
        y=alt.Y("family_label:N", sort=[row["family_label"] for row in rows], title=None)
    )
    chart = alt.layer(
        base.mark_rule(color=PLOT_BORDER_COLOR, strokeWidth=1).encode(
            x=alt.datum(0),
        ),
        base.mark_bar(height=22, cornerRadiusEnd=2, cornerRadiusTopLeft=2, cornerRadiusBottomLeft=2).encode(
            x=alt.X(
                "delta:Q",
                title="Standalone advantage over mixed (pts)",
                scale=alt.Scale(domain=[x_min, x_max]),
            ),
            color=_engine_color_scale(
                engine_domain,
                HEAD_TO_HEAD_SET_COLORS,
                field="engine_legend:N",
                field_domain=[engine_display_label(engine) for engine in engine_domain],
                legend_title="Standalone winner",
            ),
        ),
        base.transform_filter("datum.delta >= 0").mark_text(
            align="left",
            baseline="bottom",
            dx=5,
            dy=-1,
            font=PLOT_FONT,
            fontSize=8.5,
            color=PLOT_TEXT_COLOR,
            fontWeight="bold",
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[x_min, x_max])),
            text="transition_label:N",
        ),
        base.transform_filter("datum.delta >= 0").mark_text(
            align="left",
            baseline="top",
            dx=5,
            dy=1,
            font=PLOT_FONT,
            fontSize=8,
            color=PLOT_MUTED_TEXT_COLOR,
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[x_min, x_max])),
            text="value_label:N",
        ),
        base.transform_filter("datum.delta < 0").mark_text(
            align="right",
            baseline="bottom",
            dx=-5,
            dy=-1,
            font=PLOT_FONT,
            fontSize=8.5,
            color=PLOT_TEXT_COLOR,
            fontWeight="bold",
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[x_min, x_max])),
            text="transition_label:N",
        ),
        base.transform_filter("datum.delta < 0").mark_text(
            align="right",
            baseline="top",
            dx=-5,
            dy=1,
            font=PLOT_FONT,
            fontSize=8,
            color=PLOT_MUTED_TEXT_COLOR,
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[x_min, x_max])),
            text="value_label:N",
        ),
    ).properties(width=total_width - 18, height=max(180, 34 * len(rows) + 28))
    return _configure_chart(chart)


def render_diffusion_delivery_coverage(
    diffusion_engine_comparison: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    if diffusion_engine_comparison.is_empty():
        return None
    families = [
        family
        for family in DIFFUSION_FIGURE_FAMILIES
        if not diffusion_engine_comparison.filter(pl.col("family_id") == family).is_empty()
    ]
    if not families:
        return None
    engine_sets = diffusion_engine_sets(diffusion_engine_comparison)
    rows: list[dict[str, object]] = []
    for family in families:
        family_rows = diffusion_engine_comparison.filter(pl.col("family_id") == family)
        for engine_set in engine_sets:
            row = family_rows.filter(pl.col("config_id") == engine_set).head(1)
            rows.append(
                {
                    "family_label": family_label(family),
                    "engine_label": diffusion_config_label(engine_set),
                    "engine_key": engine_set,
                    "delivery": float(
                        row["delivery_probability_permille_mean"].item() if not row.is_empty() else 0
                    ),
                    "coverage": float(
                        row["coverage_permille_mean"].item() if not row.is_empty() else 0
                    ),
                }
            )
    columns = 3
    panel_width, panel_height = _panel_dimensions(
        total_width, total_height, len(families), columns
    )
    dataset = alt.InlineData(values=rows)
    y_order = [diffusion_config_label(engine) for engine in engine_sets]
    base = alt.Chart(dataset).encode(
        y=alt.Y("engine_label:N", sort=y_order, title="Engine set"),
    )
    chart = (
        alt.layer(
            base.mark_bar(height=18, cornerRadiusEnd=2).encode(
                x=alt.X(
                    "delivery:Q",
                    title="Delivery / coverage (permille)",
                    scale=alt.Scale(domain=[0, 1000]),
                ),
                color=alt.Color(
                    "engine_key:N",
                    scale=alt.Scale(
                        domain=engine_sets,
                        range=[diffusion_config_color(engine) for engine in engine_sets],
                    ),
                    legend=None,
                ),
            ),
            base.mark_rule(color="#111827", strokeWidth=1).encode(
                x="delivery:Q",
                x2="coverage:Q",
            ),
            base.mark_point(
                color="#111827",
                filled=True,
                size=48,
            ).encode(
                x="coverage:Q",
            ),
        )
        .properties(width=panel_width, height=panel_height)
        .facet(
            facet=alt.Facet(
                "family_label:N",
                sort=[family_label(family) for family in families],
                header=alt.Header(title=None, labelOrient="bottom"),
            ),
            columns=columns,
        )
    )
    return _configure_chart(chart)


def render_diffusion_resource_boundedness(
    diffusion_engine_comparison: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    if diffusion_engine_comparison.is_empty():
        return None
    families = [
        family
        for family in DIFFUSION_FIGURE_FAMILIES
        if not diffusion_engine_comparison.filter(pl.col("family_id") == family).is_empty()
    ]
    if not families:
        return None
    engine_sets = diffusion_engine_sets(diffusion_engine_comparison)
    max_tx = float(diffusion_engine_comparison["total_transmissions_mean"].max() or 0)
    rows: list[dict[str, object]] = []
    for family in families:
        family_rows = diffusion_engine_comparison.filter(pl.col("family_id") == family)
        for engine_set in engine_sets:
            row = family_rows.filter(pl.col("config_id") == engine_set).head(1)
            transmissions = float(row["total_transmissions_mean"].item() if not row.is_empty() else 0)
            reproduction = row["estimated_reproduction_permille_mean"].item() if not row.is_empty() else 0
            bounded_state = row["bounded_state_mode"].item() if not row.is_empty() else "none"
            rows.append(
                {
                    "family_label": family_label(family),
                    "engine_label": diffusion_config_label(engine_set),
                    "engine_key": engine_set,
                    "transmissions": transmissions,
                    "label_x": min(transmissions + 0.9, max(16.0, max_tx + 8.0) - 0.4),
                    "reproduction_label": f"R={int(reproduction or 0)}",
                    "bounded_state": bounded_state,
                }
            )
    columns = 3
    panel_width, panel_height = _panel_dimensions(
        total_width, total_height, len(families), columns
    )
    dataset = alt.InlineData(values=rows)
    y_order = [diffusion_config_label(engine) for engine in engine_sets]
    x_limit = max(16.0, max_tx + 8.0)
    base = alt.Chart(dataset).encode(
        y=alt.Y("engine_label:N", sort=y_order, title="Engine set"),
    )
    chart = (
        alt.layer(
            base.mark_bar(height=18, cornerRadiusEnd=2).encode(
                x=alt.X(
                    "transmissions:Q",
                    title="Tx mean",
                    scale=alt.Scale(domain=[0, x_limit]),
                ),
                color=alt.Color(
                    "engine_key:N",
                    scale=alt.Scale(
                        domain=engine_sets,
                        range=[diffusion_config_color(engine) for engine in engine_sets],
                    ),
                    legend=None,
                ),
            ),
            base.mark_text(
                align="left",
                baseline="bottom",
                dx=5,
                dy=-1,
                font=PLOT_FONT,
                fontSize=8,
                color=PLOT_TEXT_COLOR,
                clip=False,
            ).encode(
                x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, x_limit])),
                text="reproduction_label:N",
            ),
            base.mark_text(
                align="left",
                baseline="top",
                dx=5,
                dy=1,
                font=PLOT_FONT,
                fontSize=8,
                clip=False,
            ).encode(
                x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, x_limit])),
                text="bounded_state:N",
                color=alt.Color(
                    "bounded_state:N",
                    scale=alt.Scale(
                        domain=list(DIFFUSION_BOUND_STATE_COLORS),
                        range=list(DIFFUSION_BOUND_STATE_COLORS.values()),
                    ),
                    legend=None,
                ),
            ),
        )
        .properties(width=panel_width, height=panel_height)
        .facet(
            facet=alt.Facet(
                "family_label:N",
                sort=[family_label(family) for family in families],
                header=alt.Header(title=None, labelOrient="bottom"),
            ),
            columns=columns,
        )
    )
    return _configure_chart(chart)


def render_large_population_route_scaling(
    large_population_route_summary: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    if large_population_route_summary.is_empty():
        return None
    engine_order = {
        engine: index for index, engine in enumerate(ROUTE_VISIBLE_ENGINE_SET_ORDER)
    }
    rows: list[dict[str, object]] = []
    for row in large_population_route_summary.iter_rows(named=True):
        for size_band, column in [
            ("small", "small_route_present"),
            ("moderate", "moderate_route_present"),
            ("high", "high_route_present"),
        ]:
            value = row[column]
            if value is None:
                continue
            rows.append(
                {
                    "topology_label": row["topology_label"],
                    "size_band": size_band.capitalize(),
                    "engine_key": row["comparison_engine_set"],
                    "engine_label": engine_display_label(row["comparison_engine_set"]),
                    "route_present": float(value),
                    "display_route_present": float(value),
                }
            )
    if not rows:
        return None
    tied_groups: dict[tuple[str, str, float], list[dict[str, object]]] = {}
    for row in rows:
        tied_groups.setdefault(
            (row["topology_label"], row["size_band"], row["route_present"]),
            [],
        ).append(row)
    dodge_step = 8.0
    for group_rows in tied_groups.values():
        if len(group_rows) <= 1:
            continue
        ordered_rows = sorted(
            group_rows,
            key=lambda row: engine_order.get(str(row["engine_key"]), len(engine_order)),
        )
        midpoint = (len(ordered_rows) - 1) / 2.0
        for index, row in enumerate(ordered_rows):
            row["display_route_present"] = row["route_present"] + (index - midpoint) * dodge_step
    engine_domain = [
        engine
        for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER
        if engine in {row["engine_key"] for row in rows}
    ]
    size_order = ["Small", "Moderate", "High"]
    dataset = alt.InlineData(values=rows)
    base = alt.Chart(dataset).encode(
        x=alt.X("size_band:N", sort=size_order, title="Size band"),
        y=alt.Y(
            "display_route_present:Q",
            title="Total-window route presence (permille)",
            scale=alt.Scale(domain=[0, 1000]),
        ),
        color=_engine_color_scale(
            engine_domain,
            HEAD_TO_HEAD_SET_COLORS,
            field="engine_label:N",
            field_domain=[engine_display_label(engine) for engine in engine_domain],
            legend_title="Engine set",
        ),
        tooltip=[
            alt.Tooltip("topology_label:N", title="Topology"),
            alt.Tooltip("engine_label:N", title="Engine set"),
            alt.Tooltip("size_band:N", title="Size band"),
            alt.Tooltip("route_present:Q", title="Route presence", format=".0f"),
        ],
    )
    chart = alt.layer(
        base.mark_line(point=False, strokeWidth=2.2),
        base.mark_point(filled=True, size=80, stroke="white", strokeWidth=1),
    ).properties(width=(total_width - 30) // 2, height=total_height - 54).facet(
        facet=alt.Facet(
            "topology_label:N",
            sort=[
                "Diameter / fanout scaling",
                "Multi-bottleneck repair",
            ],
            header=alt.Header(title=None),
        ),
        columns=2,
    )
    return _configure_chart(chart)


def render_large_population_route_fragility(
    large_population_route_summary: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    if large_population_route_summary.is_empty():
        return None
    rows: list[dict[str, object]] = []
    for row in large_population_route_summary.iter_rows(named=True):
        delta = row["small_to_high_route_delta"]
        if delta is None:
            continue
        loss_round = row["high_first_loss_round"]
        loss_label = "no loss" if loss_round is None else f"loss r{int(round(loss_round))}"
        rows.append(
            {
                "topology_label": row["topology_label"],
                "engine_key": row["comparison_engine_set"],
                "engine_label": engine_display_label(row["comparison_engine_set"]),
                "route_delta": float(delta),
                "label_x": float(delta),
                "loss_label": loss_label,
            }
        )
    if not rows:
        return None
    engine_domain = [
        engine
        for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER
        if engine in {row["engine_key"] for row in rows}
    ]
    min_delta = min(float(row["route_delta"]) for row in rows)
    max_delta = max(float(row["route_delta"]) for row in rows)
    x_min = min(-850.0, min_delta - 80.0)
    x_max = max(120.0, max_delta + 80.0)
    dataset = alt.InlineData(values=rows)
    base = alt.Chart(dataset).encode(
        y=alt.Y(
            "engine_label:N",
            sort=[engine_display_label(engine) for engine in engine_domain],
            title="Engine set",
        ),
        color=_engine_color_scale(
            engine_domain,
            HEAD_TO_HEAD_SET_COLORS,
            field="engine_label:N",
            field_domain=[engine_display_label(engine) for engine in engine_domain],
            legend_title="Engine set",
        ),
    )
    chart = alt.layer(
        base.mark_rule(color=PLOT_BORDER_COLOR, strokeWidth=1).encode(x=alt.datum(0)),
        base.mark_bar(height=18, cornerRadiusEnd=2, cornerRadiusTopLeft=2, cornerRadiusBottomLeft=2).encode(
            x=alt.X(
                "route_delta:Q",
                title="High-band total-window route presence minus small baseline (permille)",
                scale=alt.Scale(domain=[x_min, x_max]),
            ),
        ),
        base.mark_text(
            baseline="middle",
            dx=6,
            font=PLOT_FONT,
            fontSize=8,
            color="#000000",
            clip=False,
        ).encode(
            x=alt.X("label_x:Q", scale=alt.Scale(domain=[x_min, x_max])),
            text="loss_label:N",
        ),
    ).properties(width=(total_width - 30) // 2, height=total_height - 54).facet(
        facet=alt.Facet(
            "topology_label:N",
            sort=[
                "Diameter / fanout scaling",
                "Multi-bottleneck repair",
            ],
            header=alt.Header(title=None),
        ),
        columns=2,
    )
    return _configure_chart(chart)


def render_routing_fitness_crossover(
    routing_fitness_crossover_summary: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    if routing_fitness_crossover_summary.is_empty():
        return None
    question_order = []
    for entry in ROUTING_FITNESS_CROSSOVER_FAMILIES:
        if entry["question_label"] not in question_order:
            question_order.append(entry["question_label"])
    band_order = ["low", "moderate", "high"]
    rows: list[dict[str, object]] = []
    for row in routing_fitness_crossover_summary.iter_rows(named=True):
        engine_set = row["comparison_engine_set"]
        for metric_key, metric_label, value in [
            (
                "route",
                "Route presence",
                row["route_present_total_window_permille_mean"],
            ),
            (
                "recovery",
                "Recovery success",
                row["recovery_success_permille_mean"],
            ),
        ]:
            if value is None:
                continue
            rows.append(
                {
                    "question_label": row["question_label"],
                    "band_label": str(row["band_label"]).capitalize(),
                    "band_order": row["band_order"],
                    "engine_key": engine_set,
                    "engine_label": engine_display_label(engine_set),
                    "metric_key": metric_key,
                    "metric_label": metric_label,
                    "value": float(value) / 10.0,
                    "hover_detail": (
                        f"loss r{int(row['first_loss_round_mean'])}"
                        if row["first_loss_round_mean"] is not None
                        else "no loss"
                    ),
                    "churn": float(row["route_churn_count_mean"] or 0.0),
                    "control_activity": float(row["route_observation_count_mean"] or 0.0),
                    "hop_count": row["active_route_hop_count_mean"],
                }
            )
    if not rows:
        return None
    engine_domain = [
        engine
        for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER
        if engine in {str(row["engine_key"]) for row in rows}
    ]
    dataset = alt.InlineData(values=rows)
    base = alt.Chart(dataset).encode(
        x=alt.X(
            "band_label:N",
            sort=[label.capitalize() for label in band_order],
            title="Difficulty band",
        ),
        y=alt.Y(
            "value:Q",
            title="Outcome (%)",
            scale=alt.Scale(domain=[0, 100]),
        ),
        color=_engine_color_scale(
            engine_domain,
            HEAD_TO_HEAD_SET_COLORS,
            field="engine_label:N",
            field_domain=[engine_display_label(engine) for engine in engine_domain],
            legend_title="Engine set",
        ),
        strokeDash=alt.StrokeDash(
            "metric_label:N",
            scale=alt.Scale(
                domain=["Route presence", "Recovery success"],
                range=[[1, 0], [7, 4]],
            ),
            legend=alt.Legend(title="Metric"),
        ),
        tooltip=[
            alt.Tooltip("question_label:N", title="Question"),
            alt.Tooltip("band_label:N", title="Band"),
            alt.Tooltip("engine_label:N", title="Engine"),
            alt.Tooltip("metric_label:N", title="Metric"),
            alt.Tooltip("value:Q", title="Value", format=".1f"),
            alt.Tooltip("hover_detail:N", title="Loss"),
            alt.Tooltip("churn:Q", title="Churn", format=".1f"),
            alt.Tooltip("control_activity:Q", title="Control activity", format=".1f"),
            alt.Tooltip("hop_count:Q", title="Hop mean", format=".1f"),
        ],
    )
    chart = (
        base.mark_line(strokeWidth=2.2, point=True)
        .properties(width=(total_width - 30) // 2, height=total_height - 54)
        .facet(
            facet=alt.Facet(
                "question_label:N",
                sort=question_order,
                header=alt.Header(title=None),
            ),
            columns=2,
        )
    )
    return _configure_chart(chart)


def render_routing_fitness_multiflow(
    routing_fitness_multiflow_summary: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    if routing_fitness_multiflow_summary.is_empty():
        return None
    family_order = [entry["family_label"] for entry in ROUTING_FITNESS_MULTI_FLOW_FAMILIES]
    available = set(
        routing_fitness_multiflow_summary["comparison_engine_set"].drop_nulls().unique().to_list()
    )
    engine_domain = [engine for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER if engine in available]
    rows: list[dict[str, object]] = []
    for row in routing_fitness_multiflow_summary.iter_rows(named=True):
        min_value = float(row["objective_route_presence_min_permille_mean"] or 0.0) / 10.0
        max_value = float(row["objective_route_presence_max_permille_mean"] or 0.0) / 10.0
        spread_value = float(row["objective_route_presence_spread_mean"] or 0.0) / 10.0
        label_x = min(max_value + 2.5, 103.5)
        rows.append(
            {
                "family_label": row["family_label"],
                "engine_key": row["comparison_engine_set"],
                "engine_label": engine_display_label(row["comparison_engine_set"]),
                "min_route": min_value,
                "max_route": max_value,
                "label_x": label_x,
                "spread_label": f"spread={spread_value:.1f}",
                "detail_label": (
                    f"starved={int(round(row['objective_starvation_count_mean'] or 0.0))} "
                    f"broker={float(row['broker_participation_permille_mean'] or 0.0) / 10.0:.0f}/"
                    f"{float(row['broker_concentration_permille_mean'] or 0.0) / 10.0:.0f}"
                ),
                "concurrent_rounds": float(row["concurrent_route_round_count_mean"] or 0.0),
                "broker_participation": float(row["broker_participation_permille_mean"] or 0.0)
                / 10.0,
                "broker_concentration": float(row["broker_concentration_permille_mean"] or 0.0)
                / 10.0,
                "broker_churn": float(row["broker_route_churn_count_mean"] or 0.0),
                "control_activity": float(row["route_observation_count_mean"] or 0.0),
            }
        )
    dataset = alt.InlineData(values=rows)
    y_order = [engine_display_label(engine) for engine in engine_domain]
    base = alt.Chart(dataset).encode(
        y=alt.Y("engine_label:N", sort=y_order, title="Engine set"),
        color=_engine_color_scale(
            engine_domain,
            HEAD_TO_HEAD_SET_COLORS,
            field="engine_label:N",
            field_domain=y_order,
            legend_title="Engine set",
        ),
        tooltip=[
            alt.Tooltip("family_label:N", title="Family"),
            alt.Tooltip("engine_label:N", title="Engine"),
            alt.Tooltip("min_route:Q", title="Min route", format=".1f"),
            alt.Tooltip("max_route:Q", title="Max route", format=".1f"),
            alt.Tooltip("concurrent_rounds:Q", title="Concurrent rounds", format=".1f"),
            alt.Tooltip("broker_participation:Q", title="Broker participation", format=".1f"),
            alt.Tooltip("broker_concentration:Q", title="Broker concentration", format=".1f"),
            alt.Tooltip("broker_churn:Q", title="Broker churn", format=".1f"),
            alt.Tooltip("control_activity:Q", title="Control activity", format=".1f"),
            alt.Tooltip("detail_label:N", title="Detail"),
        ],
    )
    chart = (
        alt.layer(
            base.mark_rule(strokeWidth=3).encode(
                x=alt.X(
                    "min_route:Q",
                    title="Per-flow route presence (%)",
                    scale=alt.Scale(domain=[0, 106]),
                ),
                x2="max_route:Q",
            ),
            base.mark_point(
                filled=True,
                size=85,
                stroke="white",
                strokeWidth=1,
            ).encode(x="min_route:Q"),
            base.mark_point(
                filled=True,
                size=80,
                shape="square",
                stroke="white",
                strokeWidth=1,
            ).encode(x="max_route:Q"),
            base.mark_text(
                align="left",
                baseline="bottom",
                dx=6,
                dy=-1,
                font=PLOT_FONT,
                fontSize=8.5,
                color=PLOT_TEXT_COLOR,
                clip=False,
            ).encode(
                x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 106])),
                text="spread_label:N",
            ),
            base.mark_text(
                align="left",
                baseline="top",
                dx=6,
                dy=1,
                font=PLOT_FONT,
                fontSize=8,
                color=PLOT_MUTED_TEXT_COLOR,
                clip=False,
            ).encode(
                x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, 106])),
                text="detail_label:N",
            ),
        )
        .properties(width=(total_width - 42) // 3, height=total_height - 62)
        .facet(
            facet=alt.Facet(
                "family_label:N",
                sort=family_order,
                header=alt.Header(title=None, labelOrient="bottom"),
            ),
            columns=3,
        )
    )
    return _configure_chart(chart)


def render_routing_fitness_stale_repair(
    routing_fitness_stale_repair_summary: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    if routing_fitness_stale_repair_summary.is_empty():
        return None
    family_order = [entry["family_label"] for entry in ROUTING_FITNESS_STALE_FAMILIES]
    available = set(
        routing_fitness_stale_repair_summary["comparison_engine_set"].drop_nulls().unique().to_list()
    )
    engine_domain = [engine for engine in ROUTE_VISIBLE_ENGINE_SET_ORDER if engine in available]
    rows: list[dict[str, object]] = []
    for row in routing_fitness_stale_repair_summary.iter_rows(named=True):
        persistence = float(row["stale_persistence_round_mean"] or 0.0)
        label_x = persistence + 0.25
        rows.append(
            {
                "family_label": row["family_label"],
                "engine_key": row["comparison_engine_set"],
                "engine_label": engine_display_label(row["comparison_engine_set"]),
                "stale_persistence": persistence,
                "label_x": label_x,
                "recovery_success": float(row["recovery_success_permille_mean"] or 0.0)
                / 10.0,
                "recovery_label": (
                    f"recov={float(row['recovery_success_permille_mean'] or 0.0) / 10.0:.1f}%"
                ),
                "detail_label": (
                    f"unrec={int(round(row['unrecovered_after_loss_count_mean'] or 0.0))} "
                    f"loss={int(row['first_loss_round_mean']) if row['first_loss_round_mean'] is not None else '–'}"
                ),
                "control_activity": float(row["route_observation_count_mean"] or 0.0),
            }
        )
    x_limit = max(1.0, max(float(row["stale_persistence"]) for row in rows) + 1.2)
    dataset = alt.InlineData(values=rows)
    y_order = [engine_display_label(engine) for engine in engine_domain]
    base = alt.Chart(dataset).encode(
        y=alt.Y("engine_label:N", sort=y_order, title="Engine set"),
        color=_engine_color_scale(
            engine_domain,
            HEAD_TO_HEAD_SET_COLORS,
            field="engine_label:N",
            field_domain=y_order,
            legend_title="Engine set",
        ),
        tooltip=[
            alt.Tooltip("family_label:N", title="Family"),
            alt.Tooltip("engine_label:N", title="Engine"),
            alt.Tooltip("stale_persistence:Q", title="Stale persistence", format=".1f"),
            alt.Tooltip("recovery_success:Q", title="Recovery success", format=".1f"),
            alt.Tooltip("control_activity:Q", title="Control activity", format=".1f"),
            alt.Tooltip("detail_label:N", title="Detail"),
        ],
    )
    chart = (
        alt.layer(
            base.mark_bar(height=18, cornerRadiusEnd=2).encode(
                x=alt.X(
                    "stale_persistence:Q",
                    title="Bad-route persistence after disruption (rounds)",
                    scale=alt.Scale(domain=[0, x_limit]),
                )
            ),
            base.mark_text(
                align="left",
                baseline="bottom",
                dx=6,
                dy=-1,
                font=PLOT_FONT,
                fontSize=8.5,
                color=PLOT_TEXT_COLOR,
                clip=False,
            ).encode(
                x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, x_limit])),
                text="recovery_label:N",
            ),
            base.mark_text(
                align="left",
                baseline="top",
                dx=6,
                dy=1,
                font=PLOT_FONT,
                fontSize=8,
                color=PLOT_MUTED_TEXT_COLOR,
                clip=False,
            ).encode(
                x=alt.X("label_x:Q", scale=alt.Scale(domain=[0, x_limit])),
                text="detail_label:N",
            ),
        )
        .properties(width=(total_width - 42) // 3, height=total_height - 62)
        .facet(
            facet=alt.Facet(
                "family_label:N",
                sort=family_order,
                header=alt.Header(title=None, labelOrient="bottom"),
            ),
            columns=3,
        )
    )
    return _configure_chart(chart)


def render_large_population_diffusion_transitions(
    large_population_diffusion_points: pl.DataFrame, total_width: int, total_height: int
) -> alt.TopLevelMixin | None:
    if large_population_diffusion_points.is_empty():
        return None
    rows: list[dict[str, object]] = []
    for row in large_population_diffusion_points.iter_rows(named=True):
        rows.append(
            {
                "panel_label": f"{row['question_label']} ({row['size_band']})",
                "question_label": row["question_label"],
                "size_band": row["size_band"].capitalize(),
                "config_label": compact_engine_label(row["config_id"]),
                "delivery": float(row["delivery_probability_permille_mean"]),
                "reproduction": float(row["estimated_reproduction_permille_mean"]),
                "bounded_state": row["bounded_state_mode"],
            }
        )
    max_reproduction = max(float(row["reproduction"]) for row in rows)
    x_domain_max = max(1000.0, max_reproduction + 120.0)

    # Nudge label y positions apart within each panel so labels for nearby
    # points do not stack on top of each other.
    panel_indices: dict[str, list[int]] = defaultdict(list)
    for idx, row in enumerate(rows):
        panel_indices[row["panel_label"]].append(idx)
    for row in rows:
        row["label_y"] = row["delivery"]
    text_height_y = 90.0
    overlap_x_threshold = x_domain_max * 0.45
    for indices in panel_indices.values():
        if len(indices) <= 1:
            continue
        for _ in range(30):
            moved = False
            for a in range(len(indices)):
                for b in range(a + 1, len(indices)):
                    i, j = indices[a], indices[b]
                    if abs(rows[i]["reproduction"] - rows[j]["reproduction"]) > overlap_x_threshold:
                        continue
                    dy = rows[j]["label_y"] - rows[i]["label_y"]
                    if abs(dy) < text_height_y:
                        nudge = (text_height_y - abs(dy)) / 2 + 1.0
                        if dy >= 0:
                            rows[i]["label_y"] -= nudge
                            rows[j]["label_y"] += nudge
                        else:
                            rows[i]["label_y"] += nudge
                            rows[j]["label_y"] -= nudge
                        moved = True
            if not moved:
                break
        for idx in indices:
            rows[idx]["label_y"] = max(40.0, min(960.0, rows[idx]["label_y"]))

    dataset = alt.InlineData(values=rows)
    base = alt.Chart(dataset).encode(
        x=alt.X(
            "reproduction:Q",
            title="Estimated reproduction (permille)",
            scale=alt.Scale(domain=[0, x_domain_max]),
        ),
        y=alt.Y(
            "delivery:Q",
            title="Delivery (permille)",
            scale=alt.Scale(domain=[0, 1000]),
        ),
    )
    chart = alt.layer(
        base.mark_line(color=PLOT_BORDER_COLOR, strokeWidth=1.2).encode(
            detail="panel_label:N",
            order="reproduction:Q",
        ),
        base.mark_point(filled=True, size=95, stroke="white", strokeWidth=1).encode(
            color=alt.Color(
                "bounded_state:N",
                scale=alt.Scale(
                    domain=LARGE_POPULATION_STATE_ORDER,
                    range=[DIFFUSION_BOUND_STATE_COLORS[state] for state in LARGE_POPULATION_STATE_ORDER],
                ),
                legend=alt.Legend(title="Bounded state"),
            ),
            shape=alt.Shape("bounded_state:N", legend=None),
        ),
        base.mark_text(
            align="left",
            baseline="middle",
            dx=7,
            font=PLOT_FONT,
            fontSize=8,
            color=PLOT_TEXT_COLOR,
            clip=False,
        ).encode(
            text="config_label:N",
            y=alt.Y(
                "label_y:Q",
                scale=alt.Scale(domain=[0, 1000]),
                title=None,
                axis=None,
            ),
        ),
    ).properties(width=(total_width - 42) // 3, height=(total_height - 74) // 2).facet(
        facet=alt.Facet(
            "panel_label:N",
            sort=[row["panel_label"] for row in rows],
            header=alt.Header(title=None, labelOrient="bottom"),
        ),
        columns=3,
    )
    return _configure_chart(chart)


def save_plot_artifact(
    report_dir: Path,
    key: str,
    render_fn,
    aggregates: pl.DataFrame,
) -> None:
    total_width, total_height = _plot_pixels(key)
    chart = render_fn(aggregates, total_width, total_height)
    if chart is None:
        chart = _placeholder_chart(total_width, total_height, "No data available")

    svg_path = report_dir / f"{key}.svg"
    png_path = report_dir / f"{key}.png"
    pdf_path = report_dir / f"{key}.pdf"

    chart.save(str(svg_path))
    chart.save(str(png_path), scale_factor=2)
    try:
        chart.save(str(pdf_path))
    except Exception:
        drawing = svg2rlg(str(svg_path))
        renderPDF.drawToFile(drawing, str(pdf_path))
