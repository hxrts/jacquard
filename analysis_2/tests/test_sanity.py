from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from analysis_2.report import write_outputs
from analysis_2.sanity import validate_report_artifacts


class ActiveBeliefReportSanityTests(unittest.TestCase):
    def test_generated_report_passes_sanity(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            write_outputs(artifact_dir)
            self.assertEqual([], validate_report_artifacts(artifact_dir))

    def test_rejects_missing_figure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            write_outputs(artifact_dir)
            figure = next((artifact_dir / "report").glob("figure_01*.svg"))
            figure.unlink()
            rendered = [issue.render() for issue in validate_report_artifacts(artifact_dir)]
            self.assertIn("figure_01: required figure SVG is missing", rendered)

    def test_report_contains_claim_map_and_raw_replay_rows(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            write_outputs(artifact_dir)
            report_dir = artifact_dir / "report"
            claim_map = read_csv(report_dir / "active_belief_figure_claim_map.csv")
            raw_rounds = read_csv(report_dir / "active_belief_raw_rounds.csv")
            receiver_runs = read_csv(report_dir / "active_belief_receiver_runs.csv")
            path_rows = read_csv(report_dir / "active_belief_path_validation.csv")
            demand_rows = read_csv(report_dir / "active_belief_demand_ablation.csv")
            headline_rows = read_csv(report_dir / "active_belief_headline_statistics.csv")
            self.assertEqual(23, len(claim_map))
            self.assertGreaterEqual(len(raw_rounds), 4000)
            self.assertGreaterEqual(len(receiver_runs), 2000)
            self.assertGreaterEqual(len(path_rows), 200)
            self.assertGreaterEqual(len(demand_rows), 1000)
            self.assertGreaterEqual(len(headline_rows), 10)
            self.assertTrue(all(row["no_static_path_in_core_window"] == "True" for row in path_rows))
            self.assertTrue(all(row["time_respecting_evidence_journey_exists"] == "True" for row in path_rows))

    def test_figure_manifest_records_claim_categories(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            write_outputs(artifact_dir)
            figure_rows = read_csv(artifact_dir / "report" / "active_belief_figure_artifacts.csv")
            categories = {row["claim_category"] for row in figure_rows}
            self.assertIn("main-evidence", categories)
            self.assertIn("boundary/safety", categories)
            self.assertIn("appendix/supporting", categories)
            main_sources = [row for row in figure_rows if row["claim_category"] == "main-evidence"]
            for row in main_sources:
                if row["source_artifact"] == "active_belief_headline_statistics.csv":
                    self.assertGreaterEqual(int(row["artifact_row_count"]), 10)
                else:
                    self.assertGreaterEqual(int(row["artifact_row_count"]), 100)


def read_csv(path: Path) -> list[dict[str, str]]:
    import csv

    with path.open(newline="") as handle:
        return list(csv.DictReader(handle))


if __name__ == "__main__":
    unittest.main()
