"""CLI entry point for the active-belief paper report."""

from __future__ import annotations

import shutil
import sys
import tempfile
import re
from pathlib import Path

from .data import active_belief_rows_by_dataset, ensure_dir, load_text, write_csv
from .document import write_pdf_report
from .plots import bool_value, compact_theorem, display_label, int_value, metric_label, save_active_belief_plot_artifact
from .sanity import validate_report_artifacts_or_raise

REPORT_PDF_NAME = "active-belief-report.pdf"
FIGURES = (
    ("figure_01_landscape_focus", "Landscape coming into focus", "active_belief_raw_rounds.csv"),
    ("figure_02_path_free_recovery", "Path-free recovery", "active_belief_path_validation.csv"),
    ("figure_03_three_mode_comparison", "Three-mode task surface", "coded_inference_experiment_a2_evidence_modes.csv"),
    ("figure_04_active_belief_grid", "Multi-receiver belief compatibility", "active_belief_receiver_runs.csv"),
    ("figure_05_task_algebra", "Task algebra table", "active_belief_second_tasks.csv"),
    ("figure_06_phase_diagram", "Control operating region", "coded_inference_experiment_c_phase_diagram.csv"),
    ("figure_07_active_vs_passive", "Active versus passive", "active_belief_demand_ablation.csv"),
    ("figure_08_coding_vs_replication", "Coding versus replication with spread", "coded_inference_experiment_d_coding_vs_replication.csv"),
    ("figure_09_recoding_frontier", "Regime-specific frontier summary", "active_belief_receiver_runs.csv"),
    ("figure_10_robustness_boundary", "Robustness boundary", "active_belief_exact_seed_summary.csv"),
    ("figure_11_observer_ambiguity", "Observer ambiguity frontier", "coded_inference_experiment_e_observer_frontier.csv"),
    ("figure_12_host_bridge_demand", "Demand safety audit", "active_belief_host_bridge_demand.csv"),
    ("figure_13_theorem_assumptions", "Theorem boundary table", "active_belief_theorem_assumptions.csv"),
    ("figure_14_large_regime", "Large-regime validation", "active_belief_scale_validation.csv"),
    ("figure_15_trace_validation", "Trace validation table", "active_belief_trace_validation.csv"),
    ("figure_16_strong_baselines", "Baseline fairness check", "active_belief_strong_baselines.csv"),
    ("figure_17_headline_statistics", "Headline statistical summary", "active_belief_headline_statistics.csv"),
)


def write_outputs(artifact_dir: Path) -> None:
    ensure_dir(artifact_dir)
    datasets = active_belief_rows_by_dataset()
    with tempfile.TemporaryDirectory(dir=artifact_dir, prefix=".analysis2-staging-") as tmp:
        staging = Path(tmp)
        report_dir = staging / "report"
        ensure_dir(report_dir)
        for name, rows in datasets.items():
            if name != "active_belief_figure_artifacts.csv":
                write_csv(report_dir / name, rows)
        figure_rows, figure_specs = build_figures(report_dir, datasets)
        validate_manuscript_exhibit_references(load_text(Path("analysis_2/text.md")), figure_specs)
        write_csv(report_dir / "active_belief_figure_artifacts.csv", figure_rows)
        write_pdf_report(
            artifact_dir,
            report_dir,
            staging / REPORT_PDF_NAME,
            load_text(Path("analysis_2/text.md")),
            figure_specs,
            figure_rows,
        )
        replace_path(report_dir, artifact_dir / "report")
        replace_path(staging / REPORT_PDF_NAME, artifact_dir / REPORT_PDF_NAME)


