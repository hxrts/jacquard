"""I/O helpers: load NDJSON and JSON array artifact files, write CSVs, and clean up stale report outputs."""

from __future__ import annotations

import json
from pathlib import Path

import polars as pl


def load_ndjson(path: Path) -> pl.DataFrame:
    rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    return pl.from_dicts(rows, infer_schema_length=None) if rows else pl.DataFrame()


def load_json_array(path: Path) -> pl.DataFrame:
    data = json.loads(path.read_text())
    return pl.from_dicts(data, infer_schema_length=None) if data else pl.DataFrame()


def load_optional_ndjson(path: Path) -> pl.DataFrame:
    if not path.exists():
        return pl.DataFrame()
    return load_ndjson(path)


def load_optional_json_array(path: Path) -> pl.DataFrame:
    if not path.exists():
        return pl.DataFrame()
    return load_json_array(path)


def ensure_dir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def cleanup_report_dir(report_dir: Path) -> None:
    for name in [
        "babel_decay_loss.png",
        "babel_decay_stability.png",
        "batman_bellman_transition_loss.png",
        "batman_bellman_transition_stability.png",
        "batman_classic_transition_loss.png",
        "batman_classic_transition_stability.png",
        "field_budget_reconfiguration.png",
        "field_budget_route_presence.png",
        "comparison_dominant_engine.png",
        "pathway_budget_activation.png",
        "pathway_budget_route_presence.png",
        "diffusion_delivery_coverage.png",
        "diffusion_resource_boundedness.png",
        "report.pdf",
        "tuning_report.pdf",
    ]:
        stale = report_dir / name
        if stale.exists():
            stale.unlink()


def write_csv(df: pl.DataFrame, path: Path) -> None:
    if df.is_empty():
        path.write_text("")
        return
    df.write_csv(path)
