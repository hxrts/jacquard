"""Report artifact sanity checks for generated analysis outputs."""

from __future__ import annotations

import math
import sys
from dataclasses import dataclass
from pathlib import Path

import polars as pl

from .data import load_optional_csv


REPORT_PDF_NAME = "router-tuning-report.pdf"
RETIRED_ENGINE_TOKEN = "f" "ield"


@dataclass(frozen=True)
class ReportSanityIssue:
    """A deterministic report artifact problem suitable for CI output."""

    path: str
    message: str

    def render(self) -> str:
        return f"{self.path}: {self.message}"


REQUIRED_COLUMNS: dict[str, tuple[str, ...]] = {
    "runs.csv": (
        "run_id",
        "family_id",
        "engine_family",
        "config_id",
        "seed",
        "route_present_total_window_permille",
    ),
    "aggregates.csv": (
        "family_id",
        "engine_family",
        "config_id",
        "run_count",
        "route_present_total_window_permille_mean",
    ),
    "comparison_summary.csv": (
        "family_id",
        "config_id",
        "dominant_engine",
        "route_present_total_window_permille_mean",
    ),
    "head_to_head_summary.csv": (
        "family_id",
        "config_id",
        "comparison_engine_set",
        "route_present_total_window_permille_mean",
    ),
    "large_population_route_summary.csv": (
        "topology_class",
        "comparison_engine_set",
        "high_route_present",
    ),
    "routing_fitness_crossover_summary.csv": (
        "family_id",
        "question",
        "comparison_engine_set",
        "route_present_total_window_permille_mean",
        "route_churn_count_mean",
    ),
    "routing_fitness_multiflow_summary.csv": (
        "family_id",
        "comparison_engine_set",
        "objective_route_presence_min_permille_mean",
        "objective_starvation_count_mean",
        "broker_metric_status",
        "broker_concentration_permille_mean",
    ),
    "routing_fitness_stale_repair_summary.csv": (
        "family_id",
        "comparison_engine_set",
        "route_present_total_window_permille_mean",
        "stale_persistence_round_mean",
        "recovery_success_permille_mean",
        "unrecovered_after_loss_count_mean",
        "repair_metric_status",
    ),
    "observer_ambiguity_summary.csv": (
        "observer_projection_identity",
        "attacker_target",
        "fragment_dispersion_permille",
        "attacker_top1_accuracy_permille",
        "posterior_uncertainty_permille",
        "hidden_projection_proxy_permille",
        "ambiguity_cost_frontier_area",
        "cost_bytes",
        "latency_rounds",
        "quality_permille",
    ),
}


HEADLINE_SERIES: dict[str, tuple[str, ...]] = {
    "comparison_summary.csv": ("route_present_total_window_permille_mean",),
    "head_to_head_summary.csv": ("route_present_total_window_permille_mean",),
    "large_population_route_summary.csv": (
        "small_route_present",
        "moderate_route_present",
        "high_route_present",
    ),
    "transition_metrics.csv": ("route_present_mean",),
    "routing_fitness_crossover_summary.csv": (
        "route_present_total_window_permille_mean",
    ),
    "routing_fitness_multiflow_summary.csv": (
        "objective_route_presence_min_permille_mean",
    ),
    "routing_fitness_stale_repair_summary.csv": (
        "route_present_total_window_permille_mean",
    ),
    "diffusion_engine_comparison.csv": ("delivery_probability_permille_mean",),
    "observer_ambiguity_summary.csv": ("hidden_projection_proxy_permille",),
}