def build_figures(
    report_dir: Path,
    datasets: dict[str, list[dict[str, object]]],
) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    figure_rows: list[dict[str, object]] = []
    figure_specs: list[dict[str, object]] = []
    claim_categories = figure_claim_categories(datasets)
    for index, (figure_id, title, dataset_name) in enumerate(FIGURES, start=1):
        values, labels = save_active_belief_plot_artifact(
            report_dir,
            figure_id,
            title,
            datasets[dataset_name],
            dataset_name,
        )
        caption = figure_caption(figure_id, dataset_name)
        figure_rows.append(
            {
                "figure_index": index,
                "figure_name": title,
                "source_artifact": dataset_name,
                "artifact_row_count": len(datasets[dataset_name]),
                "claim_category": claim_categories.get(index, "main-evidence"),
                "fixed_budget_label": "equal-payload-bytes",
                "sanity_passed": True,
            }
        )
        figure_specs.append(
            {
                "figure_index": index,
                "figure_id": figure_id,
                "figure_name": title,
                "source_artifact": dataset_name,
                "values": values,
                "labels": labels,
                "caption": caption,
                "display_kind": figure_display_kind(figure_id),
                "table": figure_table(figure_id, datasets[dataset_name]),
            }
        )
    return figure_rows, figure_specs


def figure_display_kind(figure_id: str) -> str:
    if figure_id in {
        "figure_03_three_mode_comparison",
        "figure_12_host_bridge_demand",
        "figure_13_theorem_assumptions",
        "figure_15_trace_validation",
        "figure_17_headline_statistics",
    }:
        return "table"
    return "figure"


def figure_table(figure_id: str, rows: list[dict[str, object]]) -> dict[str, object] | None:
    if figure_id == "figure_03_three_mode_comparison":
        return three_mode_surface_table(rows)
    if figure_id == "figure_12_host_bridge_demand":
        return host_bridge_demand_table(rows)
    if figure_id == "figure_13_theorem_assumptions":
        return theorem_assumption_table(rows)
    if figure_id == "figure_15_trace_validation":
        return trace_validation_table(rows)
    if figure_id == "figure_17_headline_statistics":
        return headline_statistics_table(rows)
    return None


def validate_manuscript_exhibit_references(markdown: str, figure_specs: list[dict[str, object]]) -> None:
    specs_by_id = {str(spec["figure_id"]): spec for spec in figure_specs}
    expected_by_number = {
        int(spec["figure_index"]): ("Table" if str(spec.get("display_kind")) == "table" else "Figure")
        for spec in figure_specs
    }
    markers = re.findall(r"\{\{EXHIBIT:([a-zA-Z0-9_]+)\}\}", markdown)
    unknown = sorted(marker for marker in set(markers) if marker not in specs_by_id)
    if unknown:
        raise RuntimeError(f"unknown active-belief exhibit markers: {', '.join(unknown)}")
    mismatches: list[str] = []
    for label, number_text in re.findall(r"\b(Figure|Table)\s+(\d+)\b", markdown):
        number = int(number_text)
        expected = expected_by_number.get(number)
        if expected is not None and expected != label:
            mismatches.append(f"{label} {number} should be {expected} {number}")
    if mismatches:
        raise RuntimeError("active-belief exhibit reference mismatch: " + "; ".join(mismatches))


