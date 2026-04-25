"""CLI entry point: load artifacts, compute tables and plots, and write the PDF report and CSV exports."""

from __future__ import annotations

import shutil
import sys
import tempfile
from pathlib import Path

import polars as pl

from .data import (
    cleanup_report_dir,
    ensure_dir,
    load_csv,
    load_json_array,
    load_ndjson,
    load_optional_json_array,
    load_optional_csv,
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
    render_large_population_diffusion_transitions,
    render_large_population_route_fragility,
    render_large_population_route_scaling,
    render_head_to_head_timing_profile,
    render_head_to_head_route_presence,
    render_mixed_vs_standalone_divergence,
    render_olsrv2_decay_loss,
    render_olsrv2_decay_stability,
    render_pathway_budget_activation,
    render_pathway_budget_route_presence,
    render_recommended_engine_robustness,
    render_routing_fitness_crossover,
    render_routing_fitness_multiflow,
    render_routing_fitness_stale_repair,
    render_scatter_profile_runtime,
    render_scatter_profile_route_presence,
    save_plot_artifact,
)
from .constants import ROUTE_VISIBLE_ENGINE_SET_ORDER
from .scoring import (
    baseline_comparison_table,
    benchmark_profile_audit_table,
    comparison_config_sensitivity_table,
    comparison_engine_round_breakdown_table,
    boundary_summary_table,
    comparison_summary_table,
    diffusion_baseline_audit_table,
    diffusion_boundary_table,
    diffusion_regime_engine_summary_table,
    diffusion_engine_comparison_table,
    diffusion_engine_summary_table,
    diffusion_family_weight_sensitivity_table,
    large_population_diffusion_state_points_table,
    large_population_diffusion_transition_table,
    large_population_route_summary_table,
    head_to_head_summary_table,
    profile_recommendation_table,
    recommendation_table,
    routing_fitness_crossover_summary_table,
    routing_fitness_multiflow_summary_table,
    routing_fitness_stale_repair_summary_table,
    transition_metrics_table,
    write_recommendations,
)
from .sanity import validate_report_artifacts_or_raise

REPORT_PDF_NAME = "router-tuning-report.pdf"
LEGACY_REPORT_PDF_NAMES = ("report.pdf", "tuning_report.pdf", "routing-tuning-report.pdf")
RUN_FAILURE_CLASS_COLUMNS = (
    "no_candidate_count",
    "inadmissible_candidate_count",
    "lost_reachability_count",
    "replacement_loop_count",
    "maintenance_failure_count",
    "activation_attempt_failure_count",
    "persistent_degraded_count",
    "other_failure_count",
)
AGGREGATE_FAILURE_CLASS_COLUMNS = tuple(
    f"{column}_mean" for column in RUN_FAILURE_CLASS_COLUMNS
)
RETIRED_ENGINE_TOKEN = "f" "ield"


def normalize_failure_summary_count(
    frame: pl.DataFrame, *, count_column: str, class_columns: tuple[str, ...]
) -> pl.DataFrame:
    if frame.is_empty() or count_column not in frame.columns:
        return frame
    terms = [
        pl.col(column).fill_null(0) if column in frame.columns else pl.lit(0)
        for column in class_columns
    ]
    total = terms[0]
    for term in terms[1:]:
        total = total + term
    return frame.with_columns(total.cast(pl.UInt32).alias(count_column))


def active_report_columns(frame: pl.DataFrame) -> pl.DataFrame:
    if frame.is_empty():
        return frame
    keep_columns = [
        column for column in frame.columns if RETIRED_ENGINE_TOKEN not in column.lower()
    ]
    return frame.select(keep_columns)


def without_retired_rows(frame: pl.DataFrame) -> pl.DataFrame:
    if frame.is_empty():
        return frame
    keep = pl.lit(True)
    for column in (
        "engine_family",
        "comparison_engine_set",
        "dominant_engine",
        "config_id",
        "family_id",
    ):
        if column not in frame.columns:
            continue
        values = pl.col(column).cast(pl.String, strict=False).str.to_lowercase()
        keep = keep & (
            pl.col(column).is_null()
            | ~values.str.contains(RETIRED_ENGINE_TOKEN).fill_null(False)
        )
    return frame.filter(keep)


