"""CLI entry point: load artifacts, compute tables and plots, and write the PDF report and CSV exports."""

from __future__ import annotations

import sys
from pathlib import Path

from .data import cleanup_report_dir, ensure_dir, load_json_array, load_ndjson, write_csv
from .document import write_pdf_report
from .plots import (
    render_batman_transition_loss,
    render_batman_transition_stability,
    render_comparison_summary,
    render_field_budget_reconfiguration,
    render_field_budget_route_presence,
    render_head_to_head_route_presence,
    render_pathway_budget_activation,
    render_pathway_budget_route_presence,
    save_plot_artifact,
)
from .scoring import (
    baseline_comparison_table,
    boundary_summary_table,
    comparison_summary_table,
    head_to_head_summary_table,
    profile_recommendation_table,
    recommendation_table,
    transition_metrics_table,
    write_recommendations,
)


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) != 1:
        print("usage: python -m analysis.report <artifact-dir>", file=sys.stderr)
        return 1

    artifact_dir = Path(argv[0]).resolve()
    report_dir = artifact_dir / "report"
    ensure_dir(report_dir)
    cleanup_report_dir(report_dir)

    runs = load_ndjson(artifact_dir / "runs.jsonl")
    aggregates = load_json_array(artifact_dir / "aggregates.json")
    breakdowns = load_json_array(artifact_dir / "breakdowns.json")
    if runs.is_empty() or aggregates.is_empty():
        print(f"no tuning data found in {artifact_dir}", file=sys.stderr)
        return 1

    recommendations = recommendation_table(aggregates, breakdowns)
    profile_recommendations = profile_recommendation_table(aggregates, breakdowns)
    transition_metrics = transition_metrics_table(runs, recommendations)
    boundary_summary = boundary_summary_table(recommendations, breakdowns)
    baseline_comparison, baseline_dir = baseline_comparison_table(
        artifact_dir, recommendations
    )
    comparison_summary = comparison_summary_table(aggregates)
    head_to_head_summary = head_to_head_summary_table(aggregates)

    write_csv(runs, report_dir / "runs.csv")
    write_csv(aggregates, report_dir / "aggregates.csv")
    write_csv(breakdowns, report_dir / "breakdowns.csv")
    write_csv(recommendations, report_dir / "recommendations.csv")
    write_csv(profile_recommendations, report_dir / "profile_recommendations.csv")
    write_csv(transition_metrics, report_dir / "transition_metrics.csv")
    write_csv(boundary_summary, report_dir / "boundary_summary.csv")
    write_csv(baseline_comparison, report_dir / "baseline_comparison.csv")
    write_csv(comparison_summary, report_dir / "comparison_summary.csv")
    write_csv(head_to_head_summary, report_dir / "head_to_head_summary.csv")
    write_recommendations(report_dir / "recommendations.md", recommendations)

    save_plot_artifact(
        report_dir,
        "batman_transition_stability",
        render_batman_transition_stability,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "batman_transition_loss",
        render_batman_transition_loss,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "pathway_budget_route_presence",
        render_pathway_budget_route_presence,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "pathway_budget_activation",
        render_pathway_budget_activation,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "field_budget_route_presence",
        render_field_budget_route_presence,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "field_budget_reconfiguration",
        render_field_budget_reconfiguration,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "comparison_dominant_engine",
        render_comparison_summary,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "head_to_head_route_presence",
        render_head_to_head_route_presence,
        aggregates,
    )

    write_pdf_report(
        artifact_dir,
        report_dir,
        recommendations,
        profile_recommendations,
        transition_metrics,
        boundary_summary,
        aggregates,
        comparison_summary,
        head_to_head_summary,
        baseline_comparison,
        baseline_dir,
    )
    print(f"Analysis report artifacts: {report_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
