"""Sanity checks for active-belief paper report artifacts."""

from __future__ import annotations

import csv
import sys
from dataclasses import dataclass
from pathlib import Path

REPORT_PDF_NAME = "active-belief-report.pdf"
MIN_PAPER_PDF_BYTES = 10_000

REQUIRED_COLUMNS: dict[str, tuple[str, ...]] = {
    "active_belief_figure_claim_map.csv": (
        "figure_index", "figure_name", "claim_category", "paper_claim",
        "source_artifact", "current_row_count", "required_row_count",
        "required_baselines", "uncertainty_required", "status",
    ),
    "active_belief_raw_rounds.csv": (
        "experiment_id", "scenario_id", "trace_family", "seed", "policy_or_mode",
        "task_kind", "fixed_budget_label", "fixed_payload_budget_bytes",
        "statistic_kind", "merge_operation", "no_static_path_in_core_window",
        "time_respecting_evidence_journey_exists", "round_index",
        "hypothesis_id", "scaled_score", "receiver_rank",
        "top_hypothesis_margin", "uncertainty_permille", "byte_count",
        "demand_byte_count", "total_byte_count", "duplicate_count", "innovative_arrival_count",
        "demand_satisfaction_permille", "r_est_permille",
        "merged_statistic_quality_permille", "canonical_trace_hash",
        "config_hash", "artifact_hash",
    ),
    "active_belief_receiver_runs.csv": (
        "experiment_id", "scenario_id", "trace_family", "seed", "receiver_id",
        "mode", "task_kind", "fixed_payload_budget_bytes",
        "quality_per_byte_permille", "quality_per_total_budget_permille",
        "collective_uncertainty_permille",
        "receiver_agreement_permille", "belief_divergence_permille",
        "commitment_time_round", "full_recovery_time_round",
        "commitment_lead_time_rounds", "bytes_at_commitment",
        "demand_bytes_at_commitment", "total_bytes_at_commitment",
        "commitment_correct", "deterministic_replay", "canonical_trace_hash",
        "config_hash", "artifact_hash",
    ),
    "active_belief_path_validation.csv": (
        "experiment_id", "scenario_id", "trace_family", "seed",
        "policy_or_mode", "fixed_budget_label", "fixed_payload_budget_bytes",
        "no_static_path_in_core_window", "static_path_absent_round_count",
        "core_window_round_count", "time_respecting_evidence_journey_exists",
        "time_respecting_journey_count", "recovery_probability_permille",
        "path_free_success_permille", "cost_to_recover_bytes", "byte_count",
        "duplicate_count", "canonical_trace_hash",
    ),
    "active_belief_demand_ablation.csv": (
        "experiment_id", "scenario_id", "trace_family", "seed", "task_kind",
        "demand_policy", "fixed_payload_budget_bytes",
        "demand_byte_count", "total_byte_count",
        "quality_per_byte_permille", "quality_per_total_budget_permille",
        "collective_uncertainty_permille",
        "receiver_agreement_permille", "demand_satisfaction_permille",
        "demand_response_lag_rounds",
        "uncertainty_reduction_after_demand_permille", "bytes_at_commitment",
        "duplicate_count", "innovative_arrival_count", "deterministic_replay",
    ),
    "active_belief_demand_byte_sweep.csv": (
        "experiment_id", "scenario_id", "trace_family", "seed", "task_kind",
        "fixed_payload_budget_bytes", "demand_byte_budget", "total_budget_bytes",
        "quality_per_byte_permille", "effective_rank_proxy",
        "collective_uncertainty_permille", "demand_satisfaction_permille",
        "innovative_arrival_count",
        "duplicate_count", "deterministic_replay",
    ),
    "active_belief_high_gap_regimes.csv": (
        "experiment_id", "regime_family", "demand_heterogeneity_percent",
        "seed", "mode", "fixed_payload_budget_bytes", "demand_byte_budget",
        "quality_per_byte_permille", "collective_uncertainty_permille",
        "active_minus_passive_gap_permille", "deterministic_replay",
    ),
    "active_belief_adversarial_demand.csv": (
        "experiment_id", "seed", "malicious_demand_fraction_percent",
        "fixed_payload_budget_bytes", "demand_byte_budget",
        "honest_receiver_quality_permille", "quality_degradation_permille",
        "false_commitment_rate_permille", "evidence_validity_changed",
        "duplicate_rank_inflation", "deterministic_replay",
    ),
    "active_belief_byzantine_injection.csv": (
        "experiment_id", "seed", "malicious_fraction_percent",
        "fixed_payload_budget_bytes", "forged_contribution_attempts",
        "forged_contribution_rejections",
        "accepted_malicious_signed_contributions",
        "duplicate_pressure_inflation_permille", "decision_accuracy_permille",
        "false_commitment_rate_permille", "quality_per_byte_permille",
        "deterministic_replay",
    ),
    "active_belief_scale_validation.csv": (
        "seed", "scenario_regime", "node_count", "runtime_ms",
        "runtime_budget_ms", "memory_kib", "replay_hash_agreement",
        "quality_per_byte_permille", "failure_rate_permille",
        "deterministic_replay",
    ),
    "active_belief_receiver_count_sweep.csv": (
        "experiment_id", "scenario_id", "trace_family", "seed",
        "receiver_count", "fixed_payload_budget_bytes",
        "quality_per_byte_permille", "receiver_agreement_permille",
        "belief_divergence_permille", "collective_uncertainty_permille",
        "commitment_lead_time_rounds_median", "deterministic_replay",
    ),
    "active_belief_independence_bottleneck.csv": (
        "experiment_id", "scenario_id", "trace_family", "seed",
        "pair_kind", "fixed_payload_budget_bytes", "raw_transmissions",
        "raw_fragment_count", "innovative_contribution_count",
        "effective_rank_proxy", "raw_reproduction_permille",
        "useful_reproduction_permille", "quality_per_byte_permille",
        "recovery_probability_permille", "demand_byte_budget",
        "deterministic_replay",
    ),
    "active_belief_convex_erm.csv": (
        "experiment_id", "scenario_id", "trace_family", "seed", "task_kind",
        "objective_id", "loss_family_id", "regularizer_id",
        "contribution_identity_count", "accepted_objective_terms",
        "effective_independent_loss_terms", "objective_value",
        "optimizer_lower_bound", "solver_gap", "decision_margin",
        "uncertainty_bound", "duplicate_discount", "guard_passed",
        "certificate_hash", "deterministic_replay",
    ),
    "coded_inference_experiment_a_landscape.csv": (
        "experiment_id", "scenario_id", "seed", "policy_or_mode",
        "fixed_budget_label", "statistic_kind", "merge_operation",
        "no_static_path_in_core_window", "time_respecting_evidence_journey_exists",
        "round_index", "hypothesis_id", "scaled_score", "receiver_rank",
        "top_hypothesis_margin", "uncertainty_permille", "byte_count",
        "duplicate_count", "merged_statistic_quality_permille",
    ),
    "coded_inference_experiment_a2_evidence_modes.csv": (
        "experiment_id", "scenario_id", "seed", "policy_or_mode",
        "statistic_kind", "merge_operation", "available_evidence_count",
        "useful_contribution_count", "receiver_rank", "top_hypothesis_margin",
        "uncertainty_permille", "byte_count", "duplicate_count",
        "storage_pressure_bytes", "merged_statistic_quality_permille",
    ),
    "coded_inference_experiment_b_path_free_recovery.csv": (
        "experiment_id", "scenario_id", "seed", "policy_or_mode",
        "fixed_budget_label", "no_static_path_in_core_window",
        "recovery_probability_permille", "path_free_success_permille",
        "cost_to_recover_bytes", "byte_count", "duplicate_count",
    ),
    "coded_inference_experiment_c_phase_diagram.csv": (
        "experiment_id", "scenario_id", "seed", "policy_or_mode",
        "reproduction_target_low_permille", "reproduction_target_high_permille",
        "r_est_permille", "raw_reproduction_permille", "useful_reproduction_permille",
        "forwarding_budget", "coding_k", "coding_n",
        "recovery_probability_permille", "quality_permille",
        "merged_statistic_quality_permille", "byte_count", "duplicate_rate_permille",
    ),
    "coded_inference_experiment_d_coding_vs_replication.csv": (
        "experiment_id", "scenario_id", "seed", "policy_or_mode",
        "fixed_budget_label", "fixed_payload_budget_bytes", "statistic_kind",
        "recovery_probability_permille", "quality_permille",
        "merged_statistic_quality_permille", "byte_count", "duplicate_count",
        "storage_pressure_bytes", "equal_quality_cost_reduction_permille",
        "equal_cost_quality_improvement_permille",
    ),
    "coded_inference_experiment_e_observer_frontier.csv": (
        "experiment_id", "scenario_id", "seed", "policy_or_mode",
        "fragment_dispersion_permille", "forwarding_randomness_permille",
        "reproduction_target_low_permille", "reproduction_target_high_permille",
        "observer_advantage_permille", "uncertainty_permille", "byte_count",
        "latency_rounds", "quality_permille", "ambiguity_metric_is_proxy",
    ),
    "active_belief_second_tasks.csv": (
        "seed", "mode", "task_kind", "statistic_kind", "receiver_rank",
        "recovery_probability_permille", "bytes_at_commitment",
        "demand_satisfaction_permille", "decision_accuracy_permille",
        "commitment_lead_time_rounds_max", "quality_per_byte_permille",
    ),
    "active_belief_host_bridge_demand.csv": (
        "seed", "mode", "execution_surface", "bridge_batch_id", "ingress_round",
        "replay_visible", "demand_contribution_count", "demand_byte_count", "evidence_validity_changed",
        "contribution_identity_created", "merge_semantics_changed",
        "route_truth_published", "duplicate_rank_inflation",
    ),
    "active_belief_theorem_assumptions.csv": (
        "theorem_name", "theorem_profile", "scenario_regime", "trace_family",
        "finite_horizon_model_valid", "contact_dependence_assumption",
        "assumption_status", "receiver_arrival_bound_permille",
        "lower_tail_failure_permille", "false_commitment_bound_permille",
        "bound_summary",
    ),
    "active_belief_large_regime.csv": (
        "seed", "scenario_regime", "requested_node_count", "executed_node_count",
        "deterministic_replay", "runtime_budget_stable", "artifact_sanity_covered",
    ),
    "active_belief_trace_validation.csv": (
        "trace_family", "external_or_semi_realistic", "canonical_preprocessing",
        "replay_deterministic", "theorem_assumption_status",
    ),
    "active_belief_strong_baselines.csv": (
        "seed", "scenario_id", "trace_family", "baseline_policy", "fixed_payload_budget_bytes",
        "decision_accuracy_permille", "quality_per_byte_permille", "deterministic",
    ),
    "active_belief_exact_seed_summary.csv": (
        "seed", "scenario_regime", "stress_kind", "stress_severity",
        "receiver_arrival_probability_permille", "commitment_accuracy_permille",
        "false_commitment_rate_permille", "commitment_lead_time_rounds_max",
        "quality_per_byte_permille",
    ),
    "active_belief_final_validation.csv": (
        "seed", "scenario_regime", "mode", "task_kind",
        "fixed_payload_budget_bytes", "collective_uncertainty_permille",
        "receiver_agreement_permille", "commitment_lead_time_rounds_max",
        "quality_per_byte_permille", "deterministic_replay",
    ),
    "active_belief_scaling_boundary.csv": (
        "requested_node_count", "executed_node_count", "documented_boundary",
        "boundary_reason",
    ),
    "active_belief_headline_statistics.csv": (
        "comparison", "metric", "unit", "baseline", "treatment",
        "treatment_median", "baseline_median", "paired_delta_median",
        "paired_delta_p25", "paired_delta_p75", "paired_delta_ci_low",
        "paired_delta_ci_high", "row_count",
        "aggregation_unit",
    ),
    "active_belief_figure_artifacts.csv": (
        "figure_index", "figure_name", "source_artifact", "artifact_row_count",
        "claim_category", "fixed_budget_label", "sanity_passed",
    ),
}

