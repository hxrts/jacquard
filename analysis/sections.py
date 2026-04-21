"""Parse text.md into named sections and provide typed line accessors used by the PDF report builder."""

from __future__ import annotations

from functools import lru_cache
from pathlib import Path
import re
from dataclasses import dataclass

import polars as pl

from .scoring import (
    top_recommendation_row,
    top_recommendation_rows,
)

BODY_PATH = Path(__file__).with_name("text.md")


@dataclass(frozen=True)
class AssetBlock:
    kind: str
    section_title: str
    asset_id: str
    lines: list[str]
    description_lines: list[str]


def _slugify(title: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")
    return slug


def _normalize_section(lines: list[str]) -> list[str]:
    normalized: list[str] = []
    paragraph: list[str] = []

    def flush_paragraph() -> None:
        nonlocal paragraph
        if paragraph:
            normalized.append(" ".join(part.strip() for part in paragraph if part.strip()))
            paragraph = []

    for raw_line in lines:
        line = raw_line.rstrip()
        stripped = line.strip()
        if not stripped:
            flush_paragraph()
            if normalized and normalized[-1] != "":
                normalized.append("")
            continue
        if stripped.startswith("- "):
            flush_paragraph()
            normalized.append(stripped)
            continue
        paragraph.append(stripped)

    flush_paragraph()
    while normalized and normalized[-1] == "":
        normalized.pop()
    return normalized


@lru_cache(maxsize=1)
def _parsed_body() -> tuple[dict[str, list[str]], dict[str, tuple[str, ...]]]:
    sections: dict[str, list[str]] = {}
    headings: dict[str, tuple[str, ...]] = {}
    current_key: str | None = None
    current_lines: list[str] = []
    stack: list[str] = []
    previous_level = 0

    for raw_line in BODY_PATH.read_text().splitlines():
        match = re.match(r"^(#+)\s+(.*\S)\s*$", raw_line)
        if match:
            if current_key is not None:
                sections[current_key] = _normalize_section(current_lines)
            level = len(match.group(1))
            title = match.group(2)
            if previous_level == 0:
                if level != 1:
                    raise ValueError(
                        f"{BODY_PATH} must start at heading level 1, found level {level} for {title!r}"
                    )
            elif level > previous_level + 1:
                raise ValueError(
                    f"{BODY_PATH} skips heading depth from {previous_level} to {level} at {title!r}"
                )
            if level <= len(stack):
                stack = stack[: level - 1]
            stack.append(title)
            current_key = _slugify(title)
            if current_key in headings:
                raise ValueError(f"duplicate section title in {BODY_PATH}: {title!r}")
            headings[current_key] = tuple(stack)
            current_lines = []
            previous_level = level
            continue
        current_lines.append(raw_line)

    if current_key is not None:
        sections[current_key] = _normalize_section(current_lines)
    return sections, headings


def section_lines(section: str) -> list[str]:
    sections, _ = _parsed_body()
    key = _section_key(section)
    return list(sections[key])


def document_title() -> str:
    _, headings = _parsed_body()
    try:
        return next(iter(headings.values()))[0]
    except StopIteration as exc:
        raise ValueError(f"missing document title in {BODY_PATH}") from exc


def _section_key(section: str) -> str:
    sections, headings = _parsed_body()
    if "/" in section:
        key = _slugify(section.rsplit("/", 1)[-1])
        expected_path = tuple(part.strip() for part in section.split("/"))
        if headings.get(key) != expected_path:
            raise KeyError(f"missing report body section path: {section}")
    else:
        key = _slugify(section)
    if key not in sections:
        raise KeyError(f"missing report body section: {section}")
    return key


def section_lines_formatted(section: str, **kwargs: object) -> list[str]:
    lines = section_lines(section)
    return [line.format(**kwargs) if line else "" for line in lines]


def asset_block(section: str, expected_kind: str | None = None) -> AssetBlock:
    sections, headings = _parsed_body()
    key = _section_key(section)
    lines = list(sections[key])
    if not lines:
        raise ValueError(f"report body section has no content: {section}")
    marker = lines[0].strip()
    match = re.fullmatch(r"@(?P<kind>table|figure)\s+(?P<asset_id>[a-z0-9_-]+)", marker)
    if not match:
        raise ValueError(f"report body section is missing an asset marker: {section}")
    kind = match.group("kind")
    if expected_kind is not None and kind != expected_kind:
        raise ValueError(
            f"report body section {section!r} expected @{expected_kind}, found @{kind}"
        )
    raw_lines = lines[1:]
    intro_lines: list[str] = []
    description_lines: list[str] = []
    guide_prefixes = ["Column guide: ", "Interpretation guide: "]
    for line in raw_lines:
        if not line:
            intro_lines.append(line)
            continue
        extracted = False
        for prefix in guide_prefixes:
            idx = line.find(prefix)
            if idx >= 0:
                before = line[:idx].rstrip()
                after = line[idx + len(prefix):]
                if before:
                    intro_lines.append(before)
                if after:
                    description_lines.append(after)
                extracted = True
                break
        if not extracted:
            intro_lines.append(line)
    while intro_lines and intro_lines[-1] == "":
        intro_lines.pop()
    return AssetBlock(
        kind=kind,
        section_title=headings[key][-1],
        asset_id=match.group("asset_id"),
        lines=intro_lines,
        description_lines=description_lines,
    )


@lru_cache(maxsize=1)
def _asset_block_index() -> dict[str, AssetBlock]:
    sections, headings = _parsed_body()
    blocks: dict[str, AssetBlock] = {}
    for key, lines in sections.items():
        if not lines:
            continue
        marker = lines[0].strip()
        match = re.fullmatch(r"@(?P<kind>table|figure)\s+(?P<asset_id>[a-z0-9_-]+)", marker)
        if match is None:
            continue
        block = asset_block("/".join(headings[key]))
        asset_id = match.group("asset_id")
        if asset_id in blocks:
            raise ValueError(f"duplicate asset id in {BODY_PATH}: {asset_id!r}")
        blocks[asset_id] = block
    return blocks


def asset_block_by_id(asset_id: str, expected_kind: str | None = None) -> AssetBlock:
    blocks = _asset_block_index()
    if asset_id not in blocks:
        raise KeyError(f"missing report asset: {asset_id}")
    block = blocks[asset_id]
    if expected_kind is not None and block.kind != expected_kind:
        raise ValueError(
            f"report asset {asset_id!r} expected @{expected_kind}, found @{block.kind}"
        )
    return block


def comparison_findings_lines(comparison_summary: pl.DataFrame) -> list[str]:
    lines: list[str] = []
    for family_id in comparison_summary["family_id"].unique().sort().to_list():
        family = (
            comparison_summary.filter(pl.col("family_id") == family_id)
            .sort("route_present_total_window_permille_mean", descending=True)
            .head(1)
        )
        if family.is_empty():
            continue
        row = family.iter_rows(named=True).__next__()
        dominant = row["dominant_engine"] if row["dominant_engine"] is not None else "none"
        lines.append(
            f"`{family_id}`: selected_rounds_leader={dominant}, "
            f"activation={row['activation_success_permille_mean']}, "
            f"total_route_presence={row['route_present_total_window_permille_mean']}"
        )
    return lines


def head_to_head_findings_lines(head_to_head_summary: pl.DataFrame) -> list[str]:
    if head_to_head_summary.is_empty():
        return section_lines("Head-To-Head Findings Empty")
    lines = section_lines("Head-To-Head Findings Intro")
    for family_id in head_to_head_summary["family_id"].unique().sort().to_list():
        family = (
            head_to_head_summary.filter(pl.col("family_id") == family_id)
            .sort(
                [
                    "route_present_total_window_permille_mean",
                    "route_present_active_window_permille_mean",
                    "activation_success_permille_mean",
                ],
                descending=[True, True, True],
            )
            .head(1)
        )
        if family.is_empty():
            continue
        row = family.iter_rows(named=True).__next__()
        lines.append(
            f"`{family_id}`: best engine set=`{row['comparison_engine_set'] or 'none'}`, activation={row['activation_success_permille_mean']} permille, total-window route presence={row['route_present_total_window_permille_mean']} permille."
        )
    return lines


def head_to_head_regime_lines() -> list[str]:
    return section_lines("Head-To-Head Regimes")


def best_head_to_head_rows(head_to_head_summary: pl.DataFrame) -> dict[str, dict]:
    rows: dict[str, dict] = {}
    for family_id in head_to_head_summary["family_id"].unique().sort().to_list():
        family = (
            head_to_head_summary.filter(pl.col("family_id") == family_id)
            .sort(
                [
                    "route_present_total_window_permille_mean",
                    "route_present_active_window_permille_mean",
                    "activation_success_permille_mean",
                ],
                descending=[True, True, True],
            )
            .head(1)
        )
        if family.is_empty():
            continue
        rows[family_id] = family.iter_rows(named=True).__next__()
    return rows


def head_to_head_row_for_engine(
    head_to_head_summary: pl.DataFrame,
    family_id: str,
    comparison_engine_set: str,
) -> dict | None:
    row = head_to_head_summary.filter(
        (pl.col("family_id") == family_id)
        & (pl.col("comparison_engine_set") == comparison_engine_set)
    ).head(1)
    if row.is_empty():
        return None
    return row.iter_rows(named=True).__next__()


def _join_code_names(names: list[str]) -> str:
    quoted = [f"`{name}`" for name in names]
    if not quoted:
        return "`none`"
    if len(quoted) == 1:
        return quoted[0]
    if len(quoted) == 2:
        return f"{quoted[0]} and {quoted[1]}"
    return ", ".join(quoted[:-1]) + f", and {quoted[-1]}"


def _top_head_to_head_rows(
    head_to_head_summary: pl.DataFrame,
    family_id: str,
) -> list[dict]:
    family = head_to_head_summary.filter(pl.col("family_id") == family_id)
    if family.is_empty():
        return []
    best = (
        family.sort(
            [
                "route_present_total_window_permille_mean",
                "route_present_active_window_permille_mean",
                "activation_success_permille_mean",
                "comparison_engine_set",
            ],
            descending=[True, True, True, False],
        )
        .head(1)
        .row(0, named=True)
    )
    return list(
        family.filter(
            (
                pl.col("route_present_active_window_permille_mean")
                == best["route_present_active_window_permille_mean"]
            )
            & (
                pl.col("route_present_total_window_permille_mean")
                == best["route_present_total_window_permille_mean"]
            )
            & (
                pl.col("activation_success_permille_mean")
                == best["activation_success_permille_mean"]
            )
        )
        .sort("comparison_engine_set")
        .iter_rows(named=True)
    )


def _route_summary_row(
    route_summary: pl.DataFrame,
    topology_class: str,
    comparison_engine_set: str,
) -> dict | None:
    row = route_summary.filter(
        (pl.col("topology_class") == topology_class)
        & (pl.col("comparison_engine_set") == comparison_engine_set)
    ).head(1)
    if row.is_empty():
        return None
    return row.row(0, named=True)


def _top_large_population_rows(
    route_summary: pl.DataFrame,
    topology_class: str,
) -> list[dict]:
    family = route_summary.filter(pl.col("topology_class") == topology_class)
    if family.is_empty():
        return []
    best = (
        family.sort(["high_route_present", "comparison_engine_set"], descending=[True, False])
        .head(1)
        .row(0, named=True)
    )
    return list(
        family.filter(pl.col("high_route_present") == best["high_route_present"])
        .sort("comparison_engine_set")
        .iter_rows(named=True)
    )


def head_to_head_takeaway_lines(head_to_head_summary: pl.DataFrame) -> list[str]:
    if head_to_head_summary.is_empty():
        return []

    connected_high_loss_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-connected-high-loss"
    )
    bridge_transition_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-bridge-transition"
    )
    medium_bridge_repair_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-medium-bridge-repair"
    )
    partial_bridge_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-partial-observability-bridge"
    )
    concurrent_mixed_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-concurrent-mixed"
    )
    field_connected_high_loss = head_to_head_row_for_engine(
        head_to_head_summary,
        "head-to-head-connected-high-loss",
        "field",
    )
    field_bridge_transition = head_to_head_row_for_engine(
        head_to_head_summary,
        "head-to-head-bridge-transition",
        "field",
    )
    field_corridor_uncertainty = head_to_head_row_for_engine(
        head_to_head_summary,
        "head-to-head-corridor-continuity-uncertainty",
        "field",
    )
    if (
        not connected_high_loss_rows
        or not bridge_transition_rows
        or not medium_bridge_repair_rows
        or not partial_bridge_rows
        or not concurrent_mixed_rows
        or field_connected_high_loss is None
        or field_bridge_transition is None
        or field_corridor_uncertainty is None
    ):
        return []

    connected_high_loss = connected_high_loss_rows[0]
    bridge_transition = bridge_transition_rows[0]
    medium_bridge_repair = medium_bridge_repair_rows[0]
    partial_bridge = partial_bridge_rows[0]
    concurrent_mixed = concurrent_mixed_rows[0]
    return section_lines_formatted(
        "Head-To-Head Takeaways",
        connected_high_loss_engine_sets=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in connected_high_loss_rows]
        ),
        connected_high_loss_route_presence=connected_high_loss[
            "route_present_total_window_permille_mean"
        ],
        bridge_transition_engine_sets=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in bridge_transition_rows]
        ),
        bridge_transition_route_presence=bridge_transition[
            "route_present_total_window_permille_mean"
        ],
        medium_bridge_repair_engine_sets=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in medium_bridge_repair_rows]
        ),
        medium_bridge_repair_route_presence=medium_bridge_repair[
            "route_present_total_window_permille_mean"
        ],
        partial_bridge_engine_sets=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in partial_bridge_rows]
        ),
        partial_bridge_route_presence=partial_bridge[
            "route_present_total_window_permille_mean"
        ],
        concurrent_mixed_engine_sets=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in concurrent_mixed_rows]
        ),
        concurrent_mixed_route_presence=concurrent_mixed[
            "route_present_total_window_permille_mean"
        ],
        field_connected_high_loss_route_presence=field_connected_high_loss[
            "route_present_total_window_permille_mean"
        ],
        field_bridge_transition_route_presence=field_bridge_transition[
            "route_present_total_window_permille_mean"
        ],
        field_corridor_uncertainty_route_presence=field_corridor_uncertainty[
            "route_present_total_window_permille_mean"
        ],
    )