FIGURE_DATASETS: dict[str, str] = {
    "comparison_dominant_engine": "comparison_summary.csv",
    "head_to_head_route_presence": "head_to_head_summary.csv",
    "head_to_head_timing_profile": "head_to_head_summary.csv",
    "recommended_engine_robustness": "transition_metrics.csv",
    "large_population_route_scaling": "large_population_route_summary.csv",
    "large_population_route_fragility": "large_population_route_summary.csv",
    "routing_fitness_crossover": "routing_fitness_crossover_summary.csv",
    "routing_fitness_multiflow": "routing_fitness_multiflow_summary.csv",
    "routing_fitness_stale_repair": "routing_fitness_stale_repair_summary.csv",
    "diffusion_delivery_coverage": "diffusion_engine_comparison.csv",
    "diffusion_resource_boundedness": "diffusion_engine_comparison.csv",
    "observer_attacker_advantage": "observer_ambiguity_summary.csv",
    "observer_uncertainty": "observer_ambiguity_summary.csv",
    "observer_ambiguity_cost_frontier": "observer_ambiguity_summary.csv",
    "observer_dispersion_cost_latency": "observer_ambiguity_summary.csv",
}

MERCATOR_REQUIRED_FIGURES = {
    "head_to_head_route_presence",
    "head_to_head_timing_profile",
    "recommended_engine_robustness",
    "large_population_route_scaling",
    "large_population_route_fragility",
    "routing_fitness_crossover",
    "routing_fitness_multiflow",
    "routing_fitness_stale_repair",
    "diffusion_delivery_coverage",
    "diffusion_resource_boundedness",
}


PERMILLE_COLUMN_TOKENS = (
    "activation_success",
    "broker_concentration",
    "broker_participation",
    "cluster_coverage",
    "corridor_persistence",
    "coverage",
    "delivery_probability",
    "objective_route_presence",
    "observer_leakage",
    "attacker_top1_accuracy",
    "hidden_projection_proxy",
    "posterior_uncertainty",
    "forwarding_contact_proxy",
    "quality",
    "collective_uncertainty",
    "receiver_agreement",
    "demand_satisfaction",
    "decision_accuracy",
    "recovery_success",
    "recovery_probability",
    "path_free_success",
    "merged_statistic_quality",
    "r_est",
    "fragment_dispersion",
    "forwarding_randomness",
    "observer_advantage",
    "receiver_arrival_bound",
    "lower_tail_failure",
    "false_commitment_bound",
    "commitment_accuracy",
    "false_commitment_rate",
    "receiver_arrival_probability",
    "equal_quality_cost_reduction",
    "equal_cost_quality_improvement",
    "route_present",
    "storage_utilization",
)


NONNEGATIVE_COLUMN_TOKENS = (
    "_count",
    "_hop",
    "_latency",
    "_round",
    "_rounds",
    "churn",
    "cost_bytes",
    "byte_count",
    "fixed_payload_budget_bytes",
    "bytes_at_commitment",
    "artifact_row_count",
    "executed_node_count",
    "requested_node_count",
    "bridge_batch_id",
    "ingress_round",
    "demand_contribution_count",
    "energy",
    "latency_rounds",
    "run_count",
    "stress_score",
    "transmissions",
)


def resolve_artifact_paths(path: Path) -> tuple[Path, Path]:
    artifact_dir = path.parent if path.name == "report" else path
    report_dir = path if path.name == "report" else artifact_dir / "report"
    return artifact_dir, report_dir


def _numeric_values(df: pl.DataFrame, column: str) -> list[float]:
    if column not in df.columns:
        return []
    casted = df[column].cast(pl.Float64, strict=False).drop_nulls()
    values: list[float] = []
    for value in casted.to_list():
        if value is None:
            continue
        number = float(value)
        if math.isfinite(number):
            values.append(number)
    return values


def _column_has_any_value(df: pl.DataFrame, columns: tuple[str, ...]) -> bool:
    for column in columns:
        if any(abs(value) > 0.000001 for value in _numeric_values(df, column)):
            return True
    return False


def _has_retired_report_value(df: pl.DataFrame) -> bool:
    if any(RETIRED_ENGINE_TOKEN in column.lower() for column in df.columns):
        return True
    for column in (
        "engine_family",
        "comparison_engine_set",
        "dominant_engine",
        "config_id",
        "family_id",
    ):
        if column not in df.columns:
            continue
        values = df[column].cast(pl.String, strict=False).str.to_lowercase()
        if df.filter(values.str.contains(RETIRED_ENGINE_TOKEN).fill_null(False)).height > 0:
            return True
    return False