REQUIRED_FIGURES = tuple(f"figure_{index:02d}" for index in range(1, 19))


@dataclass(frozen=True)
class ReportSanityIssue:
    path: str
    message: str

    def render(self) -> str:
        return f"{self.path}: {self.message}"


def resolve_artifact_paths(path: Path) -> tuple[Path, Path]:
    artifact_dir = path.parent if path.name == "report" else path
    report_dir = path if path.name == "report" else artifact_dir / "report"
    return artifact_dir, report_dir


def read_csv_rows(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    with path.open(newline="") as handle:
        reader = csv.DictReader(handle)
        rows = list(reader)
        return list(reader.fieldnames or []), rows


def check_csv(path: Path, expected: tuple[str, ...]) -> list[ReportSanityIssue]:
    if not path.exists():
        return [ReportSanityIssue(str(path), "required active-belief CSV is missing")]
    columns, rows = read_csv_rows(path)
    missing = [column for column in expected if column not in columns]
    issues: list[ReportSanityIssue] = []
    if missing:
        issues.append(ReportSanityIssue(path.name, f"missing columns: {', '.join(missing)}"))
    if not rows:
        issues.append(ReportSanityIssue(path.name, "CSV has no data rows"))
    for row in rows:
        if any(value.strip().lower() == "field" for value in row.values()):
            issues.append(ReportSanityIssue(path.name, "active-belief report must not contain Field router rows"))
            break
    return issues


def validate_report_artifacts(path: Path) -> list[ReportSanityIssue]:
    artifact_dir, report_dir = resolve_artifact_paths(path)
    issues: list[ReportSanityIssue] = []
    if not report_dir.exists():
        return [ReportSanityIssue(str(report_dir), "report directory does not exist")]
    pdf_path = artifact_dir / REPORT_PDF_NAME
    if artifact_dir != report_dir and not pdf_path.exists():
        issues.append(ReportSanityIssue(str(pdf_path), "active-belief PDF does not exist"))
    elif artifact_dir != report_dir and pdf_path.stat().st_size < MIN_PAPER_PDF_BYTES:
        issues.append(ReportSanityIssue(str(pdf_path), "active-belief PDF is too small to contain paper text and figures"))
    for file_name, columns in REQUIRED_COLUMNS.items():
        issues.extend(check_csv(report_dir / file_name, columns))
    figure_stems = {path.stem for path in report_dir.glob("figure_*.svg")}
    for prefix in REQUIRED_FIGURES:
        matches = sorted(stem for stem in figure_stems if stem.startswith(prefix))
        if not matches:
            issues.append(ReportSanityIssue(prefix, "required figure SVG is missing"))
            continue
        stem = matches[0]
        svg_path = report_dir / f"{stem}.svg"
        png_path = report_dir / f"{stem}.png"
        pdf_figure_path = report_dir / f"{stem}.pdf"
        if svg_path.stat().st_size < 1000:
            issues.append(ReportSanityIssue(svg_path.name, "figure SVG is too small"))
        if not png_path.exists() or png_path.stat().st_size < 1000:
            issues.append(ReportSanityIssue(png_path.name, "figure PNG is missing or too small"))
        if not pdf_figure_path.exists() or pdf_figure_path.stat().st_size < 1000:
            issues.append(ReportSanityIssue(pdf_figure_path.name, "figure PDF is missing or too small"))
    return issues


def validate_report_artifacts_or_raise(path: Path) -> None:
    issues = validate_report_artifacts(path)
    if issues:
        rendered = "\n".join(issue.render() for issue in issues)
        raise RuntimeError(f"active-belief report sanity failed:\n{rendered}")


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) != 1:
        print("usage: python -m analysis_2.sanity <artifact-dir-or-report-dir>", file=sys.stderr)
        return 1
    issues = validate_report_artifacts(Path(argv[0]).resolve())
    if issues:
        for issue in issues:
            print(issue.render(), file=sys.stderr)
        return 1
    print("active-belief report sanity: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
