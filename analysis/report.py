"""CLI entry point: load artifacts, compute tables and plots, and write the PDF report and CSV exports."""

from __future__ import annotations

import sys
from pathlib import Path

from .data import (
    cleanup_report_dir,
    ensure_dir,
    load_json_array,
    load_ndjson,
    load_optional_json_array,
    load_optional_ndjson,
    write_csv,
)
from .document import write_pdf_report
from .plots import (
    render_babel_decay_loss,
    render_babel_decay_stability,
    render_batman_bellman_transition_loss,
    render_batman_bellman_transition_stability,
    render_batman_classic_transition_loss,
    render_batman_classic_transition_stability,
    render_comparison_summary,
    render_diffusion_delivery_coverage,
    render_diffusion_resource_boundedness,
    render_field_budget_reconfiguration,
    render_field_budget_route_presence,
    render_head_to_head_route_presence,
    render_olsrv2_decay_loss,
    render_olsrv2_decay_stability,
    render_pathway_budget_activation,
    render_pathway_budget_route_presence,
    save_plot_artifact,
)
from .scoring import (
    baseline_comparison_table,
    boundary_summary_table,
    comparison_summary_table,
    diffusion_boundary_table,
    diffusion_regime_engine_summary_table,
    diffusion_engine_comparison_table,
    diffusion_engine_summary_table,
    field_vs_best_diffusion_alternative_table,
    field_diffusion_regime_calibration_table,
    field_profile_recommendation_table,
    field_routing_regime_calibration_table,
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
    pdf_path = artifact_dir / "report.pdf"
    ensure_dir(report_dir)
    cleanup_report_dir(report_dir)
    if pdf_path.exists():
        pdf_path.unlink()

    runs = load_ndjson(artifact_dir / "runs.jsonl")
    aggregates = load_json_array(artifact_dir / "aggregates.json")
    breakdowns = load_json_array(artifact_dir / "breakdowns.json")
    diffusion_runs = load_optional_ndjson(artifact_dir / "diffusion_runs.jsonl")
    diffusion_aggregates = load_optional_json_array(artifact_dir / "diffusion_aggregates.json")
    diffusion_boundaries = load_optional_json_array(artifact_dir / "diffusion_boundaries.json")
    if runs.is_empty() or aggregates.is_empty():
        print(f"no tuning data found in {artifact_dir}", file=sys.stderr)
        return 1

    recommendations = recommendation_table(aggregates, breakdowns)
    profile_recommendations = profile_recommendation_table(aggregates, breakdowns)
    field_profile_recommendations = field_profile_recommendation_table(aggregates, breakdowns)
    field_routing_regime_calibration = field_routing_regime_calibration_table(aggregates)
    transition_metrics = transition_metrics_table(runs, recommendations)
    boundary_summary = boundary_summary_table(recommendations, breakdowns)
    baseline_comparison, baseline_dir = baseline_comparison_table(
        artifact_dir, recommendations
    )
    comparison_summary = comparison_summary_table(aggregates)
    head_to_head_summary = head_to_head_summary_table(aggregates)
    diffusion_engine_summary = diffusion_engine_summary_table(diffusion_aggregates)
    diffusion_regime_engine_summary = diffusion_regime_engine_summary_table(
        diffusion_aggregates
    )
    diffusion_engine_comparison = diffusion_engine_comparison_table(diffusion_aggregates)
    diffusion_boundary_summary = diffusion_boundary_table(diffusion_boundaries)
    field_diffusion_regime_calibration = field_diffusion_regime_calibration_table(
        diffusion_aggregates
    )
    field_vs_best_diffusion_alternative = field_vs_best_diffusion_alternative_table(
        diffusion_aggregates, field_diffusion_regime_calibration
    )

    write_csv(runs, report_dir / "runs.csv")
    write_csv(aggregates, report_dir / "aggregates.csv")
    write_csv(breakdowns, report_dir / "breakdowns.csv")
    write_csv(recommendations, report_dir / "recommendations.csv")
    write_csv(profile_recommendations, report_dir / "profile_recommendations.csv")
    write_csv(
        field_profile_recommendations,
        report_dir / "field_profile_recommendations.csv",
    )
    write_csv(
        field_routing_regime_calibration,
        report_dir / "field_routing_regime_calibration.csv",
    )
    write_csv(transition_metrics, report_dir / "transition_metrics.csv")
    write_csv(boundary_summary, report_dir / "boundary_summary.csv")
    write_csv(baseline_comparison, report_dir / "baseline_comparison.csv")
    write_csv(comparison_summary, report_dir / "comparison_summary.csv")
    write_csv(head_to_head_summary, report_dir / "head_to_head_summary.csv")
    write_csv(diffusion_runs, report_dir / "diffusion_runs.csv")
    write_csv(diffusion_aggregates, report_dir / "diffusion_aggregates.csv")
    write_csv(diffusion_boundaries, report_dir / "diffusion_boundaries.csv")
    write_csv(diffusion_engine_summary, report_dir / "diffusion_engine_summary.csv")
    write_csv(
        diffusion_regime_engine_summary,
        report_dir / "diffusion_regime_engine_summary.csv",
    )
    write_csv(
        diffusion_engine_comparison,
        report_dir / "diffusion_engine_comparison.csv",
    )
    write_csv(diffusion_boundary_summary, report_dir / "diffusion_boundary_summary.csv")
    write_csv(
        field_diffusion_regime_calibration,
        report_dir / "field_diffusion_regime_calibration.csv",
    )
    write_csv(
        field_vs_best_diffusion_alternative,
        report_dir / "field_vs_best_diffusion_alternative.csv",
    )
    write_recommendations(report_dir / "recommendations.md", recommendations)

    save_plot_artifact(
        report_dir,
        "batman_bellman_transition_stability",
        render_batman_bellman_transition_stability,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "batman_bellman_transition_loss",
        render_batman_bellman_transition_loss,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "batman_classic_transition_stability",
        render_batman_classic_transition_stability,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "batman_classic_transition_loss",
        render_batman_classic_transition_loss,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "babel_decay_stability",
        render_babel_decay_stability,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "babel_decay_loss",
        render_babel_decay_loss,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "olsrv2_decay_stability",
        render_olsrv2_decay_stability,
        aggregates,
    )
    save_plot_artifact(
        report_dir,
        "olsrv2_decay_loss",
        render_olsrv2_decay_loss,
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
    if not diffusion_engine_comparison.is_empty():
        save_plot_artifact(
            report_dir,
            "diffusion_delivery_coverage",
            render_diffusion_delivery_coverage,
            diffusion_engine_comparison,
        )
        save_plot_artifact(
            report_dir,
            "diffusion_resource_boundedness",
            render_diffusion_resource_boundedness,
            diffusion_engine_comparison,
        )

    write_pdf_report(
        artifact_dir,
        report_dir,
        pdf_path,
        recommendations,
        profile_recommendations,
        field_profile_recommendations,
        field_routing_regime_calibration,
        transition_metrics,
        boundary_summary,
        aggregates,
        comparison_summary,
        head_to_head_summary,
        diffusion_engine_summary,
        diffusion_regime_engine_summary,
        diffusion_engine_comparison,
        diffusion_boundary_summary,
        field_diffusion_regime_calibration,
        field_vs_best_diffusion_alternative,
        baseline_comparison,
        baseline_dir,
    )
    print(f"Analysis report artifacts: {report_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
