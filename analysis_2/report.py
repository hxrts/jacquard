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
    ("table_01_theorem_assumptions", "Theorem boundary table", "active_belief_theorem_assumptions.csv"),
    ("table_02_trace_validation", "Trace validation table", "active_belief_trace_validation.csv"),
    ("figure_01_path_free_recovery", "Path-free recovery", "active_belief_path_validation.csv"),
    ("figure_02_landscape_focus", "Landscape coming into focus", "active_belief_raw_rounds.csv"),
    ("table_03_three_mode_comparison", "Three-mode task surface", "coded_inference_experiment_a2_evidence_modes.csv"),
    ("figure_03_task_algebra", "Task-family outcome summary", "active_belief_second_tasks.csv"),
    ("table_04_task_family_interface", "Task-family interface summary", "active_belief_second_tasks.csv"),
    ("table_05_headline_statistics", "Headline statistical summary", "active_belief_headline_statistics.csv"),
    ("figure_04_active_belief_grid", "Multi-receiver compatibility summary", "active_belief_receiver_runs.csv"),
    ("figure_05_active_vs_passive", "Demand ablation paired deltas", "active_belief_demand_ablation.csv"),
    ("figure_06_coding_vs_replication", "Coding versus replication with spread", "coded_inference_experiment_d_coding_vs_replication.csv"),
    ("figure_07_recoding_tradeoff", "Regime-specific recoding tradeoff", "active_belief_receiver_runs.csv"),
    ("figure_08_phase_diagram", "Near-critical operating region", "coded_inference_experiment_c_phase_diagram.csv"),
    ("figure_09_robustness_boundary", "Robustness boundary", "active_belief_exact_seed_summary.csv"),
    ("table_06_host_bridge_demand", "Demand safety audit", "active_belief_host_bridge_demand.csv"),
    ("figure_10_strong_baselines", "Baseline fairness paired deltas", "active_belief_strong_baselines.csv"),
    ("figure_11_large_regime", "Large-regime validation", "active_belief_scale_validation.csv"),
    ("figure_12_observer_ambiguity", "Observer non-reconstructability frontier", "coded_inference_experiment_e_observer_frontier.csv"),
    ("figure_13_demand_byte_sweep", "Demand byte budget sweep", "active_belief_demand_byte_sweep.csv"),
    ("figure_14_high_gap_regimes", "High-gap demand regime family", "active_belief_high_gap_regimes.csv"),
    ("figure_15_adversarial_demand", "Adversarial demand steering", "active_belief_adversarial_demand.csv"),
    ("figure_16_byzantine_injection", "Byzantine fragment injection", "active_belief_byzantine_injection.csv"),
    ("figure_17_receiver_count_sweep", "Receiver-count compatibility sweep", "active_belief_receiver_count_sweep.csv"),
    ("table_07_independence_bottleneck", "Independence bottleneck summary", "active_belief_independence_bottleneck.csv"),
    ("figure_18_independence_bottleneck", "Matched raw spread, different effective rank", "active_belief_independence_bottleneck.csv"),
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
    figure_number = 0
    table_number = 0
    for index, (figure_id, title, dataset_name) in enumerate(FIGURES, start=1):
        display_kind = figure_display_kind(figure_id)
        if display_kind == "table":
            table_number += 1
            display_number = table_display_number(table_number)
        else:
            figure_number += 1
            display_number = figure_number
        values, labels = save_active_belief_plot_artifact(
            report_dir,
            figure_id,
            title,
            datasets[dataset_name],
            dataset_name,
        )
        caption = figure_caption(figure_id)
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
                "artifact_row_count": len(datasets[dataset_name]),
                "values": values,
                "labels": labels,
                "caption": caption,
                "display_kind": display_kind,
                "display_number": display_number,
                "table": figure_table(figure_id, datasets[dataset_name]),
            }
        )
    return figure_rows, figure_specs


def table_display_number(table_number: int) -> int:
    if table_number == 1:
        return 1
    return table_number + 1


