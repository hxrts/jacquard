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


if __name__ == "__main__":
    unittest.main()
