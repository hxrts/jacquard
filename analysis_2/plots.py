"""Altair figure rendering for the active-belief paper report."""

from __future__ import annotations

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
    values, labels = headline_values(rows, dataset)
    width = int(ACTIVE_BELIEF_FIGURE_WIDTH_INCHES * PIXELS_PER_INCH)
    height = int(ACTIVE_BELIEF_FIGURE_HEIGHT_INCHES * PIXELS_PER_INCH)
    chart = render_active_belief_headline_chart(title, values, labels, width, height)
    svg_path = report_dir / f"{figure_id}.svg"
    pdf_path = report_dir / f"{figure_id}.pdf"
    chart.save(str(svg_path))
    try:
        chart.save(str(pdf_path))
    except Exception:
        drawing = svg2rlg(str(svg_path))
        renderPDF.drawToFile(drawing, str(pdf_path))
    return values, labels


def render_active_belief_headline_chart(
    title: str,
    values: list[int],
    labels: list[str],
    width: int,
    height: int,
) -> alt.TopLevelMixin:
    if not values:
        return _placeholder_chart(width, height, "No active-belief rows available")
    points = [
        {
            "label": label,
            "value": value,
            "value_label": str(value),
            "order": index,
        }
        for index, (label, value) in enumerate(zip(labels, values))
    ]
    label_order = [row["label"] for row in points]
    y_top = max(max(values), 1000 if max(values) <= 1000 else max(values))
    base = alt.Chart(alt.InlineData(values=points)).encode(
        x=alt.X(
            "label:N",
            title=None,
            sort=label_order,
            axis=alt.Axis(labelAngle=0, labelLimit=120),
        ),
        y=alt.Y(
            "value:Q",
            title=metric_axis_title(max(values)),
            scale=alt.Scale(domain=[0, y_top]),
        ),
        tooltip=[
            alt.Tooltip("label:N", title="Condition"),
            alt.Tooltip("value:Q", title="Metric"),
        ],
    )
    line = base.mark_line(
        color=ENGINE_COLORS["pathway"],
        strokeWidth=2.4,
        point=alt.OverlayMarkDef(
            filled=True,
            fill=ENGINE_COLORS["scatter"],
            size=85,
            stroke="white",
            strokeWidth=1,
        ),
    )
    labels_layer = base.mark_text(
        color=PLOT_MUTED_TEXT_COLOR,
        font=PLOT_FONT,
        fontSize=10,
        dy=-12,
    ).encode(text="value_label:N")
    chart = (line + labels_layer).properties(width=width, height=height, title=title)
    return _configure_chart(chart)


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
            compact_condition_label(
                str(
                    row.get("policy_or_mode")
                    or row.get("mode")
                    or row.get("task_kind")
                    or row.get("scenario_regime")
                    or index
                )
            )
        )
    return values, labels


def compact_condition_label(label: str) -> str:
    replacements = {
        "passive-controlled-coded": "passive\ncoded",
        "full-active-belief": "active\nbelief",
        "recoded-aggregate": "recoded\naggregate",
        "anomaly-localization": "anomaly\nlocalization",
        "majority-threshold": "majority\nthreshold",
        "bounded-histogram": "bounded\nhistogram",
        "sparse-bridge-heavy": "sparse\nbridge",
        "clustered-duplicate-heavy": "clustered\nduplicate",
        "semi-realistic-mobility": "mobility\ntrace",
    }
    return replacements.get(label, label.replace("-", "\n", 1))


def metric_axis_title(max_value: int) -> str:
    if max_value <= 1000:
        return "Metric (permille or boolean)"
    return "Metric"


def metric_for_dataset(dataset: str) -> str:
    return {
        "coded_inference_experiment_a_landscape.csv": "merged_statistic_quality_permille",
        "coded_inference_experiment_a2_evidence_modes.csv": "merged_statistic_quality_permille",
        "coded_inference_experiment_b_path_free_recovery.csv": "path_free_success_permille",
        "coded_inference_experiment_c_phase_diagram.csv": "quality_permille",
        "coded_inference_experiment_d_coding_vs_replication.csv": "quality_permille",
        "coded_inference_experiment_e_observer_frontier.csv": "uncertainty_permille",
        "active_belief_second_tasks.csv": "decision_accuracy_permille",
        "active_belief_host_bridge_demand.csv": "bridge_batch_id",
        "active_belief_theorem_assumptions.csv": "receiver_arrival_bound_permille",
        "active_belief_large_regime.csv": "executed_node_count",
        "active_belief_trace_validation.csv": "replay_deterministic",
        "active_belief_strong_baselines.csv": "quality_per_byte_permille",
        "active_belief_exact_seed_summary.csv": "quality_per_byte_permille",
        "active_belief_final_validation.csv": "quality_per_byte_permille",
    }[dataset]
