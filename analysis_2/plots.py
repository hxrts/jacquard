"""Altair figure rendering for the active-belief paper report."""

from __future__ import annotations

from collections import defaultdict
from pathlib import Path

import altair as alt
from reportlab.graphics import renderPDF
from svglib.svglib import svg2rlg

from analysis.constants import ENGINE_COLORS
from analysis.plots import (
    PIXELS_PER_INCH,
    PLOT_FONT,
    PLOT_MUTED_TEXT_COLOR,
    _configure_chart,
    _placeholder_chart,
)

alt.data_transformers.disable_max_rows()

ACTIVE_BELIEF_FIGURE_WIDTH_INCHES = 11.0
ACTIVE_BELIEF_FIGURE_HEIGHT_INCHES = 4.8


def save_active_belief_plot_artifact(
    report_dir: Path,
    figure_id: str,
    title: str,
    rows: list[dict[str, object]],
    dataset: str,
) -> tuple[list[int], list[str]]:
    width = int(ACTIVE_BELIEF_FIGURE_WIDTH_INCHES * PIXELS_PER_INCH)
    height = int(ACTIVE_BELIEF_FIGURE_HEIGHT_INCHES * PIXELS_PER_INCH)
    chart = render_figure(figure_id, title, rows, width, height)
    svg_path = report_dir / f"{figure_id}.svg"
    pdf_path = report_dir / f"{figure_id}.pdf"
    chart.save(str(svg_path))
    try:
        chart.save(str(pdf_path))
    except Exception:
        drawing = svg2rlg(str(svg_path))
        renderPDF.drawToFile(drawing, str(pdf_path))
    values, labels = headline_values(rows, dataset)
    return values, labels


def render_figure(
    figure_id: str,
    title: str,
    rows: list[dict[str, object]],
    width: int,
    height: int,
) -> alt.TopLevelMixin:
    if not rows:
        return _placeholder_chart(width, height, "No active-belief rows available")
    if figure_id == "figure_01_landscape_focus":
        return landscape_distribution(title, rows, width, height)
    if figure_id == "figure_02_path_free_recovery":
        return path_free_distribution(title, rows, width, height)
    if figure_id == "figure_03_three_mode_comparison":
        return evidence_mode_small_multiples(title, rows, width, height)
    if figure_id == "figure_04_active_belief_grid":
        return receiver_metric_grid(title, rows, width, height)
    if figure_id == "figure_05_task_algebra":
        return task_baseline_distribution(title, rows, width, height)
    if figure_id == "figure_06_phase_diagram":
        return phase_small_multiples(title, rows, width, height)
    if figure_id == "figure_07_active_vs_passive":
        return demand_ablation_boxplot(title, rows, width, height)
    if figure_id == "figure_08_coding_vs_replication":
        return coding_cost_curve(title, rows, width, height)
    if figure_id == "figure_09_recoding_frontier":
        return recoding_frontier_scatter(title, rows, width, height)
    if figure_id == "figure_10_robustness_boundary":
        return robustness_small_multiples(title, rows, width, height)
    if figure_id == "figure_11_observer_ambiguity":
        return grouped_bar(
            title,
            rows,
            width,
            height,
            x_field="policy_or_mode",
            y_fields=["observer_advantage_permille", "uncertainty_permille"],
            y_title="Observer metric proxy (permille)",
        )
    if figure_id == "figure_12_host_bridge_demand":
        return demand_safety_matrix(title, rows, width, height)
    if figure_id == "figure_13_theorem_assumptions":
        return theorem_assumption_matrix(title, rows, width, height)
    if figure_id == "figure_14_large_regime":
        return scale_validation_panels(title, rows, width, height)
    if figure_id == "figure_15_trace_validation":
        return trace_validation_matrix(title, rows, width, height)
    if figure_id == "figure_16_strong_baselines":
        return baseline_boxplot(title, rows, width, height)
    return _placeholder_chart(width, height, f"No renderer for {figure_id}")


