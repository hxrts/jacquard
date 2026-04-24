from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from analysis.sanity import validate_report_artifacts


def _write_large(path: Path, prefix: bytes = b"x") -> None:
    path.write_bytes(prefix + (b"x" * 1200))


def _write_svg(path: Path, body: str) -> None:
    padding = " " * 1200
    path.write_text(f"<svg><g role=\"graphics-symbol\"><path d=\"M0,0L1,1\"/></g>{body}{padding}</svg>")


def _write_figure(report_dir: Path, stem: str, body: str = "") -> None:
    _write_svg(report_dir / f"{stem}.svg", body)
    _write_large(report_dir / f"{stem}.png", b"png")
    _write_large(report_dir / f"{stem}.pdf", b"%PDF")


class ReportSanityTests(unittest.TestCase):
    def test_valid_minimal_artifact_passes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            issues = validate_report_artifacts(artifact_dir)
            self.assertEqual([], issues)

    def test_detects_blank_svg(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            (report_dir / "figure.svg").write_text("<svg></svg>")
            _write_large(report_dir / "figure.png", b"png")
            _write_large(report_dir / "figure.pdf", b"%PDF")
            messages = [issue.message for issue in validate_report_artifacts(artifact_dir)]
            self.assertIn("SVG is too small to contain a plot", messages)

    def test_detects_all_zero_headline_series(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            (report_dir / "comparison_summary.csv").write_text(
                "family_id,config_id,dominant_engine,route_present_total_window_permille_mean\n"
                "f,c,pathway,0\n"
            )
            messages = [issue.message for issue in validate_report_artifacts(artifact_dir)]
            self.assertEqual(
                [
                    "headline series is all zero/null: "
                    "route_present_total_window_permille_mean"
                ],
                messages,
            )

    def test_accepts_active_belief_final_csvs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            (report_dir / "active_belief_final_validation.csv").write_text(
                "seed,scenario_regime,mode,task_kind,fixed_payload_budget_bytes,"
                "collective_uncertainty_permille,receiver_agreement_permille,"
                "commitment_lead_time_rounds_max,quality_per_byte_permille,"
                "deterministic_replay\n"
                "41,sparse-bridge-heavy,full-active-belief,majority-threshold,"
                "4096,120,1000,6,4,true\n"
            )
            (report_dir / "active_belief_second_tasks.csv").write_text(
                "seed,mode,task_kind,statistic_kind,receiver_rank,"
                "recovery_probability_permille,bytes_at_commitment,"
                "demand_satisfaction_permille,decision_accuracy_permille,"
                "commitment_lead_time_rounds_max,quality_per_byte_permille\n"
                "41,full-active-belief,bounded-histogram,bounded-histogram,"
                "12,1000,384,500,1000,6,4\n"
            )
            (report_dir / "active_belief_host_bridge_demand.csv").write_text(
                "seed,mode,execution_surface,bridge_batch_id,ingress_round,"
                "replay_visible,demand_contribution_count,evidence_validity_changed,"
                "contribution_identity_created,merge_semantics_changed,"
                "route_truth_published,duplicate_rank_inflation\n"
                "41,full-active-belief,host-bridge-replay,410,4,true,0,false,"
                "false,false,false,false\n"
            )
            (report_dir / "active_belief_theorem_assumptions.csv").write_text(
                "theorem_name,scenario_regime,trace_family,"
                "finite_horizon_model_valid,contact_dependence_assumption,"
                "assumption_status,receiver_arrival_bound_permille,"
                "lower_tail_failure_permille,false_commitment_bound_permille\n"
                "receiver_arrival_reconstruction_bound,sparse-bridge-heavy,"
                "synthetic-sparse-bridge,true,adversarial-with-floor,holds,"
                "760,90,40\n"
            )
            (report_dir / "active_belief_large_regime.csv").write_text(
                "seed,scenario_regime,requested_node_count,executed_node_count,"
                "deterministic_replay,runtime_budget_stable,artifact_sanity_covered\n"
                "41,sparse-bridge-heavy,500,500,true,true,true\n"
            )
            (report_dir / "active_belief_trace_validation.csv").write_text(
                "trace_family,external_or_semi_realistic,canonical_preprocessing,"
                "replay_deterministic,theorem_assumption_status\n"
                "semi-realistic-mobility-contact,true,true,true,holds\n"
            )
            (report_dir / "active_belief_strong_baselines.csv").write_text(
                "seed,baseline_policy,fixed_payload_budget_bytes,"
                "decision_accuracy_permille,quality_per_byte_permille,deterministic\n"
                "41,prophet-contact-frequency,4096,720,175,true\n"
            )
            (report_dir / "active_belief_exact_seed_summary.csv").write_text(
                "seed,scenario_regime,receiver_arrival_probability_permille,"
                "commitment_accuracy_permille,false_commitment_rate_permille,"
                "commitment_lead_time_rounds_max,quality_per_byte_permille\n"
                "41,sparse-bridge-heavy,760,960,40,6,224\n"
            )
            (report_dir / "active_belief_scaling_boundary.csv").write_text(
                "requested_node_count,executed_node_count,documented_boundary,"
                "boundary_reason\n"
                "500,100,true,documented boundary\n"
            )
            (report_dir / "active_belief_figure_artifacts.csv").write_text(
                "figure_index,figure_name,artifact_row_count,fixed_budget_label,"
                "sanity_passed\n"
                "1,landscape-coming-into-focus,3,equal-payload-bytes,true\n"
            )

            self.assertEqual([], validate_report_artifacts(artifact_dir))

    def test_detects_stale_persistence_without_disruption(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            (report_dir / "runs.csv").write_text(
                "run_id,family_id,engine_family,config_id,seed,"
                "route_present_total_window_permille,first_disruption_round_mean,"
                "stale_persistence_round_mean\n"
                "r,f,pathway,c,1,900,,4\n"
            )
            rendered = [issue.render() for issue in validate_report_artifacts(artifact_dir)]
            self.assertIn(
                "runs.csv: 1 row(s) report stale persistence without a disruption round",
                rendered,
            )

    def test_detects_stale_repair_recovery_label(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            _write_figure(report_dir, "routing_fitness_stale_repair", "recov=0.0%")
            rendered = [issue.render() for issue in validate_report_artifacts(artifact_dir)]
            self.assertIn(
                "routing_fitness_stale_repair.svg: stale repair labels must use route presence",
                rendered,
            )
            self.assertIn(
                "routing_fitness_stale_repair.svg: stale repair labels still use recovery success",
                rendered,
            )

    def test_detects_crossover_recovery_series(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            _write_figure(report_dir, "routing_fitness_crossover", "Recovery success")
            rendered = [issue.render() for issue in validate_report_artifacts(artifact_dir)]
            self.assertIn(
                "routing_fitness_crossover.svg: "
                "crossover figure still plots non-headline recovery success",
                rendered,
            )

    def test_detects_missing_mercator_in_engine_set_figure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            (report_dir / "head_to_head_summary.csv").write_text(
                "family_id,config_id,comparison_engine_set,"
                "route_present_total_window_permille_mean\n"
                "head-to-head-connected-low-loss,head-to-head-mercator,mercator,900\n"
            )
            _write_figure(report_dir, "head_to_head_route_presence", "Pathway")

            rendered = [issue.render() for issue in validate_report_artifacts(artifact_dir)]

            self.assertIn(
                "head_to_head_route_presence.svg: "
                "source data includes Mercator but figure does not render it",
                rendered,
            )

    def test_detects_collapsed_mixed_standalone_zero_labels(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            report_dir = artifact_dir / "report"
            report_dir.mkdir()
            _write_large(artifact_dir / "router-tuning-report.pdf", b"%PDF")
            _write_figure(report_dir, "mixed_vs_standalone_divergence", "0.0 pts")
            rendered = [issue.render() for issue in validate_report_artifacts(artifact_dir)]
            self.assertIn(
                "mixed_vs_standalone_divergence.svg: "
                "mixed-vs-standalone divergence labels collapse ties to 0.0 pts",
                rendered,
            )


if __name__ == "__main__":
    unittest.main()