def three_mode_surface_table(rows: list[dict[str, object]]) -> dict[str, object]:
    grouped: dict[str, dict[str, list[int] | str]] = {}
    object_labels = {
        "source-coded-threshold": "independent payload fragments",
        "distributed-local-evidence": "local statistic contributions",
        "recoded-aggregate": "recoded aggregate contributions",
    }
    for row in rows:
        mode = str(row["policy_or_mode"])
        entry = grouped.setdefault(
            mode,
            {
                "statistic_kind": display_label(str(row["statistic_kind"])),
                "merge_operation": display_label(str(row["merge_operation"])),
                "useful": [],
                "rank": [],
                "quality": [],
                "uncertainty": [],
            },
        )
        cast = entry
        cast["useful"].append(int_value(row, "useful_contribution_count"))  # type: ignore[attr-defined]
        cast["rank"].append(int_value(row, "receiver_rank"))  # type: ignore[attr-defined]
        cast["quality"].append(int_value(row, "merged_statistic_quality_permille"))  # type: ignore[attr-defined]
        cast["uncertainty"].append(int_value(row, "uncertainty_permille"))  # type: ignore[attr-defined]
    table_rows = []
    order = [
        "source-coded-threshold",
        "distributed-local-evidence",
        "recoded-aggregate",
    ]
    for mode in order:
        if mode not in grouped:
            continue
        entry = grouped[mode]
        useful = median_label(entry["useful"])  # type: ignore[arg-type]
        rank = median_label(entry["rank"])  # type: ignore[arg-type]
        quality = median_label(entry["quality"])  # type: ignore[arg-type]
        uncertainty = median_label(entry["uncertainty"])  # type: ignore[arg-type]
        table_rows.append(
            [
                display_label(mode),
                object_labels[mode],
                str(entry["statistic_kind"]),
                useful,
                rank,
                quality,
                uncertainty,
            ]
        )
    return {
        "columns": [
            "Mode",
            "Encoded object",
            "Statistic",
            "Median useful",
            "Median rank",
            "Median quality",
            "Median uncertainty",
        ],
        "rows": table_rows,
        "widths": [2.0, 3.8, 2.1, 1.8, 1.5, 1.8, 1.8],
    }


def host_bridge_demand_table(rows: list[dict[str, object]]) -> dict[str, object]:
    mode_order = [
        "passive-controlled-coded",
        "full-active-belief",
        "stale-demand-ablation",
    ]
    grouped: dict[str, list[dict[str, object]]] = {mode: [] for mode in mode_order}
    for row in rows:
        grouped.setdefault(str(row["mode"]), []).append(row)
    table_rows = [
        [
            "replay-visible demand summaries",
            median_label([int_value(row, "demand_contribution_count") for row in grouped["passive-controlled-coded"]]),
            median_label([int_value(row, "demand_contribution_count") for row in grouped["full-active-belief"]]),
            median_label([int_value(row, "demand_contribution_count") for row in grouped["stale-demand-ablation"]]),
            "Demand is present as an explicit replay-visible message only in the active variants.",
        ]
    ]
    safety_fields = {
        "evidence_validity_changed": "Demand cannot validate or invalidate evidence.",
        "contribution_identity_created": "Demand cannot create contribution identity.",
        "merge_semantics_changed": "Demand cannot alter merge semantics.",
        "route_truth_published": "Demand cannot publish route truth.",
        "duplicate_rank_inflation": "Demand cannot inflate duplicate rank.",
    }
    for field, interpretation in safety_fields.items():
        counts = [
            sum(1 for row in grouped["passive-controlled-coded"] if bool_value(row, field)),
            sum(1 for row in grouped["full-active-belief"] if bool_value(row, field)),
            sum(1 for row in grouped["stale-demand-ablation"] if bool_value(row, field)),
        ]
        table_rows.append(
            [
                metric_label(field),
                *(str(count) for count in counts),
                interpretation,
            ]
        )
    return {
        "columns": [
            "Audit row",
            "Passive coded",
            "Active belief",
            "Stale demand",
            "Interpretation",
        ],
        "rows": table_rows,
        "widths": [4.0, 1.6, 1.6, 1.6, 6.2],
    }


def theorem_assumption_table(rows: list[dict[str, object]]) -> dict[str, object]:
    statuses: dict[str, dict[str, str]] = {}
    profiles: dict[str, str] = {}
    bounds: dict[str, str] = {}
    for row in rows:
        theorem = str(row["theorem_name"])
        scenario = display_label(str(row["scenario_regime"]))
        statuses.setdefault(theorem, {})[scenario] = str(row["assumption_status"])
        profiles.setdefault(theorem, str(row.get("theorem_profile", "")))
        if str(row["assumption_status"]) == "holds" and theorem not in bounds:
            bounds[theorem] = str(
                row.get(
                    "bound_summary",
                    (
                        f"arrival >= {int_value(row, 'receiver_arrival_bound_permille')}; "
                        f"tail <= {int_value(row, 'lower_tail_failure_permille')}; "
                        f"false <= {int_value(row, 'false_commitment_bound_permille')}"
                    ),
                )
            )
    scenario_columns = ["sparse bridge", "clustered", "mobility"]
    table_rows = []
    for theorem in sorted(statuses):
        table_rows.append(
            [
                compact_theorem(theorem),
                profiles.get(theorem, "-"),
                *[statuses[theorem].get(scenario, "missing") for scenario in scenario_columns],
                bounds.get(theorem, "empirical only"),
            ]
        )
    return {
        "columns": ["Theorem", "Profile", "Sparse bridge", "Clustered", "Mobility", "Best theorem-backed scope"],
        "rows": table_rows,
        "widths": [4.0, 3.2, 1.8, 1.8, 1.8, 4.7],
    }


