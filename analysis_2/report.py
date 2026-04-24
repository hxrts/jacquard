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
    ("figure_01_landscape_focus", "Landscape coming into focus", "active_belief_raw_rounds.csv"),
    ("figure_02_path_free_recovery", "Path-free recovery", "active_belief_path_validation.csv"),
    ("figure_03_three_mode_comparison", "Three-mode comparison", "coded_inference_experiment_a2_evidence_modes.csv"),
    ("figure_04_active_belief_grid", "Active belief grid", "active_belief_receiver_runs.csv"),
    ("figure_05_task_algebra", "Task algebra table", "active_belief_second_tasks.csv"),
    ("figure_06_phase_diagram", "Phase diagram", "coded_inference_experiment_c_phase_diagram.csv"),
    ("figure_07_active_vs_passive", "Active versus passive", "active_belief_demand_ablation.csv"),
    ("figure_08_coding_vs_replication", "Coding versus replication", "coded_inference_experiment_d_coding_vs_replication.csv"),
    ("figure_09_recoding_frontier", "Recoding frontier", "active_belief_receiver_runs.csv"),
    ("figure_10_robustness_boundary", "Robustness boundary", "active_belief_exact_seed_summary.csv"),
    ("figure_11_observer_ambiguity", "Observer ambiguity frontier", "coded_inference_experiment_e_observer_frontier.csv"),
    ("figure_12_host_bridge_demand", "Host/bridge demand safety", "active_belief_host_bridge_demand.csv"),
    ("figure_13_theorem_assumptions", "Theorem assumptions by regime", "active_belief_theorem_assumptions.csv"),
    ("figure_14_large_regime", "Large-regime validation", "active_belief_scale_validation.csv"),
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
            }
        )
    return figure_rows, figure_specs


def figure_claim_categories(datasets: dict[str, list[dict[str, object]]]) -> dict[int, str]:
    categories: dict[int, str] = {}
    for row in datasets.get("active_belief_figure_claim_map.csv", []):
        categories[int(row["figure_index"])] = str(row["claim_category"])
    return categories


def figure_caption(figure_id: str, dataset_name: str) -> str:
    descriptions = {
        "figure_01_landscape_focus": (
            "Main evidence. Median lines and interquartile bands summarize multi-seed replay rows for belief quality, margin, and uncertainty over temporal contact. This supports landscape sharpening under fixed payload-byte accounting; it does not by itself prove demand causality."
        ),
        "figure_02_path_free_recovery": (
            "Main evidence. Each distribution uses rows whose core windows have no instantaneous static source-to-receiver path and whose successful runs record time-respecting evidence journeys. This supports the path-free inference claim under the recorded trace families."
        ),
        "figure_03_three_mode_comparison": (
            "Main evidence. Separate normalized panels compare source-coded threshold evidence, distributed local evidence, and recoded aggregate evidence without mixing count and quality units on one axis. This supports task-interface breadth, not universal ML inference."
        ),
        "figure_04_active_belief_grid": (
            "Main evidence. Receiver-run summaries compare active, passive, and recoded modes for agreement, uncertainty, and commitment lead time. The figure supports collective belief improvement under the replayed regimes."
        ),
        "figure_05_task_algebra": (
            "Main evidence. Per-task baseline rows show that anomaly, majority, histogram, and set-union tasks share the direct statistic decoding surface. The result is restricted to compact mergeable tasks."
        ),
        "figure_06_phase_diagram": (
            "Main evidence. Small multiples show quality, duplicate pressure, byte cost, and measured R_est across reproduction bands. The useful region is where quality improves without runaway duplicate or byte pressure."
        ),
        "figure_07_active_vs_passive": (
            "Main evidence. Causal ablation distributions isolate propagated demand against no-demand, local-only, stale-demand, and removed-scoring-term policies under equal payload-byte budgets."
        ),
        "figure_08_coding_vs_replication": (
            "Main evidence. Quality-cost curves compare coded evidence policies with uncoded replication across payload-byte budgets. This is the fair-cost check for the coding benefit."
        ),
        "figure_09_recoding_frontier": (
            "Main evidence. The frontier plots quality against bytes at commitment for passive, active, and recoded modes. Recoding gains should be read together with duplicate non-inflation and demand-safety rows."
        ),
        "figure_10_robustness_boundary": (
            "Main evidence. Split stress panels show commitment accuracy and false-commitment rate by stress severity. The figure identifies modeled robustness boundaries and should not be read as arbitrary-adversary robustness."
        ),
        "figure_11_observer_ambiguity": (
            "Supporting proxy. Observer ambiguity is a measured projection frontier only. It is not a privacy theorem and is not required for the main active-belief thesis."
        ),
        "figure_12_host_bridge_demand": (
            "Boundary/safety evidence. Host/bridge replay rows show that first-class demand never validates evidence, creates contribution identity, alters merge semantics, publishes route truth, or inflates duplicate rank."
        ),
        "figure_13_theorem_assumptions": (
            "Boundary/safety evidence. This is the proof-to-experiment map. Rows marked holds are inside the theorem-backed boundary; empirical-only rows are reported evidence, not proof instances."
        ),
        "figure_14_large_regime": (
            "Supporting scale hygiene. Runtime, memory, replay agreement, quality, and failure-rate rows check deterministic large-regime artifact generation. This is not a production deployment claim."
        ),
        "figure_15_trace_validation": (
            "Supporting artifact hygiene. Trace validation shows canonical preprocessing and deterministic replay for synthetic and semi-realistic inputs. It supports artifact credibility, not a universal mobility claim."
        ),
        "figure_16_strong_baselines": (
            "Main evidence. Multi-seed equal-budget distributions compare active belief diffusion with deterministic opportunistic forwarding baselines. The scope is the recorded baseline set, not a complete DTN survey."
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
