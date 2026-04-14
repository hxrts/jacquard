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
    asset_id: str
    lines: list[str]


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
    return list(sections[key])


def section_lines_formatted(section: str, **kwargs: object) -> list[str]:
    lines = section_lines(section)
    return [line.format(**kwargs) if line else "" for line in lines]


def asset_block(section: str, expected_kind: str | None = None) -> AssetBlock:
    lines = section_lines(section)
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
    return AssetBlock(kind=kind, asset_id=match.group("asset_id"), lines=lines[1:])


def comparison_findings_lines(comparison_summary: pl.DataFrame) -> list[str]:
    lines: list[str] = []
    for family_id in comparison_summary["family_id"].unique().sort().to_list():
        family = (
            comparison_summary.filter(pl.col("family_id") == family_id)
            .sort("route_present_permille_mean", descending=True)
            .head(1)
        )
        if family.is_empty():
            continue
        row = family.iter_rows(named=True).__next__()
        dominant = row["dominant_engine"] if row["dominant_engine"] is not None else "none"
        lines.append(
            f"`{family_id}`: dominant_engine={dominant}, "
            f"activation={row['activation_success_permille_mean']}, "
            f"route_presence={row['route_present_permille_mean']}"
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
                ["route_present_permille_mean", "activation_success_permille_mean"],
                descending=[True, True],
            )
            .head(1)
        )
        if family.is_empty():
            continue
        row = family.iter_rows(named=True).__next__()
        lines.append(
            f"`{family_id}`: best engine set=`{row['comparison_engine_set'] or 'none'}`, activation={row['activation_success_permille_mean']} permille, route presence={row['route_present_permille_mean']} permille."
        )
    return lines


def head_to_head_regime_lines() -> list[str]:
    return section_lines("Head-To-Head Regimes")


def head_to_head_takeaway_lines(head_to_head_summary: pl.DataFrame) -> list[str]:
    if head_to_head_summary.is_empty():
        return []

    rows = {
        row["family_id"]: row
        for row in (
            head_to_head_summary.sort(
                ["family_id", "route_present_permille_mean", "activation_success_permille_mean"],
                descending=[False, True, True],
            ).iter_rows(named=True)
        )
    }

    connected_low_loss = rows.get("head-to-head-connected-low-loss")
    connected_high_loss = rows.get("head-to-head-connected-high-loss")
    bridge_transition = rows.get("head-to-head-bridge-transition")
    concurrent_mixed = rows.get("head-to-head-concurrent-mixed")
    corridor_uncertainty = rows.get("head-to-head-corridor-continuity-uncertainty")
    partial_bridge = rows.get("head-to-head-partial-observability-bridge")
    if (
        connected_low_loss is None
        or connected_high_loss is None
        or bridge_transition is None
        or concurrent_mixed is None
        or corridor_uncertainty is None
        or partial_bridge is None
    ):
        return []

    return section_lines_formatted(
        "Head-To-Head Takeaways",
        connected_high_loss_engine_set=connected_high_loss["comparison_engine_set"] or "none",
        connected_high_loss_route_presence=connected_high_loss["route_present_permille_mean"],
        bridge_transition_engine_set=bridge_transition["comparison_engine_set"] or "none",
        bridge_transition_route_presence=bridge_transition["route_present_permille_mean"],
        concurrent_mixed_engine_set=concurrent_mixed["comparison_engine_set"] or "none",
        concurrent_mixed_route_presence=concurrent_mixed["route_present_permille_mean"],
        corridor_uncertainty_route_presence=corridor_uncertainty["route_present_permille_mean"],
        partial_bridge_route_presence=partial_bridge["route_present_permille_mean"],
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
    head_to_head_rows = {
        row["family_id"]: row
        for row in (
            head_to_head_summary.sort(
                ["family_id", "route_present_permille_mean", "activation_success_permille_mean"],
                descending=[False, True, True],
            ).iter_rows(named=True)
        )
    }

    connected_low_loss = comparison_rows.get("comparison-connected-low-loss")
    corridor = comparison_rows.get("comparison-corridor-continuity-uncertainty")
    partial_bridge = comparison_rows.get("comparison-partial-observability-bridge")
    concurrent_mixed = head_to_head_rows.get("head-to-head-concurrent-mixed")
    corridor_uncertainty = head_to_head_rows.get("head-to-head-corridor-continuity-uncertainty")
    babel = top_recommendation_row(recommendations, "babel")
    olsrv2 = top_recommendation_row(recommendations, "olsrv2")
    if (
        connected_low_loss is None
        or corridor is None
        or partial_bridge is None
        or concurrent_mixed is None
        or corridor_uncertainty is None
        or babel is None
        or olsrv2 is None
    ):
        return []

    return section_lines_formatted(
        "Part II Takeaways",
        connected_low_loss_engine=connected_low_loss["dominant_engine"] or "none",
        corridor_engine=corridor["dominant_engine"] or "none",
        partial_bridge_engine=partial_bridge["dominant_engine"] or "none",
        babel_config=babel["config_id"],
        olsrv2_config=olsrv2["config_id"],
        concurrent_mixed_engine_set=concurrent_mixed["comparison_engine_set"] or "none",
        concurrent_mixed_route_presence=concurrent_mixed["route_present_permille_mean"],
        corridor_uncertainty_route_presence=corridor_uncertainty["route_present_permille_mean"],
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