def _check_csv_schema(report_dir: Path, file_name: str, df: pl.DataFrame) -> list[ReportSanityIssue]:
    issues: list[ReportSanityIssue] = []
    required = REQUIRED_COLUMNS.get(file_name, ())
    missing = [column for column in required if column not in df.columns]
    if missing:
        issues.append(
            ReportSanityIssue(file_name, f"missing required column(s): {', '.join(missing)}")
        )
        return issues
    if df.is_empty():
        return issues
    if _has_retired_report_value(df):
        issues.append(ReportSanityIssue(file_name, "retired routing engine data is present"))
    for column in required:
        if df[column].null_count() == df.height:
            issues.append(ReportSanityIssue(file_name, f"required column `{column}` is all null"))
    headline = HEADLINE_SERIES.get(file_name)
    if headline and not _column_has_any_value(df, headline):
        issues.append(
            ReportSanityIssue(
                file_name,
                f"headline series is all zero/null: {', '.join(headline)}",
            )
        )
    return issues


def _should_check_permille(column: str) -> bool:
    return "permille" in column and any(token in column for token in PERMILLE_COLUMN_TOKENS)


def _should_check_nonnegative(column: str) -> bool:
    if "delta" in column or "spread" in column:
        return False
    return any(token in column for token in NONNEGATIVE_COLUMN_TOKENS)


def _check_metric_ranges(file_name: str, df: pl.DataFrame) -> list[ReportSanityIssue]:
    issues: list[ReportSanityIssue] = []
    for column in df.columns:
        values = _numeric_values(df, column)
        if not values:
            continue
        if _should_check_permille(column):
            outside = [value for value in values if value < 0.0 or value > 1000.0]
            if outside:
                issues.append(
                    ReportSanityIssue(
                        file_name,
                        f"`{column}` has values outside 0..1000 permille",
                    )
                )
        if _should_check_nonnegative(column) and min(values) < 0.0:
            issues.append(
                ReportSanityIssue(file_name, f"`{column}` contains negative values")
            )
    return issues


def _check_recovery_consistency(file_name: str, df: pl.DataFrame) -> list[ReportSanityIssue]:
    if df.is_empty() or "stale_persistence_round_mean" not in df.columns:
        return []
    if "first_disruption_round_mean" not in df.columns:
        return []
    stale = df["stale_persistence_round_mean"].is_not_null()
    disrupted = df["first_disruption_round_mean"].is_not_null()
    bad_count = df.filter(stale & ~disrupted).height
    if bad_count == 0:
        return []
    return [
        ReportSanityIssue(
            file_name,
            f"{bad_count} row(s) report stale persistence without a disruption round",
        )
    ]


def _check_csvs(report_dir: Path) -> list[ReportSanityIssue]:
    issues: list[ReportSanityIssue] = []
    for csv_path in sorted(report_dir.glob("*.csv")):
        df = load_optional_csv(csv_path)
        file_name = csv_path.name
        issues.extend(_check_csv_schema(report_dir, file_name, df))
        if not df.is_empty():
            issues.extend(_check_metric_ranges(file_name, df))
            issues.extend(_check_recovery_consistency(file_name, df))
    return issues


def _check_svg_not_blank(svg_path: Path) -> list[ReportSanityIssue]:
    text = svg_path.read_text(errors="ignore")
    if svg_path.stat().st_size < 1000:
        return [ReportSanityIssue(svg_path.name, "SVG is too small to contain a plot")]
    if "role=\"graphics-symbol\"" not in text:
        return [ReportSanityIssue(svg_path.name, "SVG contains no plotted symbols")]
    return []


def _dataset_has_mercator_source(df: pl.DataFrame) -> bool:
    for column in ("comparison_engine_set", "engine_family", "config_id"):
        if column not in df.columns:
            continue
        values = (
            df[column]
            .cast(pl.String, strict=False)
            .str.to_lowercase()
            .str.contains("mercator")
        )
        if df.filter(values.fill_null(False)).height > 0:
            return True
    return False