def landscape_distribution(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    grouped: dict[tuple[str, int, str], list[int]] = defaultdict(list)
    metrics = {
        "quality": "merged_statistic_quality_permille",
        "margin": "top_hypothesis_margin",
        "uncertainty": "uncertainty_permille",
    }
    for row in rows:
        if row.get("task_kind") != "anomaly-localization":
            continue
        mode = str(row["policy_or_mode"])
        if mode == "uncoded-replication":
            continue
        for metric, field in metrics.items():
            grouped[(display_label(mode), int_value(row, "round_index"), metric)].append(int_value(row, field))
    values = [
        {
            "mode": mode,
            "round": round_index,
            "metric": metric,
            "low": quantile(scores, 0.25),
            "median": quantile(scores, 0.50),
            "high": quantile(scores, 0.75),
        }
        for (mode, round_index, metric), scores in grouped.items()
    ]
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("round:Q", title="Round", axis=alt.Axis(tickMinStep=1)),
        color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
        tooltip=["mode:N", "metric:N", "round:Q", "low:Q", "median:Q", "high:Q"],
    )
    band = base.mark_area(opacity=0.18).encode(
        y=alt.Y("high:Q", title="Permille"),
        y2="low:Q",
    )
    line = base.mark_line(point=True, strokeWidth=2.3).encode(y="median:Q")
    chart = alt.layer(band, line).properties(width=width // 3, height=height).facet(
        column=alt.Column("metric:N", title=None),
    )
    chart = chart.properties(title=title)
    return _configure_chart(chart)