def trace_validation_table(rows: list[dict[str, object]]) -> dict[str, object]:
    table_rows = []
    for row in rows:
        table_rows.append(
            [
                display_label(str(row["trace_family"])),
                yes_no(bool_value(row, "canonical_preprocessing")),
                yes_no(bool_value(row, "replay_deterministic")),
                yes_no(bool_value(row, "external_or_semi_realistic")),
                str(row["theorem_assumption_status"]),
            ]
        )
    return {
        "columns": ["Trace family", "Canonical preprocessing", "Deterministic replay", "Semi-real/external", "Theorem status"],
        "rows": table_rows,
        "widths": [4.1, 3.0, 3.0, 2.5, 2.4],
    }


def headline_statistics_table(rows: list[dict[str, object]]) -> dict[str, object]:
    table_rows = []
    for row in rows:
        table_rows.append(
            [
                str(row["comparison"]),
                metric_label(str(row["metric"])),
                str(row["treatment_median"]),
                str(row["baseline_median"]),
                str(row["paired_delta_median"]),
                f"[{row['paired_delta_p25']}, {row['paired_delta_p75']}]",
                str(row["aggregation_unit"]),
            ]
        )
    return {
        "columns": ["Comparison", "Metric", "Treatment median", "Baseline median", "Median delta", "IQR delta", "Unit"],
        "rows": table_rows,
        "widths": [4.1, 2.9, 2.1, 2.1, 1.8, 1.8, 2.2],
    }


def yes_no(value: bool) -> str:
    return "yes" if value else "no"