def report_scope_rows(frame: pl.DataFrame) -> pl.DataFrame:
    if frame.is_empty():
        return frame
    route_visible = ROUTE_VISIBLE_ENGINE_SET_ORDER
    engine_family_values = [
        *route_visible,
        "comparison",
        "head-to-head",
    ]
    config_prefix_values = [
        *route_visible,
        "transition-",
    ]
    frame = without_retired_rows(frame)
    keep = pl.lit(True)
    if "engine_family" in frame.columns:
        keep = keep & (
            pl.col("engine_family").is_null()
            | pl.col("engine_family").is_in(engine_family_values)
        )
    if "comparison_engine_set" in frame.columns:
        keep = keep & (
            pl.col("comparison_engine_set").is_null()
            | pl.col("comparison_engine_set").is_in(route_visible)
        )
    if "dominant_engine" in frame.columns:
        keep = keep & (
            pl.col("dominant_engine").is_null()
            | pl.col("dominant_engine").is_in([*route_visible, "tie", "none"])
        )
    if "config_id" in frame.columns:
        prefix_keep = pl.lit(False)
        for prefix in config_prefix_values:
            prefix_keep = prefix_keep | pl.col("config_id").str.starts_with(prefix)
        keep = keep & (pl.col("config_id").is_null() | prefix_keep)
    return active_report_columns(frame.filter(keep))