def analysis_takeaway_lines(
    recommendations: pl.DataFrame,
    comparison_summary: pl.DataFrame,
    head_to_head_summary: pl.DataFrame,
) -> list[str]:
    if comparison_summary.is_empty() or head_to_head_summary.is_empty():
        return []

    comparison_rows = {
        row["family_id"]: row for row in comparison_summary.iter_rows(named=True)
    }
    head_to_head_rows = best_head_to_head_rows(head_to_head_summary)

    connected_low_loss = comparison_rows.get("comparison-connected-low-loss")
    connected_high_loss = comparison_rows.get("comparison-connected-high-loss")
    bridge_transition = comparison_rows.get("comparison-bridge-transition")
    corridor = comparison_rows.get("comparison-corridor-continuity-uncertainty")
    partial_bridge = comparison_rows.get("comparison-partial-observability-bridge")
    concurrent_mixed_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-concurrent-mixed"
    )
    comparison_concurrent_mixed = comparison_rows.get("comparison-concurrent-mixed")
    head_to_head_connected_high_loss_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-connected-high-loss"
    )
    head_to_head_bridge_transition_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-bridge-transition"
    )
    head_to_head_corridor_rows = _top_head_to_head_rows(
        head_to_head_summary, "head-to-head-corridor-continuity-uncertainty"
    )
    field_connected_high_loss = head_to_head_row_for_engine(
        head_to_head_summary,
        "head-to-head-connected-high-loss",
        "field",
    )
    field_bridge_transition = head_to_head_row_for_engine(
        head_to_head_summary,
        "head-to-head-bridge-transition",
        "field",
    )
    field_corridor_uncertainty = head_to_head_row_for_engine(
        head_to_head_summary,
        "head-to-head-corridor-continuity-uncertainty",
        "field",
    )
    babel = top_recommendation_row(recommendations, "babel")
    olsrv2 = top_recommendation_row(recommendations, "olsrv2")
    if (
        connected_low_loss is None
        or connected_high_loss is None
        or bridge_transition is None
        or corridor is None
        or partial_bridge is None
        or not concurrent_mixed_rows
        or comparison_concurrent_mixed is None
        or not head_to_head_connected_high_loss_rows
        or not head_to_head_bridge_transition_rows
        or not head_to_head_corridor_rows
        or field_connected_high_loss is None
        or field_bridge_transition is None
        or field_corridor_uncertainty is None
        or babel is None
        or olsrv2 is None
    ):
        return []

    concurrent_mixed = concurrent_mixed_rows[0]
    head_to_head_connected_high_loss = head_to_head_connected_high_loss_rows[0]
    head_to_head_bridge_transition = head_to_head_bridge_transition_rows[0]
    head_to_head_corridor = head_to_head_corridor_rows[0]
    return section_lines_formatted(
        "Part II Takeaways",
        connected_low_loss_engine=connected_low_loss["dominant_engine"] or "none",
        connected_high_loss_engine=connected_high_loss["dominant_engine"] or "none",
        comparison_concurrent_mixed_engine=comparison_concurrent_mixed["dominant_engine"] or "none",
        corridor_engine=corridor["dominant_engine"] or "none",
        partial_bridge_engine=partial_bridge["dominant_engine"] or "none",
        babel_config=babel["config_id"],
        olsrv2_config=olsrv2["config_id"],
        concurrent_mixed_engine_sets=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in concurrent_mixed_rows]
        ),
        concurrent_mixed_route_presence=concurrent_mixed[
            "route_present_total_window_permille_mean"
        ],
        mixed_connected_high_loss_engine=connected_high_loss["dominant_engine"] or "none",
        mixed_connected_high_loss_route_presence=connected_high_loss[
            "route_present_total_window_permille_mean"
        ],
        head_to_head_connected_high_loss_engines=_join_code_names(
            [
                row["comparison_engine_set"] or "none"
                for row in head_to_head_connected_high_loss_rows
            ]
        ),
        head_to_head_connected_high_loss_route_presence=head_to_head_connected_high_loss[
            "route_present_total_window_permille_mean"
        ],
        head_to_head_connected_high_loss_route_verb=(
            "reaches" if len(head_to_head_connected_high_loss_rows) == 1 else "reach"
        ),
        mixed_bridge_transition_engine=bridge_transition["dominant_engine"] or "none",
        mixed_bridge_transition_route_presence=bridge_transition[
            "route_present_total_window_permille_mean"
        ],
        head_to_head_bridge_transition_engines=_join_code_names(
            [
                row["comparison_engine_set"] or "none"
                for row in head_to_head_bridge_transition_rows
            ]
        ),
        head_to_head_bridge_transition_route_presence=head_to_head_bridge_transition[
            "route_present_total_window_permille_mean"
        ],
        head_to_head_bridge_transition_route_verb=(
            "reaches" if len(head_to_head_bridge_transition_rows) == 1 else "reach"
        ),
        field_connected_high_loss_route_presence=field_connected_high_loss[
            "route_present_total_window_permille_mean"
        ],
        field_bridge_transition_route_presence=field_bridge_transition[
            "route_present_total_window_permille_mean"
        ],
        field_corridor_uncertainty_route_presence=field_corridor_uncertainty[
            "route_present_total_window_permille_mean"
        ],
        corridor_best_engines=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in head_to_head_corridor_rows]
        ),
        corridor_best_route_presence=head_to_head_corridor[
            "route_present_total_window_permille_mean"
        ],
    )