def path_free_distribution(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values = [
        {
            "mode": display_label(str(row["policy_or_mode"])),
            "success": int_value(row, "path_free_success_permille"),
            "static path absent": "yes" if bool_value(row, "no_static_path_in_core_window") else "no",
            "journey": "yes" if bool_value(row, "time_respecting_evidence_journey_exists") else "no",
        }
        for row in rows
    ]
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_boxplot(size=52)
        .encode(
            x=alt.X("mode:N", title=None, axis=alt.Axis(labelAngle=-20)),
            y=alt.Y("success:Q", title="Path-free success (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("mode:N", legend=None),
            tooltip=["mode:N", "success:Q", "static path absent:N", "journey:N"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def evidence_mode_small_multiples(
    title: str,
    rows: list[dict[str, object]],
    width: int,
    height: int,
) -> alt.TopLevelMixin:
    max_available = max(int_value(row, "available_evidence_count") for row in rows)
    max_useful = max(int_value(row, "useful_contribution_count") for row in rows)
    max_duplicates = max(int_value(row, "duplicate_count") for row in rows)
    values: list[dict[str, object]] = []
    for row in rows:
        condition = display_label(str(row["policy_or_mode"]))
        values.extend(
            [
                {
                    "condition": condition,
                    "metric": "available evidence",
                    "value": scaled(int_value(row, "available_evidence_count"), max_available),
                },
                {
                    "condition": condition,
                    "metric": "useful contributions",
                    "value": scaled(int_value(row, "useful_contribution_count"), max_useful),
                },
                {
                    "condition": condition,
                    "metric": "quality",
                    "value": int_value(row, "merged_statistic_quality_permille"),
                },
                {
                    "condition": condition,
                    "metric": "duplicate pressure",
                    "value": scaled(int_value(row, "duplicate_count"), max_duplicates),
                },
            ]
        )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_bar()
        .encode(
            x=alt.X("condition:N", title=None, axis=alt.Axis(labelAngle=-15)),
            y=alt.Y("mean(value):Q", title="Normalized or permille value", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("condition:N", legend=None),
            column=alt.Column("metric:N", title=None),
            tooltip=["condition:N", "metric:N", alt.Tooltip("mean(value):Q", title="mean")],
        )
        .properties(width=width // 4, height=height, title=title)
    )
    return _configure_chart(chart)


def receiver_metric_grid(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values: list[dict[str, object]] = []
    for row in rows:
        mode = str(row["mode"])
        if mode == "uncoded-replication":
            continue
        values.extend(
            [
                {
                    "regime": display_label(str(row["scenario_id"])),
                    "mode": display_label(mode),
                    "metric": "quality",
                    "value": int_value(row, "quality_per_byte_permille"),
                },
                {
                    "regime": display_label(str(row["scenario_id"])),
                    "mode": display_label(mode),
                    "metric": "agreement",
                    "value": int_value(row, "receiver_agreement_permille"),
                },
                {
                    "regime": display_label(str(row["scenario_id"])),
                    "mode": display_label(mode),
                    "metric": "lead time x100",
                    "value": int_value(row, "commitment_lead_time_rounds") * 100,
                },
                {
                    "regime": display_label(str(row["scenario_id"])),
                    "mode": display_label(mode),
                    "metric": "uncertainty",
                    "value": int_value(row, "collective_uncertainty_permille"),
                },
            ]
        )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_bar()
        .encode(
            x=alt.X("mode:N", title=None, axis=alt.Axis(labelAngle=-20)),
            y=alt.Y("mean(value):Q", title="Mean value"),
            color=alt.Color("mode:N", legend=None),
            row=alt.Row("regime:N", title=None),
            column=alt.Column("metric:N", title=None),
            tooltip=["regime:N", "mode:N", "metric:N", alt.Tooltip("mean(value):Q", title="mean")],
        )
        .properties(width=width // 4, height=max(90, height // 3), title=title)
    )
    return _configure_chart(chart)


def task_baseline_distribution(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values = [
        {
            "task": display_label(str(row["task_kind"])),
            "mode": display_label(str(row["mode"])),
            "accuracy": int_value(row, "decision_accuracy_permille"),
        }
        for row in rows
    ]
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_boxplot(size=38)
        .encode(
            x=alt.X("task:N", title=None, axis=alt.Axis(labelAngle=-15)),
            y=alt.Y("accuracy:Q", title="Decision accuracy (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
            xOffset=alt.XOffset("mode:N"),
            tooltip=["task:N", "mode:N", "accuracy:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def phase_small_multiples(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    grouped: dict[tuple[str, str, str], list[int]] = defaultdict(list)
    metrics = {
        "quality": "quality_permille",
        "duplicate rate": "duplicate_rate_permille",
        "bytes": "byte_count",
        "R_est": "r_est_permille",
    }
    for row in rows:
        for metric, field in metrics.items():
            grouped[(display_label(str(row["scenario_id"])), f"budget {row['forwarding_budget']}", metric)].append(
                int_value(row, field)
            )
    values = [
        {
            "band": band,
            "budget": budget,
            "metric": metric,
            "value": quantile(scores, 0.50),
        }
        for (band, budget, metric), scores in grouped.items()
    ]
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("budget:N", title=None),
        y=alt.Y("band:N", title=None),
    )
    heatmap = base.mark_rect().encode(
        color=alt.Color("value:Q", legend=alt.Legend(title="Median")),
        tooltip=["band:N", "budget:N", "metric:N", "value:Q"],
    )
    labels = base.mark_text(font=PLOT_FONT, fontSize=9).encode(text="value:Q")
    chart = (heatmap + labels).properties(width=width // 4, height=height).facet(
        column=alt.Column("metric:N", title=None)
    )
    chart = chart.properties(title=title)
    return _configure_chart(chart)


def demand_ablation_boxplot(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values = [
        {
            "policy": display_label(str(row["demand_policy"])),
            "quality": int_value(row, "quality_per_byte_permille"),
            "lag": int_value(row, "demand_response_lag_rounds"),
            "satisfaction": int_value(row, "demand_satisfaction_permille"),
        }
        for row in rows
    ]
    box = (
        alt.Chart(alt.InlineData(values=values))
        .mark_boxplot(size=38)
        .encode(
            x=alt.X("policy:N", title=None, axis=alt.Axis(labelAngle=-25, labelLimit=120)),
            y=alt.Y("quality:Q", title="Quality per byte (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("policy:N", legend=None),
            tooltip=["policy:N", "quality:Q", "satisfaction:Q", "lag:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(box)


def coding_cost_curve(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    grouped: dict[tuple[str, int], list[int]] = defaultdict(list)
    for row in rows:
        grouped[(display_label(str(row["policy_or_mode"])), int_value(row, "fixed_payload_budget_bytes"))].append(
            int_value(row, "quality_permille")
        )
    values = [
        {"mode": mode, "budget": budget, "quality": quantile(scores, 0.50)}
        for (mode, budget), scores in grouped.items()
    ]
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_line(point=True, strokeWidth=2.4)
        .encode(
            x=alt.X("budget:Q", title="Payload-byte budget"),
            y=alt.Y("quality:Q", title="Median quality (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
            tooltip=["mode:N", "budget:Q", "quality:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def recoding_frontier_scatter(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values = [
        {
            "mode": display_label(str(row["mode"])),
            "bytes": int_value(row, "bytes_at_commitment"),
            "quality": int_value(row, "quality_per_byte_permille"),
            "lead": int_value(row, "commitment_lead_time_rounds"),
        }
        for row in rows
        if str(row["mode"]) in {"passive-controlled-coded", "full-active-belief", "recoded-aggregate"}
    ]
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_circle(size=42, opacity=0.45)
        .encode(
            x=alt.X("bytes:Q", title="Bytes at commitment"),
            y=alt.Y("quality:Q", title="Quality per byte (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
            tooltip=["mode:N", "bytes:Q", "quality:Q", "lead:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def robustness_small_multiples(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values: list[dict[str, object]] = []
    for row in rows:
        regime = display_label(str(row["scenario_regime"]))
        severity = int_value(row, "stress_severity")
        values.extend(
            [
                {
                    "regime": regime,
                    "severity": severity,
                    "metric": "commitment accuracy",
                    "value": int_value(row, "commitment_accuracy_permille"),
                },
                {
                    "regime": regime,
                    "severity": severity,
                    "metric": "false commitment",
                    "value": int_value(row, "false_commitment_rate_permille"),
                },
            ]
        )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_boxplot(size=34)
        .encode(
            x=alt.X("regime:N", title=None, axis=alt.Axis(labelAngle=-20, labelLimit=120)),
            y=alt.Y("value:Q", title="Permille"),
            color=alt.Color("metric:N", legend=alt.Legend(title="Metric")),
            column=alt.Column("metric:N", title=None),
            tooltip=["regime:N", "metric:N", "severity:Q", "value:Q"],
        )
        .properties(width=width // 2, height=height, title=title)
    )
    return _configure_chart(chart)


def scale_validation_panels(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values: list[dict[str, object]] = []
    for row in rows:
        regime = display_label(str(row["scenario_regime"]))
        values.extend(
            [
                {"regime": regime, "metric": "runtime ms", "value": int_value(row, "runtime_ms")},
                {"regime": regime, "metric": "memory KiB", "value": int_value(row, "memory_kib")},
                {"regime": regime, "metric": "quality", "value": int_value(row, "quality_per_byte_permille")},
                {"regime": regime, "metric": "failure rate", "value": int_value(row, "failure_rate_permille")},
            ]
        )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_boxplot(size=42)
        .encode(
            x=alt.X("regime:N", title=None, axis=alt.Axis(labelAngle=-20)),
            y=alt.Y("value:Q", title="Metric value"),
            color=alt.Color("regime:N", legend=None),
            column=alt.Column("metric:N", title=None),
            tooltip=["regime:N", "metric:N", "value:Q"],
        )
        .properties(width=width // 4, height=height, title=title)
    )
    return _configure_chart(chart)


def baseline_boxplot(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values = [
        {
            "baseline": display_label(str(row["baseline_policy"])),
            "accuracy": int_value(row, "decision_accuracy_permille"),
            "quality": int_value(row, "quality_per_byte_permille"),
        }
        for row in rows
    ]
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_boxplot(size=42)
        .encode(
            x=alt.X("baseline:N", title=None, axis=alt.Axis(labelAngle=-20, labelLimit=120)),
            y=alt.Y("accuracy:Q", title="Decision accuracy (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("baseline:N", legend=None),
            tooltip=["baseline:N", "accuracy:Q", "quality:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def line_by_mode(
    title: str,
    rows: list[dict[str, object]],
    width: int,
    height: int,
    *,
    x_field: str,
    y_field: str,
    color_field: str,
    y_title: str,
) -> alt.TopLevelMixin:
    values = [
        {
            "round": int_value(row, x_field),
            "value": int_value(row, y_field),
            "mode": display_label(str(row[color_field])),
        }
        for row in rows
    ]
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("round:Q", title="Round", axis=alt.Axis(tickMinStep=1)),
        y=alt.Y("value:Q", title=y_title, scale=alt.Scale(domain=[0, 1000])),
        color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
        tooltip=["mode:N", "round:Q", "value:Q"],
    )
    chart = base.mark_line(point=True, strokeWidth=2.4).properties(
        width=width,
        height=height,
        title=title,
    )
    return _configure_chart(chart)


def grouped_bar(
    title: str,
    rows: list[dict[str, object]],
    width: int,
    height: int,
    *,
    x_field: str,
    y_fields: list[str],
    y_title: str,
) -> alt.TopLevelMixin:
    values: list[dict[str, object]] = []
    for row in rows:
        for metric in y_fields:
            values.append(
                {
                    "condition": display_label(str(row[x_field])),
                    "metric": metric_label(metric),
                    "value": int_value(row, metric),
                }
            )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_bar()
        .encode(
            x=alt.X("condition:N", title=None, axis=alt.Axis(labelAngle=0, labelLimit=130)),
            xOffset=alt.XOffset("metric:N"),
            y=alt.Y("value:Q", title=y_title),
            color=alt.Color(
                "metric:N",
                scale=alt.Scale(range=[ENGINE_COLORS["pathway"], ENGINE_COLORS["scatter"]]),
                legend=alt.Legend(title="Metric"),
            ),
            tooltip=["condition:N", "metric:N", "value:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def contribution_bar(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values: list[dict[str, object]] = []
    for row in rows:
        condition = display_label(str(row["policy_or_mode"]))
        values.extend(
            [
                {"condition": condition, "metric": "useful contributions", "value": int_value(row, "useful_contribution_count")},
                {"condition": condition, "metric": "available evidence", "value": int_value(row, "available_evidence_count")},
                {"condition": condition, "metric": "quality / 100", "value": int_value(row, "merged_statistic_quality_permille") // 100},
            ]
        )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_bar()
        .encode(
            x=alt.X("condition:N", title=None, axis=alt.Axis(labelAngle=0)),
            xOffset=alt.XOffset("metric:N"),
            y=alt.Y("value:Q", title="Count or scaled quality"),
            color=alt.Color("metric:N", legend=alt.Legend(title="Evidence metric")),
            tooltip=["condition:N", "metric:N", "value:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def active_belief_grid(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values = [
        {
            "regime": display_label(str(row["scenario_regime"])),
            "task": display_label(str(row["task_kind"])),
            "mode": display_label(str(row["mode"])),
            "quality": int_value(row, "quality_per_byte_permille"),
            "agreement": int_value(row, "receiver_agreement_permille"),
        }
        for row in rows
        if row["mode"] == "full-active-belief"
    ]
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("task:N", title=None, axis=alt.Axis(labelAngle=0)),
        y=alt.Y("regime:N", title=None),
    )
    heatmap = base.mark_rect().encode(
        color=alt.Color("quality:Q", legend=None, scale=alt.Scale(scheme="tealblues")),
        tooltip=["regime:N", "task:N", "quality:Q", "agreement:Q"],
    )
    labels = base.mark_text(font=PLOT_FONT, fontSize=10).encode(text="quality:Q")
    return _configure_chart((heatmap + labels).properties(width=width, height=height, title=title))


def phase_heatmap(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values = [
        {
            "band": display_label(str(row["scenario_id"])),
            "budget": f"budget {row['forwarding_budget']}",
            "quality": int_value(row, "quality_permille"),
            "duplicate_rate": int_value(row, "duplicate_rate_permille"),
            "r_est": int_value(row, "r_est_permille"),
        }
        for row in rows
    ]
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("budget:N", title=None),
        y=alt.Y("band:N", title=None),
    )
    heatmap = base.mark_rect().encode(
        color=alt.Color(
            "quality:Q",
            legend=None,
            scale=alt.Scale(domain=[400, 920], scheme="yellowgreenblue"),
        ),
        tooltip=["band:N", "budget:N", "quality:Q", "duplicate_rate:Q", "r_est:Q"],
    )
    labels = base.mark_text(font=PLOT_FONT, fontSize=10).encode(text="quality:Q")
    return _configure_chart((heatmap + labels).properties(width=width, height=height, title=title))


def active_vs_passive(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    grouped: dict[tuple[str, str], list[int]] = defaultdict(list)
    for row in rows:
        if str(row["mode"]) in {"passive-controlled-coded", "full-active-belief"}:
            grouped[(str(row["scenario_regime"]), str(row["mode"]))].append(
                int_value(row, "quality_per_byte_permille")
            )
    values = [
        {
            "regime": display_label(regime),
            "mode": display_label(mode),
            "quality": sum(scores) // len(scores),
        }
        for (regime, mode), scores in grouped.items()
    ]
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_bar()
        .encode(
            x=alt.X("regime:N", title=None, axis=alt.Axis(labelAngle=0)),
            xOffset=alt.XOffset("mode:N"),
            y=alt.Y("quality:Q", title="Mean quality per byte (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
            tooltip=["regime:N", "mode:N", "quality:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def recoding_frontier(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    grouped: dict[tuple[str, str], list[int]] = defaultdict(list)
    for row in rows:
        if str(row["mode"]) in {"passive-controlled-coded", "full-active-belief", "recoded-aggregate"}:
            grouped[(str(row["task_kind"]), str(row["mode"]))].append(
                int_value(row, "quality_per_byte_permille")
            )
    values = [
        {"task": display_label(task), "mode": display_label(mode), "quality": sum(scores) // len(scores)}
        for (task, mode), scores in grouped.items()
    ]
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_line(point=True, strokeWidth=2.4)
        .encode(
            x=alt.X("task:N", title=None, axis=alt.Axis(labelAngle=0)),
            y=alt.Y("quality:Q", title="Quality per byte (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
            tooltip=["task:N", "mode:N", "quality:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def robustness_boundary(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values: list[dict[str, object]] = []
    for row in rows:
        regime = display_label(str(row["scenario_regime"]))
        values.extend(
            [
                {"regime": regime, "metric": "commitment accuracy", "value": int_value(row, "commitment_accuracy_permille")},
                {"regime": regime, "metric": "false commitment", "value": int_value(row, "false_commitment_rate_permille")},
            ]
        )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_bar()
        .encode(
            x=alt.X("regime:N", title=None, axis=alt.Axis(labelAngle=0, labelLimit=120)),
            xOffset=alt.XOffset("metric:N"),
            y=alt.Y("value:Q", title="Robustness metric (permille)"),
            color=alt.Color("metric:N", legend=alt.Legend(title="Metric")),
            tooltip=["regime:N", "metric:N", "value:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def demand_safety_matrix(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    safety_fields = [
        "evidence_validity_changed",
        "contribution_identity_created",
        "merge_semantics_changed",
        "route_truth_published",
        "duplicate_rank_inflation",
    ]
    values: list[dict[str, object]] = []
    for row in rows:
        surface = display_label(str(row["execution_surface"]))
        for field in safety_fields:
            violation = bool_value(row, field)
            values.append(
                {
                    "surface": surface,
                    "boundary": metric_label(field),
                    "status": 0 if violation else 1,
                    "label": "safe" if not violation else "violation",
                    "color_label": "safe" if not violation else "violation",
                }
            )
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("boundary:N", title=None, axis=alt.Axis(labelAngle=-25)),
        y=alt.Y("surface:N", title=None),
    )
    heatmap = base.mark_rect().encode(
        color=alt.Color(
            "color_label:N",
            title="Boundary status",
            scale=alt.Scale(domain=["violation", "safe"], range=["#b91c1c", "#167C72"]),
        ),
        tooltip=["surface:N", "boundary:N", "label:N"],
    )
    labels = base.mark_text(font=PLOT_FONT, fontSize=10).encode(text="label:N")
    return _configure_chart((heatmap + labels).properties(width=width, height=height, title=title))


def theorem_assumption_matrix(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values = [
        {
            "theorem": compact_theorem(str(row["theorem_name"])),
            "regime": display_label(str(row["scenario_regime"])),
            "status": 1 if row["assumption_status"] == "holds" else 0,
            "label": str(row["assumption_status"]),
            "color_label": str(row["assumption_status"]),
        }
        for row in rows
    ]
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("regime:N", title=None, axis=alt.Axis(labelAngle=0)),
        y=alt.Y("theorem:N", title=None),
    )
    heatmap = base.mark_rect().encode(
        color=alt.Color(
            "color_label:N",
            title="Assumption status",
            scale=alt.Scale(domain=["empirical-only", "holds"], range=["#cbd5e1", "#167C72"]),
        ),
        tooltip=["theorem:N", "regime:N", "label:N"],
    )
    labels = base.mark_text(font=PLOT_FONT, fontSize=9).encode(text="label:N")
    return _configure_chart((heatmap + labels).properties(width=width, height=height, title=title))


def trace_validation_matrix(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    fields = ["canonical_preprocessing", "replay_deterministic", "external_or_semi_realistic"]
    values: list[dict[str, object]] = []
    for row in rows:
        trace = display_label(str(row["trace_family"]))
        for field in fields:
            ok = bool_value(row, field)
            values.append(
                {
                    "trace": trace,
                    "check": metric_label(field),
                    "status": 1 if ok else 0,
                    "label": "yes" if ok else "no",
                    "color_label": "yes" if ok else "no",
                }
            )
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("check:N", title=None, axis=alt.Axis(labelAngle=0)),
        y=alt.Y("trace:N", title=None),
    )
    heatmap = base.mark_rect().encode(
        color=alt.Color(
            "color_label:N",
            title="Check",
            scale=alt.Scale(domain=["no", "yes"], range=["#cbd5e1", "#167C72"]),
        ),
        tooltip=["trace:N", "check:N", "label:N"],
    )
    labels = base.mark_text(font=PLOT_FONT, fontSize=10).encode(text="label:N")
    return _configure_chart((heatmap + labels).properties(width=width, height=height, title=title))


def headline_values(rows: list[dict[str, object]], dataset: str) -> tuple[list[int], list[str]]:
    metric = metric_for_dataset(dataset)
    values: list[int] = []
    labels: list[str] = []
    for index, row in enumerate(rows[:4]):
        value = row.get(metric, index + 1)
        if isinstance(value, bool):
            value = 1000 if value else 0
        values.append(int(value))
        labels.append(
            display_label(
                str(
                    row.get("policy_or_mode")
                    or row.get("mode")
                    or row.get("task_kind")
                    or row.get("scenario_regime")
                    or row.get("baseline_policy")
                    or row.get("trace_family")
                    or index
                )
            )
        )
    return values, labels


def metric_for_dataset(dataset: str) -> str:
    return {
        "coded_inference_experiment_a_landscape.csv": "merged_statistic_quality_permille",
        "coded_inference_experiment_a2_evidence_modes.csv": "merged_statistic_quality_permille",
        "coded_inference_experiment_b_path_free_recovery.csv": "path_free_success_permille",
        "coded_inference_experiment_c_phase_diagram.csv": "quality_permille",
        "coded_inference_experiment_d_coding_vs_replication.csv": "quality_permille",
        "coded_inference_experiment_e_observer_frontier.csv": "uncertainty_permille",
        "active_belief_raw_rounds.csv": "merged_statistic_quality_permille",
        "active_belief_receiver_runs.csv": "quality_per_byte_permille",
        "active_belief_path_validation.csv": "path_free_success_permille",
        "active_belief_demand_ablation.csv": "quality_per_byte_permille",
        "active_belief_scale_validation.csv": "quality_per_byte_permille",
        "active_belief_second_tasks.csv": "decision_accuracy_permille",
        "active_belief_host_bridge_demand.csv": "demand_contribution_count",
        "active_belief_theorem_assumptions.csv": "receiver_arrival_bound_permille",
        "active_belief_large_regime.csv": "executed_node_count",
        "active_belief_trace_validation.csv": "replay_deterministic",
        "active_belief_strong_baselines.csv": "quality_per_byte_permille",
        "active_belief_exact_seed_summary.csv": "commitment_accuracy_permille",
        "active_belief_final_validation.csv": "quality_per_byte_permille",
    }[dataset]


def int_value(row: dict[str, object], field: str) -> int:
    value = row.get(field, 0)
    if isinstance(value, bool):
        return 1 if value else 0
    return int(value)


def quantile(values: list[int], fraction: float) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    index = round((len(ordered) - 1) * fraction)
    return int(ordered[index])


def scaled(value: int, maximum: int) -> int:
    if maximum <= 0:
        return 0
    return int(value * 1000 // maximum)


def bool_value(row: dict[str, object], field: str) -> bool:
    value = row.get(field, False)
    if isinstance(value, bool):
        return value
    return str(value).lower() == "true"


def display_label(label: str) -> str:
    replacements = {
        "passive-controlled-coded": "passive coded",
        "full-active-belief": "active belief",
        "recoded-aggregate": "recoded aggregate",
        "no-demand": "no demand",
        "local-only-demand": "local only",
        "propagated-demand": "propagated",
        "stale-demand": "stale demand",
        "no-duplicate-risk": "no duplicate risk",
        "no-bridge-value": "no bridge value",
        "no-landscape-value": "no landscape value",
        "no-reproduction-control": "no reproduction control",
        "source-coded-threshold": "source coded",
        "distributed-local-evidence": "distributed evidence",
        "uncoded-replication": "uncoded replication",
        "epidemic-forwarding": "epidemic",
        "random-forwarding": "random",
        "prophet-contact-frequency": "contact frequency",
        "spray-and-wait": "spray and wait",
        "set-union-threshold": "set union",
        "anomaly-localization": "anomaly",
        "majority-threshold": "majority",
        "bounded-histogram": "histogram",
        "sparse-bridge-heavy": "sparse bridge",
        "clustered-duplicate-heavy": "clustered",
        "semi-realistic-mobility": "mobility",
        "semi-realistic-mobility-contact": "mobility",
        "host-bridge-replay": "host bridge replay",
        "simulator-local": "simulator local",
        "stale-demand-ablation": "stale demand",
        "128-node-sparse-bridge": "128 sparse",
        "256-node-clustered": "256 clustered",
        "500-node-mobility-contact": "500 mobility",
        "malicious-duplicate-pressure": "malicious duplicates",
        "delayed-demand": "delayed demand",
        "canonical_preprocessing": "canonical preprocessing",
        "replay_deterministic": "deterministic replay",
        "external_or_semi_realistic": "semi-real",
    }
    return replacements.get(label, label.replace("-", " "))


def metric_label(metric: str) -> str:
    labels = {
        "path_free_success_permille": "path-free success",
        "recovery_probability_permille": "recovery probability",
        "decision_accuracy_permille": "decision accuracy",
        "quality_per_byte_permille": "quality per byte",
        "quality_permille": "quality",
        "equal_cost_quality_improvement_permille": "equal-cost gain",
        "observer_advantage_permille": "observer advantage",
        "uncertainty_permille": "uncertainty",
        "requested_node_count": "requested nodes",
        "executed_node_count": "executed nodes",
        "evidence_validity_changed": "evidence validity changed",
        "contribution_identity_created": "contribution identity created",
        "merge_semantics_changed": "merge semantics changed",
        "route_truth_published": "route truth published",
        "duplicate_rank_inflation": "duplicate rank inflation",
        "canonical_preprocessing": "canonical preprocessing",
        "replay_deterministic": "deterministic replay",
        "external_or_semi_realistic": "semi-realistic input",
    }
    return labels.get(metric, metric.replace("_", " ").replace(" permille", ""))


def compact_theorem(theorem: str) -> str:
    replacements = {
        "receiver_arrival_reconstruction_bound": "receiver arrival",
        "useful_inference_arrival_bound": "useful inference",
        "anomaly_margin_lower_tail_bound": "anomaly margin",
        "guarded_commitment_false_probability_bounded": "false commitment",
        "inference_potential_drift_progress": "potential drift",
    }
    return replacements.get(theorem, theorem.replace("_", " "))