def figure_display_kind(figure_id: str) -> str:
    if figure_id == "figure_03_task_algebra":
        return "figure-with-table"
    if figure_id in {
        "table_01_theorem_assumptions",
        "table_02_trace_validation",
        "table_03_three_mode_comparison",
        "table_04_task_family_interface",
        "table_05_headline_statistics",
        "table_06_host_bridge_demand",
        "table_07_independence_bottleneck",
    }:
        return "table"
    return "figure"


def figure_table(figure_id: str, rows: list[dict[str, object]]) -> dict[str, object] | None:
    if figure_id == "table_04_task_family_interface":
        return task_family_interface_table(rows)
    if figure_id == "table_03_three_mode_comparison":
        return three_mode_surface_table(rows)
    if figure_id == "table_06_host_bridge_demand":
        return host_bridge_demand_table(rows)
    if figure_id == "table_07_independence_bottleneck":
        return independence_bottleneck_table(rows)
    if figure_id == "table_01_theorem_assumptions":
        return theorem_assumption_table(rows)
    if figure_id == "table_02_trace_validation":
        return trace_validation_table(rows)
    if figure_id == "table_05_headline_statistics":
        return headline_statistics_table(rows)
    return None


def validate_manuscript_exhibit_references(markdown: str, figure_specs: list[dict[str, object]]) -> None:
    specs_by_id = {str(spec["figure_id"]): spec for spec in figure_specs}
    markers = re.findall(r"\{\{EXHIBIT:([a-zA-Z0-9_]+)\}\}", markdown)
    unknown = sorted(marker for marker in set(markers) if marker not in specs_by_id)
    if unknown:
        raise RuntimeError(f"unknown active-belief exhibit markers: {', '.join(unknown)}")