def diffusion_takeaway_lines(
    diffusion_regime_engine_summary: pl.DataFrame,
    field_vs_best_diffusion_alternative: pl.DataFrame,
) -> list[str]:
    if diffusion_regime_engine_summary.is_empty() or field_vs_best_diffusion_alternative.is_empty():
        return []

    regime_rows = {
        row["diffusion_regime"]: row
        for row in diffusion_regime_engine_summary.iter_rows(named=True)
    }
    field_rows = {
        row["field_regime"]: row
        for row in field_vs_best_diffusion_alternative.iter_rows(named=True)
    }
    balanced = regime_rows.get("balanced")
    congestion = regime_rows.get("congestion")
    continuity = regime_rows.get("continuity")
    privacy = regime_rows.get("privacy")
    scarcity = regime_rows.get("scarcity")
    field_balanced = field_rows.get("balanced")
    field_congestion = field_rows.get("congestion")
    field_continuity = field_rows.get("continuity")
    field_privacy = field_rows.get("privacy")
    field_scarcity = field_rows.get("scarcity")
    if (
        balanced is None
        or congestion is None
        or continuity is None
        or privacy is None
        or scarcity is None
        or field_balanced is None
        or field_congestion is None
        or field_continuity is None
        or field_privacy is None
        or field_scarcity is None
    ):
        return []

    return section_lines_formatted(
        "Diffusion Takeaways",
        balanced_winner=balanced["config_id"],
        scarcity_winner=scarcity["config_id"],
        congestion_winner=congestion["config_id"],
        continuity_privacy_winners=_join_code_names(
            list(dict.fromkeys([continuity["config_id"], privacy["config_id"]]))
        ),
        continuity_privacy_verb=(
            "leads" if continuity["config_id"] == privacy["config_id"] else "lead"
        ),
        field_balanced_status=field_balanced["selection_status"],
        field_balanced_score_delta=f"{field_balanced['regime_score_delta']:.1f}",
        field_scarcity_score_delta=f"{field_scarcity['regime_score_delta']:.1f}",
        field_privacy_score_delta=f"{field_privacy['regime_score_delta']:.1f}",
        field_continuity_score_delta=f"{field_continuity['regime_score_delta']:.1f}",
        field_congestion_status=field_congestion["selection_status"],
    )