def _check_figure_assets(report_dir: Path) -> list[ReportSanityIssue]:
    issues: list[ReportSanityIssue] = []
    for svg_path in sorted(report_dir.glob("*.svg")):
        stem = svg_path.stem
        svg_text = svg_path.read_text(errors="ignore")
        issues.extend(_check_svg_not_blank(svg_path))
        for suffix in (".png", ".pdf"):
            sibling = report_dir / f"{stem}{suffix}"
            if not sibling.exists():
                issues.append(ReportSanityIssue(sibling.name, "missing figure sibling"))
            elif sibling.stat().st_size < 1000:
                issues.append(ReportSanityIssue(sibling.name, "figure sibling is too small"))
        dataset = FIGURE_DATASETS.get(stem)
        if dataset and not (report_dir / dataset).exists():
            issues.append(ReportSanityIssue(svg_path.name, f"missing source CSV `{dataset}`"))
        if dataset and stem in MERCATOR_REQUIRED_FIGURES and (report_dir / dataset).exists():
            source = load_optional_csv(report_dir / dataset)
            if (
                not source.is_empty()
                and _dataset_has_mercator_source(source)
                and "Mercator" not in svg_text
            ):
                issues.append(
                    ReportSanityIssue(
                        svg_path.name,
                        "source data includes Mercator but figure does not render it",
                    )
                )
    stale_svg = report_dir / "routing_fitness_stale_repair.svg"
    if stale_svg.exists():
        text = stale_svg.read_text(errors="ignore")
        if "route=" not in text:
            issues.append(
                ReportSanityIssue(stale_svg.name, "stale repair labels must use route presence")
            )
        if "recov=" in text:
            issues.append(
                ReportSanityIssue(stale_svg.name, "stale repair labels still use recovery success")
            )
    crossover_svg = report_dir / "routing_fitness_crossover.svg"
    if crossover_svg.exists():
        text = crossover_svg.read_text(errors="ignore")
        if "Recovery success" in text:
            issues.append(
                ReportSanityIssue(
                    crossover_svg.name,
                    "crossover figure still plots non-headline recovery success",
                )
            )
    divergence_svg = report_dir / "mixed_vs_standalone_divergence.svg"
    if divergence_svg.exists():
        text = divergence_svg.read_text(errors="ignore")
        if "0.0 pts" in text:
            issues.append(
                ReportSanityIssue(
                    divergence_svg.name,
                    "mixed-vs-standalone divergence labels collapse ties to 0.0 pts",
                )
            )
    return issues


def validate_report_artifacts(path: Path) -> list[ReportSanityIssue]:
    artifact_dir, report_dir = resolve_artifact_paths(path)
    issues: list[ReportSanityIssue] = []
    if not report_dir.exists():
        return [ReportSanityIssue(str(report_dir), "report directory does not exist")]
    pdf_path = artifact_dir / REPORT_PDF_NAME
    if artifact_dir != report_dir and not pdf_path.exists():
        issues.append(ReportSanityIssue(str(pdf_path), "report PDF does not exist"))
    elif artifact_dir != report_dir and pdf_path.stat().st_size < 1000:
        issues.append(ReportSanityIssue(str(pdf_path), "report PDF is too small"))
    issues.extend(_check_csvs(report_dir))
    issues.extend(_check_figure_assets(report_dir))
    return issues


def validate_report_artifacts_or_raise(path: Path) -> None:
    issues = validate_report_artifacts(path)
    if issues:
        rendered = "\n".join(issue.render() for issue in issues)
        raise RuntimeError(f"report artifact sanity failed:\n{rendered}")


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) != 1:
        print("usage: python -m analysis.sanity <artifact-dir-or-report-dir>", file=sys.stderr)
        return 1
    issues = validate_report_artifacts(Path(argv[0]).resolve())
    if issues:
        for issue in issues:
            print(issue.render(), file=sys.stderr)
        return 1
    print("report artifact sanity: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
