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
    png_path = report_dir / f"{figure_id}.png"
    chart.save(str(svg_path))
    chart.save(str(png_path))
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
        return observer_proxy_boxplots(title, rows, width, height)
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
    chart = alt.layer(band, line).properties(width=width // 4, height=height).facet(
        column=alt.Column("metric:N", title=None),
    )
    chart = chart.properties(title=title)
    return _configure_chart(chart)


def path_free_distribution(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    mode_order = ["active belief", "passive coded", "recoded aggregate", "uncoded"]
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
            x=alt.X("mode:N", title=None, sort=mode_order, axis=alt.Axis(labelAngle=-20)),
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
        .properties(width=width // 6, height=height, title=title)
    )
    return _configure_chart(chart)


def receiver_metric_grid(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    metric_fields = [
        ("quality per byte", "quality_per_byte_permille"),
        ("receiver agreement", "receiver_agreement_permille"),
        ("collective uncertainty", "collective_uncertainty_permille"),
        ("commitment lead time", "commitment_lead_time_rounds"),
    ]
    mode_order = [
        "active belief",
        "passive coded",
        "recoded aggregate",
        "uncoded",
    ]
    values = []
    for row in rows:
        mode = str(row["mode"])
        if mode not in {
            "full-active-belief",
            "passive-controlled-coded",
            "recoded-aggregate",
            "uncoded-replication",
        }:
            continue
        regime = display_label(str(row["scenario_id"]))
        for metric, field in metric_fields:
            values.append(
                {
                    "regime": regime,
                    "mode": display_label(mode),
                    "metric": metric,
                    "value": int_value(row, field),
                }
            )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_boxplot(size=22)
        .encode(
            x=alt.X("mode:N", title=None, sort=mode_order, axis=alt.Axis(labelAngle=-20, labelLimit=100)),
            y=alt.Y("value:Q", title=None),
            color=alt.Color("mode:N", legend=None),
            row=alt.Row("regime:N", title=None, sort=["clustered", "mobility", "sparse bridge"]),
            column=alt.Column("metric:N", title=None),
            tooltip=["regime:N", "mode:N", "metric:N", "value:Q"],
        )
        .properties(width=width // 5, height=max(90, height // 3), title=title)
        .resolve_scale(y="independent")
    )
    return _configure_chart(chart)


def task_baseline_distribution(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    mode_order = ["active belief", "passive coded", "recoded aggregate", "uncoded"]
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
            color=alt.Color("mode:N", sort=mode_order, legend=alt.Legend(title="Mode")),
            xOffset=alt.XOffset("mode:N", sort=mode_order),
            tooltip=["task:N", "mode:N", "accuracy:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def phase_small_multiples(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    grouped: dict[tuple[str, int], dict[str, list[int]]] = defaultdict(lambda: defaultdict(list))
    for row in rows:
        band = display_label(str(row["scenario_id"]))
        budget = int_value(row, "forwarding_budget")
        for field in [
            "r_est_permille",
            "quality_permille",
            "duplicate_rate_permille",
            "byte_count",
            "recovery_probability_permille",
        ]:
            grouped[(band, budget)][field].append(int_value(row, field))
    values = []
    for (band, budget), metrics in grouped.items():
        label = f"{band.split()[0]}-{budget}"
        values.append(
            {
                "band": band,
                "budget": f"budget {budget}",
                "label": label,
                "r_est": quantile(metrics["r_est_permille"], 0.50),
                "quality": quantile(metrics["quality_permille"], 0.50),
                "duplicate": quantile(metrics["duplicate_rate_permille"], 0.50),
                "bytes": quantile(metrics["byte_count"], 0.50),
                "recovery": quantile(metrics["recovery_probability_permille"], 0.50),
            }
        )
    point_encoding = {
        "color": alt.Color("band:N", legend=alt.Legend(title="Control band")),
        "shape": alt.Shape("budget:N", legend=alt.Legend(title="Forwarding budget")),
        "tooltip": [
            "band:N",
            "budget:N",
            alt.Tooltip("r_est:Q", title="median R_est"),
            alt.Tooltip("quality:Q", title="median quality"),
            alt.Tooltip("duplicate:Q", title="median duplicate"),
            alt.Tooltip("bytes:Q", title="median bytes"),
            alt.Tooltip("recovery:Q", title="median recovery"),
        ],
    }
    data = alt.InlineData(values=values)
    target_band = (
        alt.Chart(alt.InlineData(values=[{"x1": 900, "x2": 1100, "y1": 450, "y2": 900}]))
        .mark_rect(color="#bbf7d0", opacity=0.22)
        .encode(x="x1:Q", x2="x2:Q", y="y1:Q", y2="y2:Q")
    )
    re_panel = alt.Chart(data).encode(
        x=alt.X("r_est:Q", title="Median R_est", scale=alt.Scale(domain=[700, 1450])),
        y=alt.Y("quality:Q", title="Median quality (permille)", scale=alt.Scale(domain=[450, 900])),
        **point_encoding,
    )
    re_points = re_panel.mark_point(filled=True, size=120)
    re_labels = re_panel.mark_text(dx=10, dy=-8, font=PLOT_FONT, fontSize=9, color="#334155").encode(text="label:N")
    duplicate_panel = alt.Chart(data).encode(
        x=alt.X("duplicate:Q", title="Median duplicate rate (permille)", scale=alt.Scale(domain=[70, 520])),
        y=alt.Y("quality:Q", title="Median quality (permille)", scale=alt.Scale(domain=[450, 900])),
        **point_encoding,
    )
    duplicate_points = duplicate_panel.mark_point(filled=True, size=120)
    duplicate_labels = duplicate_panel.mark_text(dx=10, dy=-8, font=PLOT_FONT, fontSize=9, color="#334155").encode(text="label:N")
    byte_panel = alt.Chart(data).encode(
        x=alt.X("bytes:Q", title="Median bytes", scale=alt.Scale(domain=[2200, 4150])),
        y=alt.Y("quality:Q", title="Median quality (permille)", scale=alt.Scale(domain=[450, 900])),
        **point_encoding,
    )
    byte_points = byte_panel.mark_point(filled=True, size=120)
    byte_labels = byte_panel.mark_text(dx=10, dy=-8, font=PLOT_FONT, fontSize=9, color="#334155").encode(text="label:N")
    chart = alt.hconcat(
        (target_band + re_points + re_labels).properties(width=width // 4, height=height, title="Target-band entry"),
        (duplicate_points + duplicate_labels).properties(width=width // 4, height=height, title="Duplicate pressure"),
        (byte_points + byte_labels).properties(width=width // 4, height=height, title="Byte cost"),
    ).properties(title=title)
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
            x=alt.X("policy:N", title=None, axis=alt.Axis(labelAngle=-25, labelLimit=90)),
            y=alt.Y("quality:Q", title="Quality per byte (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("policy:N", legend=None),
            tooltip=["policy:N", "quality:Q", "satisfaction:Q", "lag:Q"],
        )
        .properties(width=width - 150, height=height, title=title)
    )
    return _configure_chart(box)


def coding_cost_curve(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    grouped: dict[tuple[str, int], list[int]] = defaultdict(list)
    for row in rows:
        grouped[(display_label(str(row["policy_or_mode"])), int_value(row, "fixed_payload_budget_bytes"))].append(
            int_value(row, "quality_permille")
        )
    values = [
        {
            "mode": mode,
            "budget": budget,
            "low": quantile(scores, 0.25),
            "median": quantile(scores, 0.50),
            "high": quantile(scores, 0.75),
        }
        for (mode, budget), scores in grouped.items()
    ]
    base = alt.Chart(alt.InlineData(values=values)).encode(
        x=alt.X("budget:Q", title="Payload-byte budget"),
        color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
        tooltip=["mode:N", "budget:Q", "low:Q", "median:Q", "high:Q"],
    )
    band = base.mark_area(opacity=0.14).encode(
        y=alt.Y("high:Q", title="Quality (permille)", scale=alt.Scale(domain=[350, 1000])),
        y2="low:Q",
    )
    line = base.mark_line(point=True, strokeWidth=2.4).encode(y="median:Q")
    chart = alt.layer(band, line).properties(width=width, height=height, title=title)
    return _configure_chart(chart)


def recoding_frontier_scatter(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    grouped: dict[tuple[str, str], dict[str, list[int]]] = defaultdict(lambda: defaultdict(list))
    for row in rows:
        mode = str(row["mode"])
        if mode not in {"passive-controlled-coded", "full-active-belief", "recoded-aggregate"}:
            continue
        regime = display_label(str(row["scenario_id"]))
        grouped[(regime, display_label(mode))]["bytes"].append(int_value(row, "bytes_at_commitment"))
        grouped[(regime, display_label(mode))]["quality"].append(int_value(row, "quality_per_byte_permille"))
    values = []
    for (regime, mode), metrics in grouped.items():
        values.append(
            {
                "regime": regime,
                "mode": mode,
                "bytes_low": quantile(metrics["bytes"], 0.25),
                "bytes_median": quantile(metrics["bytes"], 0.50),
                "bytes_high": quantile(metrics["bytes"], 0.75),
                "quality_low": quantile(metrics["quality"], 0.25),
                "quality_median": quantile(metrics["quality"], 0.50),
                "quality_high": quantile(metrics["quality"], 0.75),
            }
        )
    data = alt.InlineData(values=values)
    horizontal = (
        alt.Chart(data)
        .mark_rule(strokeWidth=2.2)
        .encode(
            x=alt.X("bytes_low:Q", title="Bytes at commitment", scale=alt.Scale(domain=[1900, 2120])),
            x2="bytes_high:Q",
            y=alt.Y("quality_median:Q", title="Quality per byte (permille)", scale=alt.Scale(domain=[720, 980])),
            color=alt.Color("mode:N", legend=alt.Legend(title="Mode")),
            tooltip=["regime:N", "mode:N", "bytes_median:Q", "quality_median:Q"],
        )
    )
    vertical = (
        alt.Chart(data)
        .mark_rule(strokeWidth=2.2)
        .encode(
            x=alt.X("bytes_median:Q", scale=alt.Scale(domain=[1900, 2120])),
            y=alt.Y("quality_low:Q", scale=alt.Scale(domain=[720, 980])),
            y2="quality_high:Q",
            color=alt.Color("mode:N", legend=None),
            tooltip=["regime:N", "mode:N", "bytes_median:Q", "quality_median:Q"],
        )
    )
    points = (
        alt.Chart(data)
        .mark_point(filled=True, size=125)
        .encode(
            x=alt.X("bytes_median:Q", scale=alt.Scale(domain=[1900, 2120])),
            y=alt.Y("quality_median:Q", scale=alt.Scale(domain=[720, 980])),
            color=alt.Color("mode:N", legend=None),
            shape=alt.Shape("mode:N", legend=None),
            tooltip=["regime:N", "mode:N", "bytes_median:Q", "quality_median:Q"],
        )
    )
    labels = (
        alt.Chart(data)
        .mark_text(dx=9, dy=-7, font=PLOT_FONT, fontSize=9, color="#334155")
        .encode(
            x=alt.X("bytes_median:Q", scale=alt.Scale(domain=[1900, 2120])),
            y=alt.Y("quality_median:Q", scale=alt.Scale(domain=[720, 980])),
            text="mode:N",
        )
    )
    chart = alt.layer(horizontal, vertical, points, labels).properties(
        width=width // 4,
        height=height,
        title=title,
    ).facet(column=alt.Column("regime:N", title=None, sort=["clustered", "mobility", "sparse bridge"]))
    return _configure_chart(chart)


def robustness_small_multiples(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    accuracy_values: list[dict[str, object]] = []
    false_values: list[dict[str, object]] = []
    for row in rows:
        regime = display_label(str(row["scenario_regime"]))
        severity = int_value(row, "stress_severity")
        accuracy_values.append(
            {
                "regime": regime,
                "severity": severity,
                "value": int_value(row, "commitment_accuracy_permille"),
            }
        )
        false_values.append(
            {
                "regime": regime,
                "severity": severity,
                "value": int_value(row, "false_commitment_rate_permille"),
            }
        )
    accuracy = (
        alt.Chart(alt.InlineData(values=accuracy_values))
        .mark_boxplot(size=34)
        .encode(
            x=alt.X("regime:N", title=None, axis=alt.Axis(labelAngle=-20, labelLimit=120)),
            y=alt.Y("value:Q", title="Commitment accuracy (permille)", scale=alt.Scale(domain=[650, 1000])),
            color=alt.value(ENGINE_COLORS["pathway"]),
            tooltip=["regime:N", "severity:Q", "value:Q"],
        )
        .properties(width=width // 3, height=height, title="Commitment accuracy")
    )
    false_commitment = (
        alt.Chart(alt.InlineData(values=false_values))
        .mark_boxplot(size=34)
        .encode(
            x=alt.X("regime:N", title=None, axis=alt.Axis(labelAngle=-20, labelLimit=120)),
            y=alt.Y("value:Q", title="False commitment (permille)", scale=alt.Scale(domain=[0, 60])),
            color=alt.value(ENGINE_COLORS["scatter"]),
            tooltip=["regime:N", "severity:Q", "value:Q"],
        )
        .properties(width=width // 3, height=height, title="False commitment")
    )
    chart = alt.hconcat(accuracy, false_commitment).properties(title=title)
    return _configure_chart(chart)


def observer_proxy_boxplots(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values: list[dict[str, object]] = []
    for row in rows:
        mode = display_label(str(row["policy_or_mode"]))
        values.extend(
            [
                {
                    "condition": mode,
                    "metric": "observer advantage",
                    "value": int_value(row, "observer_advantage_permille"),
                },
                {
                    "condition": mode,
                    "metric": "uncertainty",
                    "value": int_value(row, "uncertainty_permille"),
                },
            ]
        )
    chart = (
        alt.Chart(alt.InlineData(values=values))
        .mark_boxplot(size=42)
        .encode(
            x=alt.X("condition:N", title=None, axis=alt.Axis(labelAngle=-20)),
            y=alt.Y("value:Q", title="Proxy metric (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("metric:N", legend=alt.Legend(title="Metric")),
            xOffset=alt.XOffset("metric:N"),
            tooltip=["condition:N", "metric:N", "value:Q"],
        )
        .properties(width=width, height=height, title=title)
    )
    return _configure_chart(chart)


def scale_validation_panels(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    values_by_metric: dict[str, list[dict[str, object]]] = defaultdict(list)
    for row in rows:
        regime = display_label(str(row["scenario_regime"]))
        values_by_metric["runtime ms"].append({"regime": regime, "value": int_value(row, "runtime_ms")})
        values_by_metric["memory KiB"].append({"regime": regime, "value": int_value(row, "memory_kib")})
        values_by_metric["quality"].append({"regime": regime, "value": int_value(row, "quality_per_byte_permille")})
        values_by_metric["failure rate"].append({"regime": regime, "value": int_value(row, "failure_rate_permille")})

    def panel(metric: str, domain: list[int]) -> alt.Chart:
        return (
            alt.Chart(alt.InlineData(values=values_by_metric[metric]))
            .mark_boxplot(size=36)
            .encode(
                x=alt.X("regime:N", title=None, axis=alt.Axis(labelAngle=-20, labelLimit=95)),
                y=alt.Y("value:Q", title=metric, scale=alt.Scale(domain=domain)),
                color=alt.Color("regime:N", legend=None),
                tooltip=["regime:N", "value:Q"],
            )
            .properties(width=width // 6, height=height, title=metric)
        )

    chart = alt.hconcat(
        panel("runtime ms", [1000, 6000]),
        panel("memory KiB", [50, 360]),
        panel("quality", [650, 850]),
        panel("failure rate", [0, 15]),
    ).properties(title=title)
    return _configure_chart(chart)


def baseline_boxplot(title: str, rows: list[dict[str, object]], width: int, height: int) -> alt.TopLevelMixin:
    baseline_order = [
        "active belief",
        "passive coded",
        "contact freq",
        "epidemic",
        "spray-wait",
        "random",
        "uncoded",
    ]
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
            x=alt.X("baseline:N", title=None, sort=baseline_order, axis=alt.Axis(labelAngle=-20, labelLimit=80)),
            y=alt.Y("accuracy:Q", title="Decision accuracy (permille)", scale=alt.Scale(domain=[0, 1000])),
            color=alt.Color("baseline:N", legend=None),
            tooltip=["baseline:N", "accuracy:Q", "quality:Q"],
        )
        .properties(width=width - 140, height=height, title=title)
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
    return _configure_chart((heatmap + labels).properties(width=width - 280, height=height, title=title))


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
    return _configure_chart((heatmap + labels).properties(width=width - 280, height=height, title=title))


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
    activity_by_mode: dict[tuple[str, str], list[int]] = defaultdict(list)
    for row in rows:
        surface = display_label(str(row["execution_surface"]))
        mode = display_label(str(row["mode"]))
        activity_by_mode[(surface, mode)].append(int_value(row, "demand_contribution_count"))
    activity_values = [
        {
            "surface": surface,
            "mode": mode,
            "median_demand": quantile(counts, 0.50),
            "label": str(quantile(counts, 0.50)),
        }
        for (surface, mode), counts in activity_by_mode.items()
    ]

    activity_base = alt.Chart(alt.InlineData(values=activity_values)).encode(
        x=alt.X(
            "mode:N",
            title=None,
            sort=["passive coded", "active belief", "stale demand"],
            axis=alt.Axis(labelAngle=0, labelLimit=105),
        ),
        y=alt.Y("median_demand:Q", title="Median demand summaries per replay batch"),
        color=alt.Color(
            "surface:N",
            title="Execution surface",
            scale=alt.Scale(
                domain=["simulator local", "host bridge replay"],
                range=[ENGINE_COLORS["pathway"], ENGINE_COLORS["field"]],
            ),
        ),
        tooltip=["surface:N", "mode:N", "median_demand:Q"],
    )
    activity = activity_base.mark_bar(size=42)
    activity_labels = activity_base.mark_text(font=PLOT_FONT, fontSize=11, dy=-7).encode(text="label:N")
    return _configure_chart((activity + activity_labels).properties(width=width - 260, height=height, title=title))


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
    return _configure_chart((heatmap + labels).properties(width=width - 280, height=height, title=title))


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
    return _configure_chart((heatmap + labels).properties(width=width - 280, height=height, title=title))


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
        "active_belief_headline_statistics.csv": "paired_delta_median",
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
        "stale-demand": "stale",
        "no-duplicate-risk": "no dup risk",
        "no-bridge-value": "no bridge",
        "no-landscape-value": "no landscape",
        "no-reproduction-control": "no repro ctrl",
        "source-coded-threshold": "source coded",
        "distributed-local-evidence": "distributed evidence",
        "uncoded-replication": "uncoded",
        "epidemic-forwarding": "epidemic",
        "random-forwarding": "random",
        "prophet-contact-frequency": "contact freq",
        "spray-and-wait": "spray-wait",
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