def large_population_takeaway_lines(
    large_population_route_summary: pl.DataFrame,
    large_population_diffusion_transitions: pl.DataFrame,
) -> list[str]:
    if large_population_route_summary.is_empty() or large_population_diffusion_transitions.is_empty():
        return []

    scaling_rows = _top_large_population_rows(large_population_route_summary, "diameter-fanout")
    bottleneck_rows = _top_large_population_rows(
        large_population_route_summary, "multi-bottleneck"
    )
    diameter_sensitive = (
        large_population_route_summary.filter(pl.col("topology_class") == "diameter-fanout")
        .sort(["small_to_high_route_delta", "comparison_engine_set"], descending=[False, False])
        .head(1)
    )
    bottleneck_fragile = (
        large_population_route_summary.filter(pl.col("topology_class") == "multi-bottleneck")
        .sort(["small_to_high_route_delta", "comparison_engine_set"], descending=[False, False])
        .head(1)
    )
    sparse_high = large_population_diffusion_transitions.filter(
        pl.col("family_id") == "diffusion-large-sparse-threshold-high"
    ).head(1)
    congestion_moderate = large_population_diffusion_transitions.filter(
        pl.col("family_id") == "diffusion-large-congestion-threshold-moderate"
    ).head(1)
    congestion_high = large_population_diffusion_transitions.filter(
        pl.col("family_id") == "diffusion-large-congestion-threshold-high"
    ).head(1)
    regional_high = large_population_diffusion_transitions.filter(
        pl.col("family_id") == "diffusion-large-regional-shift-high"
    ).head(1)
    core_periphery_field = _route_summary_row(
        large_population_route_summary,
        "diameter-fanout",
        "field",
    )
    core_periphery_scatter = _route_summary_row(
        large_population_route_summary,
        "diameter-fanout",
        "scatter",
    )
    multi_bottleneck_field = _route_summary_row(
        large_population_route_summary,
        "multi-bottleneck",
        "field",
    )
    multi_bottleneck_scatter = _route_summary_row(
        large_population_route_summary,
        "multi-bottleneck",
        "scatter",
    )
    multi_bottleneck_pathway = _route_summary_row(
        large_population_route_summary,
        "multi-bottleneck",
        "pathway",
    )
    multi_bottleneck_pathway_batman = _route_summary_row(
        large_population_route_summary,
        "multi-bottleneck",
        "pathway-batman-bellman",
    )
    if (
        not scaling_rows
        or not bottleneck_rows
        or diameter_sensitive.is_empty()
        or bottleneck_fragile.is_empty()
        or sparse_high.is_empty()
        or congestion_moderate.is_empty()
        or congestion_high.is_empty()
        or regional_high.is_empty()
        or core_periphery_field is None
        or core_periphery_scatter is None
        or multi_bottleneck_field is None
        or multi_bottleneck_scatter is None
        or multi_bottleneck_pathway is None
        or multi_bottleneck_pathway_batman is None
    ):
        return []

    scaling_row = scaling_rows[0]
    bottleneck_row = bottleneck_rows[0]
    diameter_row = diameter_sensitive.row(0, named=True)
    bottleneck_fragile_row = bottleneck_fragile.row(0, named=True)
    sparse_row = sparse_high.row(0, named=True)
    congestion_row = congestion_moderate.row(0, named=True)
    congestion_high_row = congestion_high.row(0, named=True)
    regional_row = regional_high.row(0, named=True)
    return section_lines_formatted(
        "Large-Population Takeaways",
        scaling_best_engines=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in scaling_rows]
        ),
        scaling_high_route=int(round(scaling_row["high_route_present"] or 0)),
        bottleneck_best_engines=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in bottleneck_rows]
        ),
        bottleneck_high_route=int(round(bottleneck_row["high_route_present"] or 0)),
        diameter_sensitive_engine=diameter_row["comparison_engine_set"],
        diameter_delta=int(round(diameter_row["small_to_high_route_delta"] or 0)),
        bottleneck_fragile_engine=bottleneck_fragile_row["comparison_engine_set"],
        bottleneck_delta=int(round(bottleneck_fragile_row["small_to_high_route_delta"] or 0)),
        core_periphery_scatter_route=int(round(core_periphery_scatter["high_route_present"] or 0)),
        core_periphery_field_route=int(round(core_periphery_field["high_route_present"] or 0)),
        multi_bottleneck_scatter_route=int(round(multi_bottleneck_scatter["high_route_present"] or 0)),
        multi_bottleneck_field_route=int(round(multi_bottleneck_field["high_route_present"] or 0)),
        multi_bottleneck_pathway_route=int(round(multi_bottleneck_pathway["high_route_present"] or 0)),
        multi_bottleneck_pathway_batman_route=int(round(multi_bottleneck_pathway_batman["high_route_present"] or 0)),
        core_periphery_field_blocker=core_periphery_field.get("high_field_promotion_blocker") or "none",
        core_periphery_field_resolution=core_periphery_field.get("high_field_commitment_resolution") or "none",
        core_periphery_field_selected_results=int(round(core_periphery_field.get("high_field_selected_result_rounds") or 0)),
        core_periphery_field_inadmissible=int(round(core_periphery_field.get("high_inadmissible_candidate_count") or 0)),
        multi_bottleneck_field_blocker=multi_bottleneck_field.get("high_field_promotion_blocker") or "none",
        multi_bottleneck_field_resolution=multi_bottleneck_field.get("high_field_commitment_resolution") or "none",
        multi_bottleneck_field_selected_results=int(round(multi_bottleneck_field.get("high_field_selected_result_rounds") or 0)),
        multi_bottleneck_field_inadmissible=int(round(multi_bottleneck_field.get("high_inadmissible_candidate_count") or 0)),
        sparse_viable=sparse_row.get("viable_config_id") or "none",
        sparse_explosive=sparse_row.get("explosive_config_id") or "none",
        congestion_viable=congestion_row.get("viable_config_id") or "none",
        congestion_collapse=congestion_row.get("collapse_config_id") or "none",
        congestion_high_states=congestion_high_row.get("observed_states") or "none",
        regional_states=regional_row.get("observed_states") or "none",
    )