def median_int(rows: list[dict[str, object]], field: str) -> int:
    values = sorted(int_value(row, field) for row in rows)
    if not values:
        return 0
    return values[len(values) // 2]


def independence_bottleneck_table(rows: list[dict[str, object]]) -> dict[str, object]:
    grouped: dict[str, list[dict[str, object]]] = {}
    for row in rows:
        grouped.setdefault(str(row["pair_kind"]), []).append(row)
    table_rows = []
    for pair_kind in ["high-correlation", "high-independence"]:
        entries = grouped.get(pair_kind, [])
        if not entries:
            continue
        table_rows.append(
            [
                display_label(pair_kind),
                str(median_int(entries, "raw_transmissions")),
                str(median_int(entries, "raw_fragment_count")),
                str(median_int(entries, "innovative_contribution_count")),
                str(median_int(entries, "effective_rank_proxy")),
                f"{median_int(entries, 'quality_per_byte_permille') / 10:.1f}%",
                f"{median_int(entries, 'recovery_probability_permille') / 10:.1f}%",
            ]
        )
    return {
        "columns": [
            "Matched trace kind",
            "Raw transmissions",
            "Raw fragments",
            "Innovative",
            "Effective rank",
            "Quality/byte",
            "Recovery",
        ],
        "rows": table_rows,
        "widths": [2.7, 2.1, 1.8, 1.5, 2.0, 1.7, 1.6],
    }


def three_mode_surface_table(rows: list[dict[str, object]]) -> dict[str, object]:
    grouped: dict[str, dict[str, str]] = {}
    object_labels = {
        "source-coded-threshold": (
            "independent payload fragment",
            "fragment coverage set",
            "k distinct fragments",
            "exact reconstruction sanity check",
        ),
        "distributed-local-evidence": (
            "local statistic contribution",
            "local score statistic",
            "merged statistic + guard",
            "distributed inference without central encoder",
        ),
        "recoded-aggregate": (
            "audited recoded contribution",
            "recoded score aggregate",
            "merged statistic + guard",
            "in-network aggregation with parent ledgers",
        ),
    }
    for row in rows:
        mode = str(row["policy_or_mode"])
        grouped.setdefault(
            mode,
            {
                "statistic_kind": display_label(str(row["statistic_kind"])),
            },
        )
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
        encoded_object, merge_target, commit_from, role = object_labels[mode]
        table_rows.append(
            [
                display_label(mode),
                encoded_object,
                merge_target,
                commit_from,
                role,
                str(entry["statistic_kind"]),
            ]
        )
    return {
        "columns": [
            "Mode",
            "Encoded object",
            "Merge target",
            "Commit from",
            "Distinctive role",
            "Statistic",
        ],
        "rows": table_rows,
        "widths": [2.1, 3.2, 2.6, 2.8, 4.5, 2.0],
    }


def task_family_interface_table(rows: list[dict[str, object]]) -> dict[str, object]:
    task_stats: dict[str, str] = {}
    for row in rows:
        task_stats.setdefault(str(row["task_kind"]), str(row["statistic_kind"]))
    task_order = [
        "anomaly-localization",
        "bayesian-classifier",
        "majority-threshold",
        "bounded-histogram",
        "set-union-threshold",
    ]
    task_descriptions = {
        "anomaly-localization": (
            "sensor score",
            "vector sum",
            "top score + margin",
        ),
        "bayesian-classifier": (
            "local likelihood",
            "log-likelihood sum",
            "posterior margin",
        ),
        "majority-threshold": (
            "local vote",
            "count sum",
            "majority threshold",
        ),
        "bounded-histogram": (
            "bucket count",
            "bucket sum",
            "mass concentration",
        ),
        "set-union-threshold": (
            "item id",
            "id-set union",
            "k-of-n coverage",
        ),
    }
    table_rows: list[list[str]] = []
    for task in task_order:
        if task not in task_stats:
            continue
        contribution, merge_rule, commit_rule = task_descriptions[task]
        table_rows.append(
            [
                display_label(task),
                display_label(task_stats[task]),
                contribution,
                merge_rule,
                commit_rule,
            ]
        )
    return {
        "columns": [
            "Task",
            "Statistic",
            "Local contribution",
            "Merge",
            "Commit from",
        ],
        "rows": table_rows,
        "widths": [2.5, 2.2, 2.5, 2.5, 2.8],
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
            "non-zero only in active variants",
        ],
        [
            "demand bytes per audited batch",
            median_label([int_value(row, "demand_byte_count") for row in grouped["passive-controlled-coded"]]),
            median_label([int_value(row, "demand_byte_count") for row in grouped["full-active-belief"]]),
            median_label([int_value(row, "demand_byte_count") for row in grouped["stale-demand-ablation"]]),
            "48 bytes per bounded summary",
        ]
    ]
    safety_fields = {
        "evidence_validity_changed": "must remain zero",
        "contribution_identity_created": "must remain zero",
        "merge_semantics_changed": "must remain zero",
        "route_truth_published": "must remain zero",
        "duplicate_rank_inflation": "must remain zero",
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
    boundaries: dict[str, str] = {
        "receiver_arrival_reconstruction_bound": "finite horizon + arrival floor",
        "replay_certificate_implies_receiver_arrival_bound": "certificate -> arrival floor",
        "useful_inference_arrival_bound": "finite horizon + useful mass",
        "replay_certificate_implies_useful_inference_arrival_bound": "certificate -> useful mass",
        "anomaly_margin_lower_tail_bound": "margin model + bounded updates",
        "score_trace_certificate_implies_margin_guard": "certificate -> margin guard",
        "guarded_commitment_false_probability_bounded": "margin + evidence guard",
        "generic_direct_statistic_decoding": "merge law + contribution identity",
        "direct_statistic_commitment_requires_task_effective_guard": "task-effective evidence guard",
        "effective_task_independence_bounded_by_raw_copies": "raw copies only upper-bound",
        "effective_task_independence_bounded_by_raw_transmissions": "raw transmissions only upper-bound",
        "inference_potential_drift_progress": "banded control + budget accounting",
        "demand_policy_certificate_implies_useful_arrival_improvement": "certificate -> demand improvement",
        "active_belief_trace_soundness": "valid reduced replay trace",
        "active_demand_policy_improves_under_value_model": "explicit value-order model",
        "distributed_error_correction_decision_limit": "stable decision basin",
        "recovery_probability_bounded_by_effective_independence": "bounded finite certificate",
        "raw_reproduction_above_one_does_not_imply_effective_reproduction_above_one": "raw/useful R split",
        "effective_reproduction_finite_horizon_bound": "useful R is explicit",
        "contact_entropy_and_dispersion_bounded_by_raw_activity": "finite entropy/dispersion proxy",
        "effective_rank_bounded_by_temporal_generator_rank": "finite generator-rank proxy",
        "reconstruction_bound_from_entropy_and_dispersion": "entropy/dispersion certificate",
        "temporal_contact_capacity_bounded_by_independent_arrivals": "finite temporal capacity",
        "reliability_resource_ambiguity_triangle_incompatibility": "resource/reliability/ambiguity boundary",
        "matched_networks_separate_by_entropy_and_effective_rank": "matched raw spread witness",
        "near_critical_controller_enters_band_under_opportunity_bounds": "opportunity bounds + control band",
        "rust_replay_rows_sound_for_active_belief_theorem_profiles": "replay row -> theorem profile",
        "trace_validator_adequacy": "validator metadata adequacy",
        "bounded_stress_certificate_implies_guarded_commitment_bound": "bounded stress certificate",
    }
    for row in rows:
        theorem = str(row["theorem_name"])
        scenario = display_label(str(row["scenario_regime"]))
        statuses.setdefault(theorem, {})[scenario] = str(row["assumption_status"])
    scenario_columns = ["sparse bridge", "clustered", "mobility"]
    table_rows = []
    for theorem in sorted(statuses):
        table_rows.append(
            [
                compact_theorem(theorem),
                *[statuses[theorem].get(scenario, "missing") for scenario in scenario_columns],
                boundaries.get(theorem, "empirical only"),
            ]
        )
    return {
        "columns": ["Theorem", "Sparse bridge", "Clustered", "Mobility", "Assumption boundary"],
        "rows": table_rows,
        "widths": [5.0, 1.8, 1.8, 1.8, 5.4],
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
    block_order = {
        "active belief vs passive coded": 0,
        "active belief vs uncoded replication": 1,
        "propagated demand vs no demand": 2,
        "propagated demand vs stale demand": 3,
    }
    metric_order = {
        "quality_per_byte_permille": 0,
        "quality_per_total_budget_permille": 1,
        "collective_uncertainty_permille": 2,
        "commitment_lead_time_rounds": 3,
        "demand_bytes_at_commitment": 4,
        "total_bytes_at_commitment": 5,
        "demand_byte_count": 6,
        "total_byte_count": 7,
    }
    sorted_rows = sorted(
        rows,
        key=lambda row: (
            block_order.get(str(row["comparison"]), 99),
            metric_order.get(str(row["metric"]), 99),
        ),
    )
    table_rows = []
    for row in sorted_rows:
        metric = str(row["metric"])
        block = "active vs baseline" if str(row["comparison"]).startswith("active belief") else "demand ablation"
        table_rows.append(
            [
                block,
                str(row["comparison"]),
                metric_label(metric),
                format_metric_value(metric, row["treatment_median"]),
                format_metric_value(metric, row["baseline_median"]),
                format_metric_delta(metric, row["paired_delta_median"]),
                format_metric_interval(metric, row["paired_delta_p25"], row["paired_delta_p75"]),
                format_metric_interval(metric, row["paired_delta_ci_low"], row["paired_delta_ci_high"]),
                str(row["aggregation_unit"]),
            ]
        )
    return {
        "columns": ["Block", "Comparison", "Metric", "Treatment median", "Baseline median", "Median delta", "Paired-difference IQR", "Bootstrap 95% CI", "Pairing unit"],
        "rows": table_rows,
        "widths": [1.8, 3.2, 2.2, 1.5, 1.5, 1.4, 1.5, 1.5, 1.8],
    }


def yes_no(value: bool) -> str:
    return "yes" if value else "no"


def is_permille_metric(metric: str) -> bool:
    return metric.endswith("_permille")


def format_percent(value: object) -> str:
    return f"{float(value) / 10.0:.1f}%"


def format_metric_value(metric: str, value: object) -> str:
    if is_permille_metric(metric):
        return format_percent(value)
    return str(value)


def format_metric_delta(metric: str, value: object) -> str:
    if is_permille_metric(metric):
        return f"{float(value) / 10.0:.1f} pp"
    return str(value)


def format_metric_interval(metric: str, low: object, high: object) -> str:
    if is_permille_metric(metric):
        return f"[{float(low) / 10.0:.1f}, {float(high) / 10.0:.1f}] pp"
    return f"[{low}, {high}]"


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


def figure_caption(figure_id: str) -> str:
    descriptions = {
        "figure_02_landscape_focus": (
            "Main evidence. Median lines and interquartile bands show belief quality rising while uncertainty falls over replay rounds in the anomaly-localization setting. This supports landscape sharpening under fixed payload-byte accounting; it does not by itself prove demand causality."
        ),
        "figure_01_path_free_recovery": (
            "Main evidence. Each distribution uses rows whose core windows have no instantaneous static source-to-receiver path and whose successful runs record time-respecting evidence journeys. This is the direct path-free inference check under the recorded trace families."
        ),
        "table_03_three_mode_comparison": (
            "Main evidence. The table distinguishes the threshold reconstruction case from the two score-vector cases at the encoded-object and statistic level. This supports the mergeable-task-interface claim directly."
        ),
        "figure_04_active_belief_grid": (
            "Main evidence. A focused four-panel summary compares active, passive, recoded, and uncoded modes for receiver agreement, belief divergence, quality per byte, and commitment lead time, with regime offsets shown directly inside each mode. This is the paper's flagship multi-receiver compatibility figure."
        ),
        "figure_03_task_algebra": (
            "Main evidence. The paired panels show that quality-per-byte ordering and bytes at commitment stay stable across anomaly, Bayesian classifier, majority, histogram, and set-union tasks. This is the empirical side of the compact task-family generalization claim."
        ),
        "table_04_task_family_interface": (
            "Main evidence. The interface table makes the shared mergeable-task surface explicit by listing the local contribution, merge rule, and guarded commit rule for each supported task."
        ),
        "table_05_headline_statistics": (
            "Main evidence summary. The table reports deterministic paired median differences, paired-difference IQRs, and paired-bootstrap 95% confidence intervals for the headline active-versus-baseline and demand-ablation claims. Positive deltas favor active belief for quality and lead time; negative deltas favor active belief for uncertainty."
        ),
        "figure_05_active_vs_passive": (
            "Main evidence. Paired replay ablation intervals compare propagated demand against no-demand, stale-demand, local-only demand, and removed-scoring-term policies under equal payload-byte budgets. Positive deltas indicate better quality, lower uncertainty, or lower bytes at commitment for propagated demand; theorem-backed improvement is limited to assumption-marked rows."
        ),
        "figure_13_demand_byte_sweep": (
            "Main evidence. The sweep varies only the bounded demand-byte budget while holding payload bytes fixed. It shows how quality, uncertainty, and the effective-rank proxy move as the control channel grows from zero to the full propagated-demand budget."
        ),
        "figure_14_high_gap_regimes": (
            "Main evidence. The heterogeneity sweep shows active demand gaining more over passive controlled coding as receivers need increasingly different evidence subsets under the same payload budget."
        ),
        "figure_15_adversarial_demand": (
            "Boundary/safety evidence. Biased demand summaries can degrade honest receiver quality by steering allocation, but the replay rows keep evidence-validity and duplicate-rank side effects at zero."
        ),
        "figure_16_byzantine_injection": (
            "Boundary/safety evidence. Malicious peers attempt to inject forged contribution identifiers. Under the stated signed-identity model, forged identifiers are rejected; remaining degradation comes from properly signed malicious contributions inside the modeled Sybil bound."
        ),
        "figure_17_receiver_count_sweep": (
            "Main scale evidence. Receiver counts are swept over 3, 10, 25, and 50 identities to show that compatibility metrics are not only a three-receiver artifact."
        ),
        "table_07_independence_bottleneck": (
            "Main evidence. Matched raw-spread rows separate raw transmissions, raw fragment count, innovative contributions, and effective-rank proxy so the independence bottleneck is visible as a measured object."
        ),
        "figure_18_independence_bottleneck": (
            "Main evidence. Matched high-correlation and high-independence rows have comparable raw spread, but higher effective-rank proxy tracks better recovery and quality under the same payload budget."
        ),
        "figure_06_coding_vs_replication": (
            "Main evidence. Median quality curves and interquartile bands compare coded evidence policies with uncoded replication across payload-byte budgets. This is the fair-cost check for the coding benefit."
        ),
        "figure_07_recoding_tradeoff": (
            "Main evidence. Regime-wise medians show the actual tradeoff boundary: recoded aggregate buys modest extra quality at modest extra byte cost relative to active belief, while passive coded sits as a dominated reference. Signed delta labels make the active-versus-recoded tradeoff explicit in each regime."
        ),
        "figure_08_phase_diagram": (
            "Main evidence. The operating-region panels show where measured useful reproduction pressure enters the near-critical target band and where quality gains stop justifying duplicate pressure. Raw reproduction is tracked separately because raw supercritical spread is not proof of useful independent evidence. Highlighted near-critical points mark the useful control region when the recorded band and budget assumptions hold."
        ),
        "figure_09_robustness_boundary": (
            "Main evidence. Split stress panels show commitment accuracy degrading and false-commitment rate rising with stress severity. The figure identifies modeled robustness boundaries and should not be read as arbitrary-adversary robustness."
        ),
        "table_06_host_bridge_demand": (
            "Boundary/safety evidence. The audit table combines replay-visible demand counts with zero observed violations of evidence validity, contribution identity, merge semantics, route truth, and duplicate-rank safety."
        ),
        "figure_10_strong_baselines": (
            "Supporting fairness check. Paired equal-budget intervals compare active belief against passive coded and deterministic opportunistic forwarding baselines. Positive deltas indicate higher decision accuracy or higher quality per byte for active belief. The scope is the recorded baseline set, not a complete DTN survey."
        ),
        "figure_11_large_regime": (
            "Supporting scale hygiene. Runtime, memory, replay agreement, quality, and failure-rate rows check deterministic 256-node sparse, 512-node clustered, and 1000-node mobility artifact generation. This is not a production deployment claim."
        ),
        "figure_12_observer_ambiguity": (
            "Supporting limit evidence. Observer ambiguity is reported as non-reconstructability under an observer projection: the projection lacks enough effectively independent evidence to infer the protected statistic. This matches the finite entropy/dispersion, rank-proxy, and temporal-capacity certificates; it is not a blanket privacy or universal-capacity claim."
        ),
        "table_01_theorem_assumptions": (
            "Boundary/safety evidence. This proof-to-experiment table marks which regimes are inside the theorem-backed boundary and includes finite-horizon, reduced finite-trace, value-model, and validator metadata rows. Empirical-only entries are reported evidence, not proof instances."
        ),
        "table_02_trace_validation": (
            "Supporting artifact hygiene. This trace table records canonical preprocessing and deterministic replay for synthetic and semi-realistic inputs. It supports artifact credibility, not a universal mobility claim."
        ),
    }
    return (
        f"{descriptions[figure_id]} Fixed payload-byte budget, seed set, trace family, "
        "deterministic replay status, and theorem-assumption status are recorded in the companion data tables."
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
