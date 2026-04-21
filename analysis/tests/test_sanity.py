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