def route_analysis_rows(frame: pl.DataFrame) -> pl.DataFrame:
    if frame.is_empty():
        return frame
    route_visible = ROUTE_VISIBLE_ENGINE_SET_ORDER
    frame = without_retired_rows(frame)
    keep = pl.lit(True)
    if "engine_family" in frame.columns:
        keep = keep & (
            pl.col("engine_family").is_null()
            | pl.col("engine_family").is_in([*route_visible, "comparison", "head-to-head"])
        )
    if "comparison_engine_set" in frame.columns:
        keep = keep & (
            pl.col("comparison_engine_set").is_null()
            | pl.col("comparison_engine_set").is_in(route_visible)
        )
    return active_report_columns(frame.filter(keep))


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) != 1:
        print("usage: python -m analysis.report <artifact-dir>", file=sys.stderr)
        return 1

    artifact_arg = Path(argv[0]).resolve()
    artifact_dir = artifact_arg.parent if artifact_arg.name == "report" else artifact_arg
    report_dir = artifact_arg if artifact_arg.name == "report" else artifact_dir / "report"
    pdf_path = artifact_dir / REPORT_PDF_NAME

    def load_required_frame(raw_name: str, csv_name: str) -> tuple[Path | None, object]:
        raw_path = artifact_dir / raw_name
        if raw_path.exists():
            if raw_name.endswith(".jsonl"):
                return raw_path, load_ndjson(raw_path)
            return raw_path, load_json_array(raw_path)
        csv_path = report_dir / csv_name
        if csv_path.exists():
            return csv_path, load_csv(csv_path)
        return None, load_csv(csv_path) if csv_path.exists() else load_optional_csv(csv_path)

    def load_optional_frame(raw_name: str, csv_name: str) -> object:
        raw_path = artifact_dir / raw_name
        if raw_path.exists():
            if raw_name.endswith(".jsonl"):
                return load_optional_ndjson(raw_path)
            return load_optional_json_array(raw_path)
        return load_optional_csv(report_dir / csv_name)

    runs_source, runs = load_required_frame("runs.jsonl", "runs.csv")
    aggregates_source, aggregates = load_required_frame("aggregates.json", "aggregates.csv")
    breakdowns_source, breakdowns = load_required_frame("breakdowns.json", "breakdowns.csv")
    diffusion_runs = load_optional_frame("diffusion_runs.jsonl", "diffusion_runs.csv")
    diffusion_aggregates = load_optional_frame(
        "diffusion_aggregates.json", "diffusion_aggregates.csv"
    )
    diffusion_boundaries = load_optional_frame(
        "diffusion_boundaries.json", "diffusion_boundaries.csv"
    )
    runs = normalize_failure_summary_count(
        runs,
        count_column="failure_summary_count",
        class_columns=RUN_FAILURE_CLASS_COLUMNS,
    )
    aggregates = normalize_failure_summary_count(
        aggregates,
        count_column="failure_summary_count_mean",
        class_columns=AGGREGATE_FAILURE_CLASS_COLUMNS,
    )
    if runs.is_empty() or aggregates.is_empty():
        print(
            "no tuning data found in "
            f"{artifact_dir} (expected raw artifacts or report CSVs such as "
            f"{runs_source or report_dir / 'runs.csv'} and "
            f"{aggregates_source or report_dir / 'aggregates.csv'})",
            file=sys.stderr,
        )
        return 1
    routing_runs = route_analysis_rows(runs)
    routing_aggregates = route_analysis_rows(aggregates)
    routing_breakdowns = route_analysis_rows(breakdowns)
    diffusion_runs = report_scope_rows(diffusion_runs)
    diffusion_aggregates = report_scope_rows(diffusion_aggregates)
    diffusion_boundaries = report_scope_rows(diffusion_boundaries)

    with tempfile.TemporaryDirectory(
        dir=artifact_dir, prefix=".report-staging-"
    ) as staging_root:
        staging_root_path = Path(staging_root)
        output_report_dir = staging_root_path / "report"
        output_pdf_path = staging_root_path / "report.pdf"
        ensure_dir(output_report_dir)
        cleanup_report_dir(output_report_dir)

        recommendations = recommendation_table(routing_aggregates, routing_breakdowns)
        profile_recommendations = profile_recommendation_table(
            routing_aggregates, routing_breakdowns
        )
        benchmark_profile_audit = benchmark_profile_audit_table(
            routing_aggregates, profile_recommendations
        )
        transition_metrics = transition_metrics_table(routing_runs, recommendations)
        recommended_engine_robustness = (
            recommendations.sort(["engine_family", "mean_score", "config_id"], descending=[False, True, False])
            .group_by("engine_family")
            .agg(
                pl.first("config_id").alias("config_id"),
                pl.first("max_sustained_stress_score").alias("max_sustained_stress_score"),
            )
            .join(transition_metrics, on=["engine_family", "config_id"], how="left")
            .with_columns(
                pl.col("route_present_mean").alias("route_present_mean_permille"),
                pl.col("route_present_stddev").alias("route_present_stddev_permille"),
            )
            .sort("engine_family")
        )
        boundary_summary = boundary_summary_table(recommendations, routing_breakdowns)
        baseline_comparison, baseline_dir = baseline_comparison_table(
            artifact_dir, recommendations
        )
        comparison_summary = comparison_summary_table(routing_aggregates)
        comparison_engine_round_breakdown = comparison_engine_round_breakdown_table(
            routing_aggregates
        )
        comparison_config_sensitivity = comparison_config_sensitivity_table(routing_aggregates)
        head_to_head_summary = head_to_head_summary_table(routing_aggregates)
        diffusion_engine_summary = diffusion_engine_summary_table(diffusion_aggregates)
        diffusion_baseline_audit = diffusion_baseline_audit_table(diffusion_aggregates)
        diffusion_weight_sensitivity = diffusion_family_weight_sensitivity_table(
            diffusion_aggregates
        )
        diffusion_regime_engine_summary = diffusion_regime_engine_summary_table(
            diffusion_aggregates
        )
        diffusion_engine_comparison = diffusion_engine_comparison_table(diffusion_aggregates)
        diffusion_boundary_summary = diffusion_boundary_table(diffusion_boundaries)
        large_population_route_summary = large_population_route_summary_table(
            routing_aggregates
        )
        routing_fitness_crossover_summary = routing_fitness_crossover_summary_table(
            routing_aggregates
        )
        routing_fitness_multiflow_summary = routing_fitness_multiflow_summary_table(
            routing_aggregates
        )
        routing_fitness_stale_repair_summary = routing_fitness_stale_repair_summary_table(
            routing_aggregates
        )
        large_population_diffusion_points = large_population_diffusion_state_points_table(
            diffusion_aggregates
        )
        large_population_diffusion_transitions = large_population_diffusion_transition_table(
            diffusion_aggregates
        )
        write_csv(routing_runs, output_report_dir / "runs.csv")
        write_csv(routing_aggregates, output_report_dir / "aggregates.csv")
        write_csv(routing_breakdowns, output_report_dir / "breakdowns.csv")
        write_csv(recommendations, output_report_dir / "recommendations.csv")
        write_csv(
            profile_recommendations, output_report_dir / "profile_recommendations.csv"
        )
        write_csv(
            benchmark_profile_audit,
            output_report_dir / "benchmark_profile_audit.csv",
        )
        write_csv(transition_metrics, output_report_dir / "transition_metrics.csv")
        write_csv(boundary_summary, output_report_dir / "boundary_summary.csv")
        write_csv(baseline_comparison, output_report_dir / "baseline_comparison.csv")
        write_csv(comparison_summary, output_report_dir / "comparison_summary.csv")
        write_csv(
            comparison_engine_round_breakdown,
            output_report_dir / "comparison_engine_round_breakdown.csv",
        )
        write_csv(
            comparison_config_sensitivity,
            output_report_dir / "comparison_config_sensitivity.csv",
        )
        write_csv(head_to_head_summary, output_report_dir / "head_to_head_summary.csv")
        write_csv(diffusion_runs, output_report_dir / "diffusion_runs.csv")
        write_csv(diffusion_aggregates, output_report_dir / "diffusion_aggregates.csv")
        write_csv(diffusion_boundaries, output_report_dir / "diffusion_boundaries.csv")
        write_csv(
            diffusion_engine_summary, output_report_dir / "diffusion_engine_summary.csv"
        )
        write_csv(
            diffusion_baseline_audit,
            output_report_dir / "diffusion_baseline_audit.csv",
        )
        write_csv(
            diffusion_weight_sensitivity,
            output_report_dir / "diffusion_weight_sensitivity.csv",
        )
        write_csv(
            diffusion_regime_engine_summary,
            output_report_dir / "diffusion_regime_engine_summary.csv",
        )
        write_csv(
            diffusion_engine_comparison,
            output_report_dir / "diffusion_engine_comparison.csv",
        )
        write_csv(
            diffusion_boundary_summary,
            output_report_dir / "diffusion_boundary_summary.csv",
        )
        write_csv(
            large_population_route_summary,
            output_report_dir / "large_population_route_summary.csv",
        )
        write_csv(
            routing_fitness_crossover_summary,
            output_report_dir / "routing_fitness_crossover_summary.csv",
        )
        write_csv(
            routing_fitness_multiflow_summary,
            output_report_dir / "routing_fitness_multiflow_summary.csv",
        )
        write_csv(
            routing_fitness_stale_repair_summary,
            output_report_dir / "routing_fitness_stale_repair_summary.csv",
        )
        write_csv(
            large_population_diffusion_points,
            output_report_dir / "large_population_diffusion_points.csv",
        )
        write_csv(
            large_population_diffusion_transitions,
            output_report_dir / "large_population_diffusion_transitions.csv",
        )
        write_recommendations(output_report_dir / "recommendations.md", recommendations)

        save_plot_artifact(
            output_report_dir,
            "batman_bellman_transition_stability",
            render_batman_bellman_transition_stability,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "batman_bellman_transition_loss",
            render_batman_bellman_transition_loss,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "batman_classic_transition_stability",
            render_batman_classic_transition_stability,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "batman_classic_transition_loss",
            render_batman_classic_transition_loss,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "babel_decay_stability",
            render_babel_decay_stability,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "babel_decay_loss",
            render_babel_decay_loss,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "olsrv2_decay_stability",
            render_olsrv2_decay_stability,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "olsrv2_decay_loss",
            render_olsrv2_decay_loss,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "scatter_profile_route_presence",
            render_scatter_profile_route_presence,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "scatter_profile_runtime",
            render_scatter_profile_runtime,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "pathway_budget_route_presence",
            render_pathway_budget_route_presence,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "pathway_budget_activation",
            render_pathway_budget_activation,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "comparison_dominant_engine",
            render_comparison_summary,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "head_to_head_route_presence",
            render_head_to_head_route_presence,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "head_to_head_timing_profile",
            render_head_to_head_timing_profile,
            routing_aggregates,
        )
        save_plot_artifact(
            output_report_dir,
            "recommended_engine_robustness",
            render_recommended_engine_robustness,
            recommended_engine_robustness,
        )
        save_plot_artifact(
            output_report_dir,
            "mixed_vs_standalone_divergence",
            render_mixed_vs_standalone_divergence,
            routing_aggregates,
        )
        if not diffusion_engine_comparison.is_empty():
            save_plot_artifact(
                output_report_dir,
                "diffusion_delivery_coverage",
                render_diffusion_delivery_coverage,
                diffusion_engine_comparison,
            )
            save_plot_artifact(
                output_report_dir,
                "diffusion_resource_boundedness",
                render_diffusion_resource_boundedness,
                diffusion_engine_comparison,
            )
        save_plot_artifact(
            output_report_dir,
            "large_population_route_scaling",
            render_large_population_route_scaling,
            large_population_route_summary,
        )
        save_plot_artifact(
            output_report_dir,
            "large_population_route_fragility",
            render_large_population_route_fragility,
            large_population_route_summary,
        )
        save_plot_artifact(
            output_report_dir,
            "routing_fitness_crossover",
            render_routing_fitness_crossover,
            routing_fitness_crossover_summary,
        )
        save_plot_artifact(
            output_report_dir,
            "routing_fitness_multiflow",
            render_routing_fitness_multiflow,
            routing_fitness_multiflow_summary,
        )
        save_plot_artifact(
            output_report_dir,
            "routing_fitness_stale_repair",
            render_routing_fitness_stale_repair,
            routing_fitness_stale_repair_summary,
        )
        save_plot_artifact(
            output_report_dir,
            "large_population_diffusion_transitions",
            render_large_population_diffusion_transitions,
            large_population_diffusion_points,
        )

        write_pdf_report(
            artifact_dir,
            output_report_dir,
            output_pdf_path,
            recommendations,
            profile_recommendations,
            benchmark_profile_audit,
            transition_metrics,
            boundary_summary,
            routing_aggregates,
            comparison_summary,
            comparison_engine_round_breakdown,
            comparison_config_sensitivity,
            head_to_head_summary,
            diffusion_engine_summary,
            diffusion_baseline_audit,
            diffusion_weight_sensitivity,
            diffusion_regime_engine_summary,
            diffusion_engine_comparison,
            diffusion_boundary_summary,
            large_population_route_summary,
            routing_fitness_crossover_summary,
            routing_fitness_multiflow_summary,
            routing_fitness_stale_repair_summary,
            large_population_diffusion_transitions,
            baseline_comparison,
            baseline_dir,
        )
        if report_dir.exists():
            shutil.rmtree(report_dir)
        shutil.move(str(output_report_dir), str(report_dir))
        for legacy_name in LEGACY_REPORT_PDF_NAMES:
            legacy_path = artifact_dir / legacy_name
            if legacy_path.exists():
                legacy_path.unlink()
        if pdf_path.exists():
            pdf_path.unlink()
        shutil.move(str(output_pdf_path), str(pdf_path))
    validate_report_artifacts_or_raise(artifact_dir)
    print(f"Analysis report artifacts: {report_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