def _top_rows(
    table: pl.DataFrame,
    *,
    filters: list[tuple[str, object]],
    sort_columns: list[str],
    descending: list[bool],
) -> list[dict]:
    subset = table
    for column, value in filters:
        subset = subset.filter(pl.col(column) == value)
    if subset.is_empty():
        return []
    available_sort = [
        (column, direction)
        for column, direction in zip(sort_columns, descending, strict=True)
        if column in subset.columns
    ]
    available_columns = [column for column, _ in available_sort]
    available_descending = [direction for _, direction in available_sort]
    if not available_columns:
        return list(subset.sort("comparison_engine_set").iter_rows(named=True))
    best = subset.sort(available_columns, descending=available_descending).head(1).row(0, named=True)
    tied_best_filters = [
        pl.col(column).is_null() if best[column] is None else pl.col(column) == best[column]
        for column in available_columns
    ]
    return list(
        subset.filter(pl.all_horizontal(tied_best_filters))
        .sort("comparison_engine_set")
        .iter_rows(named=True)
    )


def routing_fitness_takeaway_lines(
    routing_fitness_crossover_summary: pl.DataFrame,
    routing_fitness_multiflow_summary: pl.DataFrame,
    routing_fitness_stale_repair_summary: pl.DataFrame,
) -> list[str]:
    if (
        routing_fitness_crossover_summary.is_empty()
        or routing_fitness_multiflow_summary.is_empty()
        or routing_fitness_stale_repair_summary.is_empty()
    ):
        return []

    search_high_rows = _top_rows(
        routing_fitness_crossover_summary,
        filters=[("question", "search-burden"), ("band_label", "high")],
        sort_columns=[
            "route_present_total_window_permille_mean",
            "route_churn_count_mean",
        ],
        descending=[True, False],
    )
    maintenance_high_rows = _top_rows(
        routing_fitness_crossover_summary,
        filters=[("question", "maintenance-benefit"), ("band_label", "high")],
        sort_columns=[
            "route_present_total_window_permille_mean",
            "route_churn_count_mean",
        ],
        descending=[True, False],
    )
    shared_corridor_rows = _top_rows(
        routing_fitness_multiflow_summary,
        filters=[("family_label", "Shared corridor")],
        sort_columns=[
            "objective_route_presence_min_permille_mean",
            "objective_starvation_count_mean",
        ],
        descending=[True, False],
    )
    detour_choice_rows = _top_rows(
        routing_fitness_multiflow_summary,
        filters=[("family_label", "Detour choice")],
        sort_columns=[
            "objective_route_presence_min_permille_mean",
            "objective_starvation_count_mean",
        ],
        descending=[True, False],
    )
    stale_best_rows = _top_rows(
        routing_fitness_stale_repair_summary,
        filters=[("family_label", "Recovery window")],
        sort_columns=[
            "route_present_total_window_permille_mean",
            "stale_persistence_round_mean",
        ],
        descending=[True, False],
    )
    if (
        not search_high_rows
        or not maintenance_high_rows
        or not shared_corridor_rows
        or not detour_choice_rows
        or not stale_best_rows
    ):
        return []

    search_high = search_high_rows[0]
    maintenance_high = maintenance_high_rows[0]
    shared_corridor = shared_corridor_rows[0]
    detour_choice = detour_choice_rows[0]
    stale_best = stale_best_rows[0]
    worst_starvation = (
        routing_fitness_multiflow_summary.sort(
            [
                "objective_starvation_count_mean",
                "objective_route_presence_min_permille_mean",
                "comparison_engine_set",
            ],
            descending=[True, False, False],
        )
        .head(1)
        .row(0, named=True)
    )
    stale_sort_columns = ["stale_persistence_round_mean", "comparison_engine_set"]
    stale_sort_descending = [True, False]
    if "unrecovered_after_loss_count_mean" in routing_fitness_stale_repair_summary.columns:
        stale_sort_columns.insert(1, "unrecovered_after_loss_count_mean")
        stale_sort_descending.insert(1, True)
    worst_stale = (
        routing_fitness_stale_repair_summary.sort(
            stale_sort_columns,
            descending=stale_sort_descending,
        )
        .head(1)
        .row(0, named=True)
    )
    envelope = (
        "fit-for-purpose inside the tested search-plus-maintenance envelope"
        if (
            maintenance_high["comparison_engine_set"] == "pathway-batman-bellman"
            and float(stale_best["route_present_total_window_permille_mean"] or 0.0)
            >= 900.0
        )
        else "directionally supported, but still carrying one unresolved routing-risk regime"
    )
    return section_lines_formatted(
        "Routing-Fitness Takeaways",
        search_high_engines=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in search_high_rows]
        ),
        search_high_route=search_high["route_present_total_window_permille_mean"],
        maintenance_high_engines=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in maintenance_high_rows]
        ),
        maintenance_high_route=maintenance_high["route_present_total_window_permille_mean"],
        shared_corridor_engines=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in shared_corridor_rows]
        ),
        shared_corridor_min_route=shared_corridor["objective_route_presence_min_permille_mean"],
        detour_choice_engines=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in detour_choice_rows]
        ),
        detour_choice_min_route=detour_choice["objective_route_presence_min_permille_mean"],
        worst_starvation_family=worst_starvation["family_label"],
        worst_starvation_engine=worst_starvation["comparison_engine_set"] or "none",
        worst_starvation_value=f"{float(worst_starvation['objective_starvation_count_mean'] or 0.0):.1f}",
        stale_best_engines=_join_code_names(
            [row["comparison_engine_set"] or "none" for row in stale_best_rows]
        ),
        stale_best_persistence=f"{float(stale_best['stale_persistence_round_mean'] or 0.0):.1f}",
        stale_best_route=stale_best["route_present_total_window_permille_mean"],
        worst_stale_family=worst_stale["family_label"],
        worst_stale_engine=worst_stale["comparison_engine_set"] or "none",
        worst_stale_persistence=f"{float(worst_stale['stale_persistence_round_mean'] or 0.0):.1f}",
        worst_stale_route=worst_stale["route_present_total_window_permille_mean"],
        routing_fitness_envelope=envelope,
    )


