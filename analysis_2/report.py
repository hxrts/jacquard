"""CLI entry point for the active-belief paper report."""

from __future__ import annotations

import shutil
import sys
import tempfile
from pathlib import Path

from .data import active_belief_rows_by_dataset, ensure_dir, load_text, write_csv
from .document import write_pdf_report
from .plots import save_active_belief_plot_artifact
from .sanity import validate_report_artifacts_or_raise

REPORT_PDF_NAME = "active-belief-report.pdf"
FIGURES = (
    ("figure_01_landscape_focus", "Landscape coming into focus", "coded_inference_experiment_a_landscape.csv"),
    ("figure_02_path_free_recovery", "Path-free recovery", "coded_inference_experiment_b_path_free_recovery.csv"),
    ("figure_03_three_mode_comparison", "Three-mode comparison", "coded_inference_experiment_a2_evidence_modes.csv"),
    ("figure_04_active_belief_grid", "Active belief grid", "active_belief_final_validation.csv"),
    ("figure_05_task_algebra", "Task algebra table", "active_belief_second_tasks.csv"),
    ("figure_06_phase_diagram", "Phase diagram", "coded_inference_experiment_c_phase_diagram.csv"),
    ("figure_07_active_vs_passive", "Active versus passive", "active_belief_final_validation.csv"),
    ("figure_08_coding_vs_replication", "Coding versus replication", "coded_inference_experiment_d_coding_vs_replication.csv"),
    ("figure_09_recoding_frontier", "Recoding frontier", "active_belief_final_validation.csv"),
    ("figure_10_robustness_boundary", "Robustness boundary", "active_belief_exact_seed_summary.csv"),
    ("figure_11_observer_ambiguity", "Observer ambiguity frontier", "coded_inference_experiment_e_observer_frontier.csv"),
    ("figure_12_host_bridge_demand", "Host/bridge demand safety", "active_belief_host_bridge_demand.csv"),
    ("figure_13_theorem_assumptions", "Theorem assumptions by regime", "active_belief_theorem_assumptions.csv"),
    ("figure_14_large_regime", "Large-regime validation", "active_belief_large_regime.csv"),
    ("figure_15_trace_validation", "Trace validation", "active_belief_trace_validation.csv"),
    ("figure_16_strong_baselines", "Opportunistic baseline comparison", "active_belief_strong_baselines.csv"),
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
            }
        )
    return figure_rows, figure_specs


def figure_caption(figure_id: str, dataset_name: str) -> str:
    descriptions = {
        "figure_01_landscape_focus": (
            "Read this as the motivating belief-formation trace: higher merged-statistic quality means the receiver's compact evidence ledger is turning intermittent contacts into a sharper task belief. The significance is that useful belief improves without treating route availability as the measured objective."
        ),
        "figure_02_path_free_recovery": (
            "This plot isolates the path-free condition. The important reading is whether reconstruction or useful inference succeeds even when the core window has no static end-to-end path, which is the main separation from ordinary route-continuity claims."
        ),
        "figure_03_three_mode_comparison": (
            "The three points compare source-coded evidence, distributed local evidence, and recoded aggregate evidence under the same accounting surface. The significance is that active belief diffusion is not just payload recovery; it also covers mergeable local observations and safe aggregation."
        ),
        "figure_04_active_belief_grid": (
            "This grid-level summary should be read as a compact view of receiver agreement, uncertainty, and quality per byte. It shows whether demand-aware allocation improves the collective belief statistic rather than merely moving more bytes."
        ),
        "figure_05_task_algebra": (
            "This figure summarizes breadth across compact mergeable tasks. The key point is that the claim is not tied to one anomaly-localization fixture: exact threshold, vote, and histogram-style statistics share the same bounded merge-and-commit discipline."
        ),
        "figure_06_phase_diagram": (
            "Read the phase diagram as the cost-control boundary: subcritical pressure under-delivers, supercritical pressure wastes budget, and the useful region is where quality rises without runaway duplicate pressure."
        ),
        "figure_07_active_vs_passive": (
            "This comparison is the causal active-demand test. Under equal payload bytes, the active mode should improve quality per byte over passive controlled coded diffusion while keeping demand non-evidential."
        ),
        "figure_08_coding_vs_replication": (
            "This plot contrasts coded contribution movement with plain replication under the fixed byte budget. The significance is a fair-cost check: any claimed benefit must appear as better quality or lower cost at the same payload budget."
        ),
        "figure_09_recoding_frontier": (
            "Read this as the aggregation frontier. Gains are meaningful only when recoding preserves contribution lineage, so the plot should be interpreted together with duplicate non-inflation and recoding-soundness rows."
        ),
        "figure_10_robustness_boundary": (
            "This boundary plot marks where the reduced thesis still holds and where it becomes empirical-only or fails. The important use is negative: it keeps the paper from overstating robustness beyond the modeled stress set."
        ),
        "figure_11_observer_ambiguity": (
            "This plot reports observer ambiguity proxies, not formal privacy. Higher uncertainty or lower attacker advantage is evidence about the measured projection surface, but the caption boundary prevents reading it as a privacy theorem."
        ),
        "figure_12_host_bridge_demand": (
            "This is the safety-surface plot for first-class demand messages. Demand may change priority, custody, and allocation, but the rows should show that it never validates evidence, creates contribution identity, alters merge semantics, or publishes route truth."
        ),
        "figure_13_theorem_assumptions": (
            "Use this figure as the proof-to-experiment map. Rows with satisfied assumptions can be read as instances of the Lean-backed boundary; rows outside those assumptions are empirical evidence only."
        ),
        "figure_14_large_regime": (
            "This plot checks scale hygiene rather than claiming production deployment. The important signal is deterministic replay and stable artifact generation at the stated large-regime size."
        ),
        "figure_15_trace_validation": (
            "This trace-validation figure shows whether synthetic and semi-realistic mobility-contact inputs were canonically preprocessed and replayed. It supports artifact credibility, not a universal mobility claim."
        ),
        "figure_16_strong_baselines": (
            "This baseline comparison positions active belief diffusion against deterministic opportunistic forwarding references. The significance is bounded: it checks the stronger paper package against explicit baselines without claiming a complete DTN survey."
        ),
    }
    return (
        f"{descriptions[figure_id]} Source: {dataset_name}. Fixed payload-byte budget, "
        "deterministic replay, and theorem-assumption status are recorded in the corresponding CSV rows."
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