def median_label(values: list[int]) -> str:
    if not values:
        return "-"
    ordered = sorted(values)
    return str(ordered[len(ordered) // 2])


def figure_claim_categories(datasets: dict[str, list[dict[str, object]]]) -> dict[int, str]:
    categories: dict[int, str] = {}
    for row in datasets.get("active_belief_figure_claim_map.csv", []):
        categories[int(row["figure_index"])] = str(row["claim_category"])
    return categories


def figure_caption(figure_id: str, dataset_name: str) -> str:
    descriptions = {
        "figure_01_landscape_focus": (
            "Main evidence. Median lines and interquartile bands show belief quality rising while uncertainty falls over replay rounds in the anomaly-localization setting. This supports landscape sharpening under fixed payload-byte accounting; it does not by itself prove demand causality."
        ),
        "figure_02_path_free_recovery": (
            "Main evidence. Each distribution uses rows whose core windows have no instantaneous static source-to-receiver path and whose successful runs record time-respecting evidence journeys. This is the direct path-free inference check under the recorded trace families."
        ),
        "figure_03_three_mode_comparison": (
            "Main evidence. The table distinguishes the threshold reconstruction case from the two score-vector cases at the encoded-object and statistic level, then reports the median useful contribution, rank, quality, and uncertainty rows. This supports the mergeable-task-interface claim directly."
        ),
        "figure_04_active_belief_grid": (
            "Main evidence. Receiver-level distributions compare active, passive, recoded, and uncoded modes in direct units for quality, uncertainty, agreement, and commitment lead time across the three regimes. This is the paper's current multi-receiver evidence object."
        ),
        "figure_05_task_algebra": (
            "Main evidence. Per-task distributions show that anomaly, majority, histogram, and set-union tasks share the same direct statistic-decoding surface. The result is restricted to compact mergeable tasks."
        ),
        "figure_06_phase_diagram": (
            "Main evidence. The operating-region panels show where measured reproduction pressure enters the near-critical target band and where quality gains stop justifying duplicate and byte pressure. Near-critical runs form the useful control region."
        ),
        "figure_07_active_vs_passive": (
            "Main evidence. Causal ablation distributions isolate propagated demand against no-demand, local-only, stale-demand, and removed-scoring-term policies under equal payload-byte budgets. The propagated-demand variant should dominate the quality-per-byte distribution."
        ),
        "figure_08_coding_vs_replication": (
            "Main evidence. Median quality curves and interquartile bands compare coded evidence policies with uncoded replication across payload-byte budgets. This is the fair-cost check for the coding benefit."
        ),
        "figure_09_recoding_frontier": (
            "Main evidence. Regime-specific frontier summaries place the median bytes-at-commitment and quality point for passive, active, and recoded modes together with interquartile spreads. This makes frontier tradeoffs visible without overplot clutter."
        ),
        "figure_10_robustness_boundary": (
            "Main evidence. Split stress panels show commitment accuracy degrading and false-commitment rate rising with stress severity. The figure identifies modeled robustness boundaries and should not be read as arbitrary-adversary robustness."
        ),
        "figure_11_observer_ambiguity": (
            "Supporting proxy. Observer ambiguity is a measured projection frontier only. It is not a privacy theorem and is not required for the main active-belief thesis."
        ),
        "figure_12_host_bridge_demand": (
            "Boundary/safety evidence. The audit table combines replay-visible demand counts with zero observed violations of evidence validity, contribution identity, merge semantics, route truth, and duplicate-rank safety."
        ),
        "figure_13_theorem_assumptions": (
            "Boundary/safety evidence. This proof-to-experiment table marks which regimes are inside the theorem-backed boundary and includes the strongest bound row carried by each theorem. Empirical-only entries are reported evidence, not proof instances."
        ),
        "figure_14_large_regime": (
            "Supporting scale hygiene. Runtime, memory, replay agreement, quality, and failure-rate rows check deterministic large-regime artifact generation. This is not a production deployment claim."
        ),
        "figure_15_trace_validation": (
            "Supporting artifact hygiene. This trace table records canonical preprocessing and deterministic replay for synthetic and semi-realistic inputs. It supports artifact credibility, not a universal mobility claim."
        ),
        "figure_16_strong_baselines": (
            "Supporting fairness check. Multi-seed equal-budget distributions compare active belief diffusion with deterministic opportunistic forwarding baselines and show that the gains are not an artifact of comparing only against weak controls. The scope is the recorded baseline set, not a complete DTN survey."
        ),
        "figure_17_headline_statistics": (
            "Main evidence summary. The table reports deterministic paired median differences and interquartile paired-difference intervals for the headline active-versus-baseline and demand-ablation claims. Positive deltas favor active belief for quality and lead time; negative deltas favor active belief for uncertainty."
        ),
    }
    return (
        f"{descriptions[figure_id]} Source: {dataset_name}. Fixed payload-byte budget, "
        "seed set, trace family, deterministic replay status, and theorem-assumption status are recorded in the report CSV rows."
    )


def replace_path(source: Path, destination: Path) -> None:
    if destination.exists():
        if destination.is_dir():
            shutil.rmtree(destination)
        else:
            destination.unlink()
    shutil.move(str(source), str(destination))


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) != 1:
        print("usage: python -m analysis_2.report <artifact-dir>", file=sys.stderr)
        return 1
    artifact_dir = Path(argv[0]).resolve()
    write_outputs(artifact_dir)
    validate_report_artifacts_or_raise(artifact_dir)
    print(f"Active-belief report: {artifact_dir / REPORT_PDF_NAME}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