def diffusion_field_posture_lines(diffusion_engine_comparison: pl.DataFrame) -> list[str]:
    if diffusion_engine_comparison.is_empty():
        return []
    field_rows = diffusion_engine_comparison.filter(
        pl.col("config_id").str.starts_with("field")
    )
    bridge = (
        field_rows.filter(pl.col("family_id") == "diffusion-bridge-drought")
        .sort("score", descending=True)
        .head(1)
    )
    energy = (
        field_rows.filter(pl.col("family_id") == "diffusion-energy-starved-relay")
        .sort("score", descending=True)
        .head(1)
    )
    congestion = (
        field_rows.filter(pl.col("family_id") == "diffusion-congestion-cascade")
        .sort("score", descending=True)
        .head(1)
    )
    if bridge.is_empty() or energy.is_empty() or congestion.is_empty():
        return []
    bridge_row = bridge.iter_rows(named=True).__next__()
    energy_row = energy.iter_rows(named=True).__next__()
    congestion_row = congestion.iter_rows(named=True).__next__()
    return section_lines_formatted(
        "Field Diffusion Posture",
        bridge_drought_posture=bridge_row.get("field_posture_mode") or "none",
        bridge_drought_transitions=bridge_row.get("field_posture_transition_count_mean") or 0,
        bridge_drought_protected_budget=bridge_row.get("field_protected_budget_used_mean") or 0,
        bridge_drought_bridge_use=bridge_row.get("field_protected_bridge_usage_count_mean") or 0,
        bridge_drought_bridge_opportunities=bridge_row.get("field_bridge_opportunity_count_mean")
        or 0,
        energy_starved_posture=energy_row.get("field_posture_mode") or "none",
        energy_starved_first_scarcity=energy_row.get("field_first_scarcity_transition_round_mean")
        if energy_row.get("field_first_scarcity_transition_round_mean") is not None
        else "-",
        energy_starved_expensive_suppressions=energy_row.get(
            "field_expensive_transport_suppression_count_mean"
        )
        or 0,
        congestion_posture=congestion_row.get("field_posture_mode") or "none",
        congestion_first_transition=congestion_row.get("field_first_congestion_transition_round_mean")
        if congestion_row.get("field_first_congestion_transition_round_mean") is not None
        else "-",
        congestion_cluster_seed_uses=congestion_row.get(
            "field_cluster_seed_usage_count_mean"
        )
        or 0,
        congestion_cluster_starvation=congestion_row.get(
            "field_cluster_coverage_starvation_count_mean"
        )
        or 0,
        congestion_redundant_suppressions=congestion_row.get(
            "field_redundant_forward_suppression_count_mean"
        )
        or 0,
        congestion_same_cluster_suppressions=congestion_row.get(
            "field_same_cluster_suppression_count_mean"
        )
        or 0,
    )


def pressure_findings_lines(aggregates: pl.DataFrame) -> list[str]:
    lines: list[str] = []
    batman_bellman_pressure = aggregates.filter(
        (pl.col("engine_family") == "batman-bellman")
        & pl.col("family_id").is_in(
            [
                "batman-bellman-decay-window-pressure",
                "batman-bellman-partition-recovery",
                "batman-bellman-asymmetry-relink-transition",
            ]
        )
    ).sort(["family_id", "batman_bellman_stale_after_ticks"])
    if not batman_bellman_pressure.is_empty():
        stability_values = batman_bellman_pressure["stability_total_mean"].to_list()
        if len(set(stability_values)) <= 2:
            lines.extend(section_lines("Pressure Findings Batman Plateau"))
        else:
            lines.extend(section_lines("Pressure Findings Batman Separation"))

    batman_classic_pressure = aggregates.filter(
        (pl.col("engine_family") == "batman-classic")
        & pl.col("family_id").is_in(
            [
                "batman-classic-decay-window-pressure",
                "batman-classic-partition-recovery",
                "batman-classic-asymmetry-relink-transition",
            ]
        )
    ).sort(["family_id", "batman_classic_stale_after_ticks"])
    if not batman_classic_pressure.is_empty():
        stability_values = batman_classic_pressure["stability_total_mean"].to_list()
        if len(set(stability_values)) <= 2:
            lines.extend(section_lines("Pressure Findings Batman Classic Plateau"))
        else:
            lines.extend(section_lines("Pressure Findings Batman Classic Separation"))

    babel_pressure = aggregates.filter(
        (pl.col("engine_family") == "babel")
        & pl.col("family_id").is_in(
            [
                "babel-decay-window-pressure",
                "babel-asymmetry-cost-penalty",
                "babel-partition-feasibility-recovery",
            ]
        )
    ).sort(["family_id", "babel_stale_after_ticks"])
    if not babel_pressure.is_empty():
        stability_values = babel_pressure["stability_total_mean"].to_list()
        if len(set(stability_values)) <= 2:
            lines.extend(section_lines("Pressure Findings Babel Plateau"))
        else:
            lines.extend(section_lines("Pressure Findings Babel Separation"))

    pathway_pressure = aggregates.filter(
        (pl.col("engine_family") == "pathway")
        & pl.col("family_id").is_in(
            [
                "pathway-search-budget-pressure",
                "pathway-high-fanout-budget-pressure",
                "pathway-bridge-failure-service",
            ]
        )
    ).sort(["family_id", "pathway_query_budget", "pathway_heuristic_mode"])
    if not pathway_pressure.is_empty():
        low = pathway_pressure.filter(pl.col("config_id") == "pathway-1-zero")
        if not low.is_empty():
            row = low.iter_rows(named=True).__next__()
            if row["activation_success_permille_mean"] == 0:
                lines.extend(section_lines("Pressure Findings Pathway Cliff"))
    field_pressure = aggregates.filter(
        (pl.col("engine_family") == "field")
        & pl.col("family_id").is_in(
            [
                "field-partial-observability-bridge",
                "field-reconfiguration-recovery",
                "field-asymmetric-envelope-shift",
                "field-uncertain-service-fanout",
                "field-bridge-anti-entropy-continuity",
                "field-bootstrap-upgrade-window",
            ]
        )
    ).sort(["family_id", "field_query_budget", "field_heuristic_mode"])
    if not field_pressure.is_empty():
        low = field_pressure.filter(pl.col("config_id") == "field-2-zero")
        if not low.is_empty():
            row = low.iter_rows(named=True).__next__()
            lines.extend(
                section_lines_formatted(
                    "Pressure Findings Field Plateau",
                    route_present=row["route_present_permille_mean"],
                    bootstrap_activation=row["field_bootstrap_activation_permille_mean"],
                    bootstrap_upgrade=row["field_bootstrap_upgrade_permille_mean"],
                )
            )
    return lines


def simulation_setup_lines() -> list[str]:
    return section_lines("Simulation Setup")


def methodology_lines() -> list[str]:
    return section_lines("Matrix Design")


def regime_assumption_lines() -> list[str]:
    return section_lines("Regime Assumptions")


def regime_characterization_lines() -> list[str]:
    return section_lines("Regime Characterization")


def batman_bellman_algorithm_lines() -> list[str]:
    return section_lines("BATMAN Bellman Algorithm")


def batman_classic_algorithm_lines() -> list[str]:
    return section_lines("BATMAN Classic Algorithm")


def babel_algorithm_lines() -> list[str]:
    return section_lines("Babel Algorithm")


def olsrv2_algorithm_lines() -> list[str]:
    return section_lines("OLSRv2 Algorithm")


def scatter_algorithm_lines() -> list[str]:
    return section_lines("Scatter Algorithm")


def pathway_algorithm_lines() -> list[str]:
    return section_lines("Pathway Algorithm")


def field_algorithm_lines() -> list[str]:
    return section_lines("Field Algorithm")


def approach_lines() -> list[str]:
    return section_lines("Analytical Approach")


def scoring_lines() -> list[str]:
    return section_lines("Recommendation Logic")


def executive_summary_lines(
    recommendations: pl.DataFrame,
    aggregates: pl.DataFrame,
    comparison_summary: pl.DataFrame,
) -> list[str]:
    return section_lines("Executive Summary Intro")


def engine_section_lines(
    recommendations: pl.DataFrame, aggregates: pl.DataFrame, engine_family: str
) -> list[str]:
    lines: list[str] = []
    row = top_recommendation_row(recommendations, engine_family)
    if row is None:
        if engine_family == "field":
            return section_lines("Engine Section Empty Field")
        return section_lines_formatted("Engine Section Empty Generic", engine_family=engine_family)
    lines.extend(
        section_lines_formatted(
            "Engine Section Recommended",
            config_id=row["config_id"],
            score=f"{row['mean_score']:.1f}",
            activation=f"{row['activation_success_mean']:.1f}",
            route_presence=f"{row['route_present_mean']:.1f}",
            max_stress=row["max_sustained_stress_score"],
        )
    )
    family_rows = aggregates.filter(pl.col("engine_family") == engine_family)
    if family_rows.is_empty():
        return lines
    if engine_family == "batman-bellman":
        pressure = family_rows.filter(
            pl.col("family_id").is_in(
                [
                    "batman-bellman-decay-window-pressure",
                    "batman-bellman-partition-recovery",
                    "batman-bellman-asymmetry-relink-transition",
                ]
            )
        )
        if not pressure.is_empty():
            stability_values = pressure["stability_total_mean"].to_list()
            if len(set(stability_values)) == 1:
                lines.extend(section_lines("Engine Section Batman Bellman Plateau"))
            else:
                best = pressure.sort(
                    ["stability_total_mean", "route_present_permille_mean"],
                    descending=[True, True],
                ).head(1)
                best_row = best.iter_rows(named=True).__next__()
                lines.extend(
                    section_lines_formatted(
                        "Engine Section Batman Bellman Best",
                        config_id=best_row["config_id"],
                        stability_total=best_row["stability_total_mean"],
                        route_presence=best_row["route_present_permille_mean"],
                    )
                )
        lines.extend(section_lines("Engine Section Batman Bellman Closing"))
    if engine_family == "batman-classic":
        pressure = family_rows.filter(
            pl.col("family_id").is_in(
                [
                    "batman-classic-decay-window-pressure",
                    "batman-classic-partition-recovery",
                    "batman-classic-asymmetry-relink-transition",
                ]
            )
        )
        if not pressure.is_empty():
            stability_values = pressure["stability_total_mean"].to_list()
            if len(set(stability_values)) == 1:
                lines.extend(section_lines("Engine Section Batman Classic Plateau"))
            else:
                best = pressure.sort(
                    ["stability_total_mean", "route_present_permille_mean"],
                    descending=[True, True],
                ).head(1)
                best_row = best.iter_rows(named=True).__next__()
                lines.extend(
                    section_lines_formatted(
                        "Engine Section Batman Classic Best",
                        config_id=best_row["config_id"],
                        stability_total=best_row["stability_total_mean"],
                        route_presence=best_row["route_present_permille_mean"],
                    )
                )
        lines.extend(section_lines("Engine Section Batman Classic Closing"))
    if engine_family == "babel":
        pressure = family_rows.filter(
            pl.col("family_id").is_in(
                [
                    "babel-decay-window-pressure",
                    "babel-asymmetry-cost-penalty",
                    "babel-partition-feasibility-recovery",
                ]
            )
        )
        if not pressure.is_empty():
            stability_values = pressure["stability_total_mean"].to_list()
            if len(set(stability_values)) == 1:
                lines.extend(section_lines("Engine Section Babel Plateau"))
            else:
                best = pressure.sort(
                    ["stability_total_mean", "route_present_permille_mean"],
                    descending=[True, True],
                ).head(1)
                best_row = best.iter_rows(named=True).__next__()
                lines.extend(
                    section_lines_formatted(
                        "Engine Section Babel Best",
                        config_id=best_row["config_id"],
                        stability_total=best_row["stability_total_mean"],
                        route_presence=best_row["route_present_permille_mean"],
                    )
                )
        lines.extend(section_lines("Engine Section Babel Closing"))
    if engine_family == "olsrv2":
        pressure = family_rows.filter(
            pl.col("family_id").is_in(
                [
                    "olsrv2-topology-propagation-latency",
                    "olsrv2-partition-recovery",
                    "olsrv2-mpr-flooding-stability",
                    "olsrv2-asymmetric-relink-transition",
                ]
            )
        )
        if not pressure.is_empty():
            stability_values = pressure["stability_total_mean"].to_list()
            if len(set(stability_values)) == 1:
                lines.extend(section_lines("Engine Section OLSRv2 Plateau"))
            else:
                best = pressure.sort(
                    ["stability_total_mean", "route_present_permille_mean"],
                    descending=[True, True],
                ).head(1)
                best_row = best.iter_rows(named=True).__next__()
                lines.extend(
                    section_lines_formatted(
                        "Engine Section OLSRv2 Best",
                        config_id=best_row["config_id"],
                        stability_total=best_row["stability_total_mean"],
                        route_presence=best_row["route_present_permille_mean"],
                    )
                )
        lines.extend(section_lines("Engine Section OLSRv2 Closing"))
    if engine_family == "pathway":
        pressure = family_rows.filter(
            pl.col("family_id").is_in(
                [
                    "pathway-search-budget-pressure",
                    "pathway-high-fanout-budget-pressure",
                    "pathway-bridge-failure-service",
                ]
            )
        )
        if not pressure.is_empty():
            low = pressure.filter(pl.col("config_id") == "pathway-1-zero")
            stable = pressure.filter(pl.col("config_id") != "pathway-1-zero")
            if not low.is_empty() and not stable.is_empty():
                low_row = low.iter_rows(named=True).__next__()
                stable_best = stable.sort(
                    ["route_present_permille_mean", "activation_success_permille_mean"],
                    descending=[True, True],
                ).head(1)
                stable_row = stable_best.iter_rows(named=True).__next__()
                lines.extend(
                    section_lines_formatted(
                        "Engine Section Pathway Cliff",
                        activation=low_row["activation_success_permille_mean"],
                    )
                )
                lines.extend(
                    section_lines_formatted(
                        "Engine Section Pathway Floor",
                        config_id=stable_row["config_id"],
                    )
                )
    if engine_family == "scatter":
        pressure = family_rows.filter(
            pl.col("family_id").is_in(
                [
                    "scatter-low-rate-transfer-threshold",
                    "scatter-stability-window-threshold",
                    "scatter-conservative-constrained-threshold",
                ]
            )
        )
        if not pressure.is_empty():
            best = pressure.sort(
                [
                    "scatter_handoff_rounds_mean",
                    "scatter_constrained_rounds_mean",
                    "scatter_bridging_rounds_mean",
                    "route_present_total_window_permille_mean",
                ],
                descending=[True, True, True, True],
            ).head(1)
            best_row = best.iter_rows(named=True).__next__()
            lines.extend(
                section_lines_formatted(
                    "Engine Section Scatter Best",
                    family_id=best_row["family_id"],
                    route_presence=best_row["route_present_total_window_permille_mean"],
                    handoff=best_row["scatter_handoff_rounds_mean"],
                    constrained=best_row["scatter_constrained_rounds_mean"],
                    bridging=best_row["scatter_bridging_rounds_mean"],
                )
            )
        lines.extend(section_lines("Engine Section Scatter Closing"))
    if engine_family == "field":
        pressure = family_rows.filter(
            pl.col("family_id").is_in(
                [
                    "field-partial-observability-bridge",
                    "field-reconfiguration-recovery",
                    "field-asymmetric-envelope-shift",
                    "field-uncertain-service-fanout",
                    "field-bridge-anti-entropy-continuity",
                    "field-bootstrap-upgrade-window",
                ]
            )
        )
        if not pressure.is_empty():
            best = pressure.sort(
                [
                    "route_present_permille_mean",
                    "field_continuation_shift_count_mean",
                    "field_search_reconfiguration_rounds_mean",
                ],
                descending=[True, False, False],
            ).head(1)
            best_row = best.iter_rows(named=True).__next__()
            lines.extend(
                section_lines_formatted(
                    "Engine Section Field Best",
                    config_id=best_row["config_id"],
                    route_presence=best_row["route_present_permille_mean"],
                    continuation_shifts=best_row["field_continuation_shift_count_mean"],
                )
            )
            lines.extend(
                section_lines_formatted(
                    "Engine Section Field Bootstrap",
                    activation=f"{row['field_bootstrap_activation_mean']:.1f}",
                    hold=f"{row['field_bootstrap_hold_mean']:.1f}",
                    narrow=f"{row['field_bootstrap_narrow_mean']:.1f}",
                    upgrade=f"{row['field_bootstrap_upgrade_mean']:.1f}",
                    withdrawal=f"{row['field_bootstrap_withdraw_mean']:.1f}",
                    degraded=f"{(row['field_degraded_steady_round_mean'] or 0):.1f}",
                    service=f"{(row['field_service_retention_carry_forward_mean'] or 0):.1f}",
                    shift=f"{(row['field_asymmetric_shift_success_mean'] or 0):.1f}",
                    commitment=row["field_commitment_resolution_mode"] or "none",
                    outcome=row["field_last_outcome_mode"] or "none",
                    band=row["field_continuity_band_mode"] or "none",
                    transition=row["field_last_continuity_transition_mode"] or "none",
                    decision=row["field_last_promotion_decision_mode"] or "none",
                    blocker=row["field_last_promotion_blocker_mode"] or "none",
                )
            )
            lines.extend(section_lines("Engine Section Field Tied"))
        lines.extend(section_lines("Engine Section Field Replay"))
        lines.extend(section_lines("Engine Section Field Families"))
        lines.extend(section_lines("Engine Section Field Diagnosis"))
    return lines


def recommendation_rationale_lines(
    recommendations: pl.DataFrame, aggregates: pl.DataFrame, engine_family: str
) -> list[str]:
    rows = top_recommendation_rows(recommendations, engine_family, 3)
    if not rows:
        if engine_family == "field":
            return section_lines("Recommendation Rationale Empty Field")
        return section_lines_formatted(
            "Recommendation Rationale Empty Generic", engine_family=engine_family
        )
    top = rows[0]
    lines = section_lines_formatted(
        "Recommendation Rationale Primary",
        config_id=top["config_id"],
        score=f"{top['mean_score']:.1f}",
        activation=f"{top['activation_success_mean']:.1f}",
        route_presence=f"{top['route_present_mean']:.1f}",
        max_stress=top["max_sustained_stress_score"],
    )
    if len(rows) > 1:
        runner_up = rows[1]
        score_gap = float(top["mean_score"]) - float(runner_up["mean_score"])
        lines.extend(
            section_lines_formatted(
                "Recommendation Rationale Runner Up",
                config_id=runner_up["config_id"],
                score_gap=f"{score_gap:.1f}",
            )
        )
        if score_gap < 10:
            lines.extend(section_lines("Recommendation Rationale Small Gap"))
        else:
            lines.extend(section_lines("Recommendation Rationale Large Gap"))
    if engine_family == "batman-bellman":
        lines.extend(section_lines("Recommendation Rationale Batman Bellman 1"))
        lines.extend(section_lines("Recommendation Rationale Batman Bellman 2"))
    if engine_family == "batman-classic":
        lines.extend(section_lines("Recommendation Rationale Batman Classic 1"))
        lines.extend(section_lines("Recommendation Rationale Batman Classic 2"))
    if engine_family == "babel":
        lines.extend(section_lines("Recommendation Rationale Babel 1"))
        lines.extend(section_lines("Recommendation Rationale Babel 2"))
    if engine_family == "olsrv2":
        lines.extend(section_lines("Recommendation Rationale OLSRv2 1"))
        lines.extend(section_lines("Recommendation Rationale OLSRv2 2"))
    if engine_family == "pathway":
        lines.extend(section_lines("Recommendation Rationale Pathway 1"))
        lines.extend(section_lines("Recommendation Rationale Pathway 2"))
    if engine_family == "scatter":
        lines.extend(
            section_lines_formatted(
                "Recommendation Rationale Scatter 1",
                handoff=f"{(top['scatter_handoff_mean'] or 0):.1f}",
                constrained=f"{(top['scatter_constrained_mean'] or 0):.1f}",
                bridging=f"{(top['scatter_bridging_mean'] or 0):.1f}",
            )
        )
        lines.extend(
            section_lines_formatted(
                "Recommendation Rationale Scatter 2",
                retained=f"{(top['scatter_retained_peak_mean'] or 0):.1f}",
                delivered=f"{(top['scatter_delivered_peak_mean'] or 0):.1f}",
            )
        )
    if engine_family == "field":
        lines.extend(section_lines("Recommendation Rationale Field 1"))
        lines.extend(
            section_lines_formatted(
                "Recommendation Rationale Field 2",
                config_id=top["config_id"],
                activation=f"{top['field_bootstrap_activation_mean']:.1f}",
                hold=f"{top['field_bootstrap_hold_mean']:.1f}",
                narrow=f"{top['field_bootstrap_narrow_mean']:.1f}",
                upgrade=f"{top['field_bootstrap_upgrade_mean']:.1f}",
                withdrawal=f"{top['field_bootstrap_withdraw_mean']:.1f}",
                degraded=f"{(top['field_degraded_steady_round_mean'] or 0):.1f}",
                service=f"{(top['field_service_retention_carry_forward_mean'] or 0):.1f}",
                shift=f"{(top['field_asymmetric_shift_success_mean'] or 0):.1f}",
                commitment=top["field_commitment_resolution_mode"] or "none",
                outcome=top["field_last_outcome_mode"] or "none",
                band=top["field_continuity_band_mode"] or "none",
                transition=top["field_last_continuity_transition_mode"] or "none",
                decision=top["field_last_promotion_decision_mode"] or "none",
                blocker=top["field_last_promotion_blocker_mode"] or "none",
            )
        )
        lines.extend(section_lines("Recommendation Rationale Field 3"))
        lines.extend(section_lines("Recommendation Rationale Field 4"))
    return lines


def limitations_lines() -> list[str]:
    return section_lines("Limitations And Next Steps")


def profile_recommendation_lines(profile_recommendations: pl.DataFrame) -> list[str]:
    if profile_recommendations.is_empty():
        return section_lines("Profile Recommendation Logic Empty")
    return section_lines("Profile Recommendation Logic")
