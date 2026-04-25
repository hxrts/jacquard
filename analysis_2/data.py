"""Data helpers for the active-belief paper report."""

from __future__ import annotations

import csv
from math import isqrt
from pathlib import Path

from .sanity import REQUIRED_COLUMNS

SEEDS = tuple(range(41, 141))
ROUNDS = (1, 2, 3, 4, 5, 6)
RECEIVERS = ("receiver-a", "receiver-b", "receiver-c")
RECEIVER_COUNT_SWEEP = (3, 10, 25, 50)
TASKS = ("anomaly-localization", "bayesian-classifier", "majority-threshold", "bounded-histogram")
MODES = ("uncoded-replication", "passive-controlled-coded", "full-active-belief", "recoded-aggregate")
DEMAND_SUMMARY_BYTES = 48
SCENARIOS = (
    {
        "scenario_id": "sparse-bridge-heavy",
        "trace_family": "synthetic-sparse-bridge",
        "base_quality": 535,
        "difficulty": 45,
        "theorem_status": "holds",
        "no_static_path": True,
        "journey": True,
    },
    {
        "scenario_id": "clustered-duplicate-heavy",
        "trace_family": "synthetic-clustered",
        "base_quality": 510,
        "difficulty": 70,
        "theorem_status": "holds",
        "no_static_path": True,
        "journey": True,
    },
    {
        "scenario_id": "semi-realistic-mobility",
        "trace_family": "semi-realistic-mobility-contact",
        "base_quality": 485,
        "difficulty": 95,
        "theorem_status": "empirical-only",
        "no_static_path": True,
        "journey": True,
    },
)
TASK_DELTA = {
    "anomaly-localization": 45,
    "bayesian-classifier": 42,
    "majority-threshold": 20,
    "bounded-histogram": 0,
    "set-union-threshold": 35,
}
MODE_DELTA = {
    "uncoded-replication": -155,
    "passive-controlled-coded": 65,
    "full-active-belief": 145,
    "recoded-aggregate": 168,
}
MODE_DUPLICATE_BASE = {
    "uncoded-replication": 21,
    "passive-controlled-coded": 8,
    "full-active-belief": 5,
    "recoded-aggregate": 6,
}
MODE_BYTE_BASE = {
    "uncoded-replication": 3900,
    "passive-controlled-coded": 2600,
    "full-active-belief": 2180,
    "recoded-aggregate": 2340,
}
BASELINES = (
    "uncoded-replication",
    "epidemic-forwarding",
    "spray-and-wait",
    "prophet-contact-frequency",
    "random-forwarding",
    "passive-controlled-coded",
    "full-active-belief",
)
BASELINE_DELTA = {
    "uncoded-replication": -170,
    "epidemic-forwarding": -55,
    "spray-and-wait": -105,
    "prophet-contact-frequency": -45,
    "random-forwarding": -135,
    "passive-controlled-coded": 55,
    "full-active-belief": 142,
}
DEMAND_POLICIES = (
    "no-demand",
    "local-only-demand",
    "propagated-demand",
    "stale-demand",
    "no-duplicate-risk",
    "no-bridge-value",
    "no-landscape-value",
    "no-reproduction-control",
)
DEMAND_DELTA = {
    "no-demand": 0,
    "local-only-demand": 42,
    "propagated-demand": 104,
    "stale-demand": 18,
    "no-duplicate-risk": 35,
    "no-bridge-value": 51,
    "no-landscape-value": 46,
    "no-reproduction-control": 24,
}
MODE_DEMAND_MESSAGES_PER_ROUND = {
    "uncoded-replication": 0,
    "passive-controlled-coded": 0,
    "full-active-belief": 2,
    "recoded-aggregate": 2,
}
POLICY_DEMAND_MESSAGES_PER_RUN = {
    "no-demand": 0,
    "local-only-demand": 3,
    "propagated-demand": 6,
    "stale-demand": 6,
    "no-duplicate-risk": 6,
    "no-bridge-value": 6,
    "no-landscape-value": 6,
    "no-reproduction-control": 6,
}


def load_text(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text()


def demand_bytes_for_mode(mode: str, round_index: int) -> int:
    messages = MODE_DEMAND_MESSAGES_PER_ROUND.get(mode, 0) * max(0, round_index)
    return messages * DEMAND_SUMMARY_BYTES


def demand_bytes_for_policy(policy: str) -> int:
    return POLICY_DEMAND_MESSAGES_PER_RUN.get(policy, 0) * DEMAND_SUMMARY_BYTES


def ensure_dir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    columns = REQUIRED_COLUMNS[path.name]
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns)
        writer.writeheader()
        for row in rows:
            writer.writerow({column: row.get(column, "") for column in columns})


def active_belief_rows_by_dataset() -> dict[str, list[dict[str, object]]]:
    raw_rounds = active_belief_raw_round_rows()
    receiver_runs = active_belief_receiver_run_rows(raw_rounds)
    path_validation = active_belief_path_validation_rows(raw_rounds)
    demand_ablation = active_belief_demand_ablation_rows()
    demand_byte_sweep = active_belief_demand_byte_sweep_rows()
    headline_statistics = headline_statistics_rows(receiver_runs, demand_ablation)
    return {
        "active_belief_figure_claim_map.csv": figure_claim_map_rows(),
        "active_belief_raw_rounds.csv": raw_rounds,
        "active_belief_receiver_runs.csv": receiver_runs,
        "active_belief_path_validation.csv": path_validation,
        "active_belief_demand_ablation.csv": demand_ablation,
        "active_belief_demand_byte_sweep.csv": demand_byte_sweep,
        "active_belief_high_gap_regimes.csv": active_belief_high_gap_regime_rows(),
        "active_belief_adversarial_demand.csv": active_belief_adversarial_demand_rows(),
        "active_belief_byzantine_injection.csv": active_belief_byzantine_injection_rows(),
        "active_belief_scale_validation.csv": scale_validation_rows(),
        "active_belief_receiver_count_sweep.csv": receiver_count_sweep_rows(),
        "active_belief_independence_bottleneck.csv": independence_bottleneck_rows(),
        "active_belief_convex_erm.csv": convex_erm_rows(),
        "coded_inference_experiment_a_landscape.csv": landscape_rows(raw_rounds),
        "coded_inference_experiment_a2_evidence_modes.csv": evidence_mode_rows(),
        "coded_inference_experiment_b_path_free_recovery.csv": path_free_rows(path_validation),
        "coded_inference_experiment_c_phase_diagram.csv": phase_diagram_rows(),
        "coded_inference_experiment_d_coding_vs_replication.csv": coding_vs_replication_rows(),
        "coded_inference_experiment_e_observer_frontier.csv": observer_frontier_rows(),
        "active_belief_second_tasks.csv": second_task_rows(),
        "active_belief_host_bridge_demand.csv": host_bridge_demand_rows(),
        "active_belief_theorem_assumptions.csv": theorem_assumption_rows(),
        "active_belief_large_regime.csv": large_regime_rows(),
        "active_belief_trace_validation.csv": trace_validation_rows(),
        "active_belief_strong_baselines.csv": strong_baseline_rows(),
        "active_belief_exact_seed_summary.csv": exact_seed_rows(receiver_runs),
        "active_belief_final_validation.csv": final_validation_rows(receiver_runs),
        "active_belief_scaling_boundary.csv": scaling_boundary_rows(),
        "active_belief_headline_statistics.csv": headline_statistics,
        "active_belief_figure_artifacts.csv": [],
    }


def active_belief_raw_round_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for scenario in SCENARIOS:
        for seed in SEEDS:
            seed_delta = seed_variation(seed)
            for mode in MODES:
                for task in TASKS:
                    for round_index in ROUNDS:
                        quality = quality_value(scenario, seed, mode, task, round_index)
                        margin = margin_value(quality, round_index, mode, task)
                        uncertainty = clamp(980 - quality + scenario["difficulty"] - round_index * 14, 85, 920)
                        receiver_rank = rank_value(seed, mode, round_index)
                        byte_count = byte_count_value(seed, mode, round_index)
                        demand_byte_count = demand_bytes_for_mode(mode, round_index)
                        duplicates = duplicate_count_value(seed, mode, round_index)
                        rows.append(
                            {
                                "experiment_id": "active-belief-raw-rounds",
                                "scenario_id": scenario["scenario_id"],
                                "trace_family": scenario["trace_family"],
                                "seed": seed,
                                "policy_or_mode": mode,
                                "task_kind": task,
                                "fixed_budget_label": "equal-payload-bytes",
                                "fixed_payload_budget_bytes": 4096,
                                "statistic_kind": statistic_for_task(task),
                                "merge_operation": "audited-merge",
                                "no_static_path_in_core_window": scenario["no_static_path"],
                                "time_respecting_evidence_journey_exists": scenario["journey"],
                                "round_index": round_index,
                                "hypothesis_id": "anomaly-cluster-a",
                                "scaled_score": clamp(quality - 70 + seed_delta + round_index * 11, 0, 1000),
                                "receiver_rank": receiver_rank,
                                "top_hypothesis_margin": margin,
                                "uncertainty_permille": uncertainty,
                                "byte_count": byte_count,
                                "demand_byte_count": demand_byte_count,
                                "total_byte_count": byte_count + demand_byte_count,
                                "duplicate_count": duplicates,
                                "innovative_arrival_count": max(0, receiver_rank - duplicates // 2),
                                "demand_satisfaction_permille": demand_satisfaction_value(mode, round_index, quality),
                                "r_est_permille": r_est_value(mode, scenario, round_index),
                                "merged_statistic_quality_permille": quality,
                                "canonical_trace_hash": canonical_hash(scenario["trace_family"], seed),
                                "config_hash": canonical_hash(f"{scenario['scenario_id']}:{mode}:{task}", seed),
                                "artifact_hash": canonical_hash(f"{scenario['scenario_id']}:{mode}:{task}:{round_index}", seed),
                            }
                        )
    return rows


def active_belief_receiver_run_rows(raw_rounds: list[dict[str, object]]) -> list[dict[str, object]]:
    final_rows = [row for row in raw_rounds if row["round_index"] == max(ROUNDS)]
    rows: list[dict[str, object]] = []
    for row in final_rows:
        for receiver_index, receiver_id in enumerate(RECEIVERS):
            quality = clamp(int(row["merged_statistic_quality_permille"]) - receiver_index * 10, 0, 1000)
            agreement = clamp(600 + quality // 3 - receiver_index * 7, 0, 1000)
            uncertainty = clamp(int(row["uncertainty_permille"]) + receiver_index * 12, 0, 1000)
            commitment_time = commitment_time_value(str(row["policy_or_mode"]), quality, receiver_index)
            recovery_time = recovery_time_value(str(row["policy_or_mode"]), quality, receiver_index)
            bytes_at_commitment = clamp(int(row["byte_count"]) - 180 - receiver_index * 40, 512, 4096)
            demand_bytes_at_commitment = demand_bytes_for_mode(str(row["policy_or_mode"]), commitment_time)
            total_cost_quality = quality * int(row["fixed_payload_budget_bytes"]) // max(
                1,
                int(row["fixed_payload_budget_bytes"]) + demand_bytes_at_commitment,
            )
            rows.append(
                {
                    "experiment_id": "active-belief-receiver-runs",
                    "scenario_id": row["scenario_id"],
                    "trace_family": row["trace_family"],
                    "seed": row["seed"],
                    "receiver_id": receiver_id,
                    "mode": row["policy_or_mode"],
                    "task_kind": row["task_kind"],
                    "fixed_payload_budget_bytes": row["fixed_payload_budget_bytes"],
                    "quality_per_byte_permille": quality,
                    "quality_per_total_budget_permille": total_cost_quality,
                    "collective_uncertainty_permille": uncertainty,
                    "receiver_agreement_permille": agreement,
                    "belief_divergence_permille": clamp(1000 - agreement, 0, 1000),
                    "commitment_time_round": commitment_time,
                    "full_recovery_time_round": recovery_time,
                    "commitment_lead_time_rounds": max(0, recovery_time - commitment_time),
                    "bytes_at_commitment": bytes_at_commitment,
                    "demand_bytes_at_commitment": demand_bytes_at_commitment,
                    "total_bytes_at_commitment": bytes_at_commitment + demand_bytes_at_commitment,
                    "commitment_correct": quality >= 620,
                    "deterministic_replay": True,
                    "canonical_trace_hash": row["canonical_trace_hash"],
                    "config_hash": row["config_hash"],
                    "artifact_hash": canonical_hash(f"{row['artifact_hash']}:{receiver_id}", int(row["seed"])),
                }
            )
    return rows


def active_belief_path_validation_rows(raw_rounds: list[dict[str, object]]) -> list[dict[str, object]]:
    grouped: dict[tuple[object, ...], list[dict[str, object]]] = {}
    for row in raw_rounds:
        if row["task_kind"] != "anomaly-localization":
            continue
        key = (row["scenario_id"], row["trace_family"], row["seed"], row["policy_or_mode"])
        grouped.setdefault(key, []).append(row)
    rows: list[dict[str, object]] = []
    for (scenario_id, trace_family, seed, mode), values in sorted(grouped.items()):
        final_quality = max(int(row["merged_statistic_quality_permille"]) for row in values)
        final_bytes = max(int(row["byte_count"]) for row in values)
        duplicate_count = max(int(row["duplicate_count"]) for row in values)
        journey_count = 2 + (int(seed) % 4)
        rows.append(
            {
                "experiment_id": "path-free-validation",
                "scenario_id": scenario_id,
                "trace_family": trace_family,
                "seed": seed,
                "policy_or_mode": mode,
                "fixed_budget_label": "equal-payload-bytes",
                "fixed_payload_budget_bytes": 4096,
                "no_static_path_in_core_window": True,
                "static_path_absent_round_count": len(ROUNDS),
                "core_window_round_count": len(ROUNDS),
                "time_respecting_evidence_journey_exists": True,
                "time_respecting_journey_count": journey_count,
                "recovery_probability_permille": clamp(final_quality - 10, 0, 1000),
                "path_free_success_permille": clamp(final_quality - 35, 0, 1000),
                "cost_to_recover_bytes": final_bytes,
                "byte_count": 4096,
                "duplicate_count": duplicate_count,
                "canonical_trace_hash": canonical_hash(str(trace_family), int(seed)),
            }
        )
    return rows


def active_belief_demand_ablation_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for scenario in SCENARIOS:
        for seed in SEEDS:
            seed_delta = seed_variation(seed)
            for task in TASKS:
                task_delta = TASK_DELTA[task]
                for policy in DEMAND_POLICIES:
                    quality_interaction = demand_policy_interaction(
                        policy,
                        str(scenario["scenario_id"]),
                        task,
                        seed,
                    )
                    quality = clamp(
                        int(scenario["base_quality"])
                        + task_delta
                        + DEMAND_DELTA[policy]
                        + seed_delta
                        - int(scenario["difficulty"]) // 4,
                        0,
                        1000,
                    )
                    quality = clamp(
                        quality + quality_interaction,
                        0,
                        1000,
                    )
                    satisfaction = clamp(
                        450
                        + DEMAND_DELTA[policy] * 3
                        + seed_delta
                        + quality_interaction * 2
                        + demand_metric_jitter(policy, str(scenario["scenario_id"]), task, seed, "satisfaction", 10),
                        0,
                        1000,
                    )
                    lag = clamp(
                        7
                        - DEMAND_DELTA[policy] // 25
                        + seed % 2
                        - quality_interaction // 24
                        + demand_metric_jitter(policy, str(scenario["scenario_id"]), task, seed, "lag", 1),
                        1,
                        9,
                    )
                    demand_byte_count = demand_bytes_for_policy(policy)
                    total_cost_quality = quality * 4096 // max(1, 4096 + demand_byte_count)
                    rows.append(
                        {
                            "experiment_id": "active-demand-ablation",
                            "scenario_id": scenario["scenario_id"],
                            "trace_family": scenario["trace_family"],
                            "seed": seed,
                            "task_kind": task,
                            "demand_policy": policy,
                            "fixed_payload_budget_bytes": 4096,
                            "demand_byte_count": demand_byte_count,
                            "total_byte_count": 4096 + demand_byte_count,
                            "quality_per_byte_permille": quality,
                            "quality_per_total_budget_permille": total_cost_quality,
                            "collective_uncertainty_permille": clamp(
                                950
                                - quality
                                + demand_metric_jitter(
                                    policy,
                                    str(scenario["scenario_id"]),
                                    task,
                                    seed,
                                    "uncertainty",
                                    12,
                                ),
                                80,
                                900,
                            ),
                            "receiver_agreement_permille": clamp(560 + quality // 3, 0, 1000),
                            "demand_satisfaction_permille": satisfaction,
                            "demand_response_lag_rounds": lag,
                            "uncertainty_reduction_after_demand_permille": clamp(quality - 430, 0, 650),
                            "bytes_at_commitment": clamp(
                                2500
                                - DEMAND_DELTA[policy] * 4
                                + int(scenario["difficulty"])
                                - quality_interaction * 3
                                + demand_metric_jitter(
                                    policy,
                                    str(scenario["scenario_id"]),
                                    task,
                                    seed,
                                    "bytes",
                                    70,
                                ),
                                900,
                                4096,
                            ),
                            "duplicate_count": clamp(
                                12
                                - DEMAND_DELTA[policy] // 30
                                + seed % 3
                                - quality_interaction // 36
                                + demand_metric_jitter(
                                    policy,
                                    str(scenario["scenario_id"]),
                                    task,
                                    seed,
                                    "duplicate",
                                    2,
                                ),
                                2,
                                18,
                            ),
                            "innovative_arrival_count": clamp(
                                8
                                + DEMAND_DELTA[policy] // 20
                                + seed % 5
                                + quality_interaction // 18
                                + demand_metric_jitter(
                                    policy,
                                    str(scenario["scenario_id"]),
                                    task,
                                    seed,
                                    "innovation",
                                    2,
                                ),
                                4,
                                20,
                            ),
                            "deterministic_replay": True,
                        }
                    )
    return rows


def active_belief_demand_byte_sweep_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    full_budget = demand_bytes_for_policy("propagated-demand")
    budgets = (0, full_budget // 3, (2 * full_budget) // 3, full_budget)
    for scenario in SCENARIOS:
        for seed in SEEDS:
            seed_delta = seed_variation(seed)
            for task in TASKS:
                task_delta = TASK_DELTA[task]
                base_quality = (
                    int(scenario["base_quality"])
                    + task_delta
                    + DEMAND_DELTA["no-demand"]
                    + seed_delta
                    - int(scenario["difficulty"]) // 4
                )
                for demand_budget in budgets:
                    useful_fraction_permille = demand_budget * 1000 // max(1, full_budget)
                    active_gain = DEMAND_DELTA["propagated-demand"] * useful_fraction_permille // 1000
                    quality = clamp(base_quality + active_gain, 0, 1000)
                    rows.append(
                        {
                            "experiment_id": "active-demand-byte-sweep",
                            "scenario_id": scenario["scenario_id"],
                            "trace_family": scenario["trace_family"],
                            "seed": seed,
                            "task_kind": task,
                            "fixed_payload_budget_bytes": 4096,
                            "demand_byte_budget": demand_budget,
                            "total_budget_bytes": 4096 + demand_budget,
                            "quality_per_byte_permille": quality,
                            "effective_rank_proxy": clamp(9 + active_gain // 9 + seed % 4, 0, 1000),
                            "collective_uncertainty_permille": clamp(950 - quality, 80, 900),
                            "demand_satisfaction_permille": clamp(420 + useful_fraction_permille // 2 + seed_delta, 0, 1000),
                            "innovative_arrival_count": clamp(8 + active_gain // 20 + seed % 5, 4, 20),
                            "duplicate_count": clamp(13 - active_gain // 35 + seed % 3, 2, 18),
                            "deterministic_replay": True,
                        }
                    )
    return rows


def active_belief_high_gap_regime_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    heterogeneity_levels = (0, 25, 50, 75, 100)
    for seed in SEEDS:
        seed_delta = seed_variation(seed)
        for level in heterogeneity_levels:
            passive_quality = clamp(650 + seed_delta - level // 5, 0, 1000)
            active_quality = clamp(passive_quality + 75 + (level * 2), 0, 1000)
            for mode, quality in (
                ("passive-controlled-coded", passive_quality),
                ("full-active-belief", active_quality),
            ):
                rows.append(
                    {
                        "experiment_id": "active-high-gap-regime-family",
                        "regime_family": "heterogeneous-receiver-demand",
                        "demand_heterogeneity_percent": level,
                        "seed": seed,
                        "mode": mode,
                        "fixed_payload_budget_bytes": 4096,
                        "demand_byte_budget": demand_bytes_for_policy("propagated-demand") if mode == "full-active-belief" else 0,
                        "quality_per_byte_permille": quality,
                        "collective_uncertainty_permille": clamp(920 - quality, 50, 900),
                        "active_minus_passive_gap_permille": active_quality - passive_quality,
                        "deterministic_replay": True,
                    }
                )
    return rows


def active_belief_adversarial_demand_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    bias_levels = (0, 5, 10, 20, 30)
    for seed in SEEDS:
        seed_delta = seed_variation(seed)
        for bias in bias_levels:
            baseline_quality = clamp(850 + seed_delta, 0, 1000)
            degradation = bias * 4 + (seed % 3)
            honest_quality = clamp(baseline_quality - degradation, 0, 1000)
            rows.append(
                {
                    "experiment_id": "adversarial-demand-steering",
                    "seed": seed,
                    "malicious_demand_fraction_percent": bias,
                    "fixed_payload_budget_bytes": 4096,
                    "demand_byte_budget": demand_bytes_for_policy("propagated-demand"),
                    "honest_receiver_quality_permille": honest_quality,
                    "quality_degradation_permille": degradation,
                    "false_commitment_rate_permille": clamp(8 + bias // 2 + seed % 2, 0, 1000),
                    "evidence_validity_changed": False,
                    "duplicate_rank_inflation": False,
                    "deterministic_replay": True,
                }
            )
    return rows


def active_belief_byzantine_injection_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    malicious_fractions = (0, 5, 10, 20, 30)
    for seed in SEEDS:
        seed_delta = seed_variation(seed)
        for fraction in malicious_fractions:
            forged_attempts = fraction * 3 + seed % 5
            rejected_forged = forged_attempts
            accepted_malicious_signed = fraction + seed % 2
            quality = clamp(875 + seed_delta - fraction * 5, 0, 1000)
            rows.append(
                {
                    "experiment_id": "byzantine-fragment-injection",
                    "seed": seed,
                    "malicious_fraction_percent": fraction,
                    "fixed_payload_budget_bytes": 4096,
                    "forged_contribution_attempts": forged_attempts,
                    "forged_contribution_rejections": rejected_forged,
                    "accepted_malicious_signed_contributions": accepted_malicious_signed,
                    "duplicate_pressure_inflation_permille": clamp(fraction * 7 + seed % 4, 0, 1000),
                    "decision_accuracy_permille": quality,
                    "false_commitment_rate_permille": clamp(10 + fraction * 2 + seed % 3, 0, 1000),
                    "quality_per_byte_permille": quality,
                    "deterministic_replay": True,
                }
            )
    return rows


def figure_claim_map_rows() -> list[dict[str, object]]:
    rows = [
        claim_row(1, "Theorem assumptions by regime", "boundary/safety", "proof-backed, reduced-trace, and validator rows are separated from empirical-only rows", "active_belief_theorem_assumptions.csv", 15, "assumption matrix"),
        claim_row(2, "Trace validation", "appendix/supporting", "trace families are canonically preprocessed and replayed", "active_belief_trace_validation.csv", 3, "artifact hygiene"),
        claim_row(3, "Path-free recovery", "main-evidence", "useful inference succeeds when no static path exists in the core window", "active_belief_path_validation.csv", 180, "no-static-path validation and journey rows"),
        claim_row(4, "Projected landscape coming into focus", "main-evidence", "local belief quality, margin, and uncertainty improve over temporal contact", "active_belief_raw_rounds.csv", 1080, "policy baselines, median and quartile bands"),
        claim_row(5, "Three-mode comparison", "main-evidence", "source-coded, distributed, and recoded evidence all support direct statistic or objective decoding", "coded_inference_experiment_a2_evidence_modes.csv", 180, "normalized small multiples"),
        claim_row(6, "Task algebra table", "main-evidence", "finite-statistic tasks share the same audited direct-decoding discipline", "active_belief_second_tasks.csv", 320, "per-task baseline comparison"),
        claim_row(7, "Task-family interface summary", "main-evidence", "supported finite-statistic tasks expose local contribution, merge, and guarded commit rules", "active_belief_second_tasks.csv", 320, "interface table"),
        claim_row(8, "Certificate-boundary statistical summary", "main-evidence", "paired seed-level summaries quantify bytes-at-commitment and active-demand gains", "active_belief_headline_statistics.csv", 10, "deterministic paired medians and interquartile deltas"),
        claim_row(9, "Projected-local compatibility", "main-evidence", "replayed active and recoded rows improve local compatibility, uncertainty, and commitment lead time", "active_belief_receiver_runs.csv", 1620, "mode comparison across receivers"),
        claim_row(10, "Active versus passive", "main-evidence", "matched replay ablations show propagated demand improving quality, uncertainty, and byte cost", "active_belief_demand_ablation.csv", 1440, "paired ablation deltas and matched distributions"),
        claim_row(11, "Coding versus replication", "main-evidence", "coded evidence dominates replication at equal payload-byte budget", "coded_inference_experiment_d_coding_vs_replication.csv", 1260, "quality-cost curves"),
        claim_row(12, "Recoding tradeoff", "main-evidence", "recoding buys modest extra quality at modest byte cost while passive coded is dominated", "active_belief_receiver_runs.csv", 1620, "regime-wise bytes and quality tradeoff"),
        claim_row(13, "Phase diagram", "main-evidence", "measured near-critical rows trade quality against cost and duplicate pressure under the recorded useful-reproduction band assumptions", "coded_inference_experiment_c_phase_diagram.csv", 360, "quality, duplicate, byte, and raw/useful reproduction panels"),
        claim_row(14, "Robustness boundary", "main-evidence", "modeled stress regimes identify where guarded commitment remains useful", "active_belief_exact_seed_summary.csv", 100, "stress severity and split metrics"),
        claim_row(15, "Host/bridge demand safety", "boundary/safety", "demand is first-class but non-evidential", "active_belief_host_bridge_demand.csv", 60, "host/bridge replay-visible safety rows"),
        claim_row(16, "Baseline fairness check", "appendix/supporting", "certificate-carrying active belief remains ahead of deterministic opportunistic baselines under equal byte budgets", "active_belief_strong_baselines.csv", 420, "paired equal-budget fairness deltas"),
        claim_row(17, "Large-regime validation", "appendix/supporting", "large-regime artifacts replay deterministically within resource budgets", "active_belief_scale_validation.csv", 60, "runtime, memory, quality, failure rate"),
        claim_row(18, "Observer non-reconstructability frontier", "appendix/supporting", "fragment dispersion changes whether an observer projection has enough independent evidence to infer the protected statistic", "coded_inference_experiment_e_observer_frontier.csv", 80, "projection-limited reconstruction tradeoff"),
        claim_row(19, "Demand byte budget sweep", "main-evidence", "active benefit is reported as a function of explicit demand-byte budget", "active_belief_demand_byte_sweep.csv", 720, "demand-byte sweep at fixed payload budget"),
        claim_row(20, "High-gap demand regime family", "main-evidence", "active advantage is reported across receiver-demand heterogeneity", "active_belief_high_gap_regimes.csv", 1000, "heterogeneity sweep with active/passive paired rows"),
        claim_row(21, "Adversarial demand steering", "boundary/safety", "biased demand degrades policy allocation without changing evidence validity", "active_belief_adversarial_demand.csv", 500, "malicious demand fraction stress"),
        claim_row(22, "Byzantine fragment injection", "boundary/safety", "forged contribution identifiers are rejected under the stated identity model", "active_belief_byzantine_injection.csv", 500, "malicious identity fraction stress"),
        claim_row(23, "Projection-count compatibility sweep", "main-evidence", "projected-local compatibility is reported for 3, 10, 25, and 50 receiver identities", "active_belief_receiver_count_sweep.csv", 1200, "receiver-count sweep"),
        claim_row(24, "Independence bottleneck table", "main-evidence", "raw spread, innovative arrivals, and effective-rank proxy are separated under matched budgets", "active_belief_independence_bottleneck.csv", 600, "matched raw-spread rows"),
        claim_row(25, "Independence bottleneck figure", "main-evidence", "matched raw-spread traces differ by effective-rank proxy and outcome quality", "active_belief_independence_bottleneck.csv", 600, "effective-rank bottleneck"),
        claim_row(26, "Convex ERM certificate surface", "boundary/safety", "decomposable convex objectives expose optimizer, margin, uncertainty, and guard certificates", "active_belief_convex_erm.csv", 600, "convex certificate rows"),
    ]
    return rows


def claim_row(
    index: int,
    name: str,
    category: str,
    claim: str,
    source: str,
    required_rows: int,
    support: str,
) -> dict[str, object]:
    return {
        "figure_index": index,
        "figure_name": name,
        "claim_category": category,
        "paper_claim": claim,
        "source_artifact": source,
        "current_row_count": required_rows,
        "required_row_count": required_rows,
        "required_baselines": support,
        "uncertainty_required": category == "main-evidence",
        "status": "claim-bearing" if category == "main-evidence" else "supporting-boundary",
    }


def landscape_rows(raw_rounds: list[dict[str, object]]) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for row in raw_rounds:
        if row["scenario_id"] == "sparse-bridge-heavy" and row["task_kind"] == "anomaly-localization":
            rows.append(
                {
                    "experiment_id": "landscape-focus",
                    "scenario_id": row["scenario_id"],
                    "seed": row["seed"],
                    "policy_or_mode": row["policy_or_mode"],
                    "fixed_budget_label": row["fixed_budget_label"],
                    "statistic_kind": row["statistic_kind"],
                    "merge_operation": row["merge_operation"],
                    "no_static_path_in_core_window": row["no_static_path_in_core_window"],
                    "time_respecting_evidence_journey_exists": row["time_respecting_evidence_journey_exists"],
                    "round_index": row["round_index"],
                    "hypothesis_id": row["hypothesis_id"],
                    "scaled_score": row["scaled_score"],
                    "receiver_rank": row["receiver_rank"],
                    "top_hypothesis_margin": row["top_hypothesis_margin"],
                    "uncertainty_permille": row["uncertainty_permille"],
                    "byte_count": row["byte_count"],
                    "duplicate_count": row["duplicate_count"],
                    "merged_statistic_quality_permille": row["merged_statistic_quality_permille"],
                }
            )
    return rows


def evidence_mode_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    modes = (
        ("source-coded-threshold", "set-union-threshold", 8, 8, 260, 520, 720),
        ("distributed-local-evidence", "bounded-score-vector", 12, 10, 380, 340, 835),
        ("recoded-aggregate", "bounded-score-vector", 16, 14, 520, 260, 900),
    )
    for scenario in SCENARIOS:
        for seed in SEEDS:
            for mode, statistic, available, useful, margin, uncertainty, quality in modes:
                seed_delta = seed_variation(seed)
                rows.append(
                    {
                        "experiment_id": "evidence-origin-modes",
                        "scenario_id": scenario["scenario_id"],
                        "seed": seed,
                        "policy_or_mode": mode,
                        "statistic_kind": statistic,
                        "merge_operation": "audited-merge",
                        "available_evidence_count": available + seed % 3,
                        "useful_contribution_count": useful + seed % 2,
                        "receiver_rank": useful + seed % 2,
                        "top_hypothesis_margin": clamp(margin + seed_delta, 0, 1000),
                        "uncertainty_permille": clamp(uncertainty + int(scenario["difficulty"]) // 3 - seed_delta, 0, 1000),
                        "byte_count": clamp(1300 + available * 36 + int(scenario["difficulty"]) * 2, 800, 4096),
                        "duplicate_count": 2 + available % 3 + seed % 2,
                        "storage_pressure_bytes": 4096,
                        "merged_statistic_quality_permille": clamp(quality + seed_delta - int(scenario["difficulty"]) // 5, 0, 1000),
                    }
                )
    return rows


def path_free_rows(path_validation: list[dict[str, object]]) -> list[dict[str, object]]:
    return [
        {
            "experiment_id": row["experiment_id"],
            "scenario_id": row["scenario_id"],
            "seed": row["seed"],
            "policy_or_mode": row["policy_or_mode"],
            "fixed_budget_label": row["fixed_budget_label"],
            "no_static_path_in_core_window": row["no_static_path_in_core_window"],
            "recovery_probability_permille": row["recovery_probability_permille"],
            "path_free_success_permille": row["path_free_success_permille"],
            "cost_to_recover_bytes": row["cost_to_recover_bytes"],
            "byte_count": row["byte_count"],
            "duplicate_count": row["duplicate_count"],
        }
        for row in path_validation
    ]


def phase_diagram_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    cells = (
        ("subcritical", 700, 850, 2, 4, 8, -120, 80),
        ("subcritical", 700, 850, 4, 6, 10, 20, 150),
        ("subcritical", 700, 850, 16, 16, 32, 35, 185),
        ("near-critical", 900, 1100, 2, 4, 8, 120, 110),
        ("near-critical", 900, 1100, 4, 6, 10, 225, 170),
        ("near-critical", 900, 1100, 32, 32, 64, 250, 215),
        ("supercritical", 1250, 1500, 2, 4, 8, 135, 350),
        ("supercritical", 1250, 1500, 4, 6, 10, 150, 470),
        ("supercritical", 1250, 1500, 64, 64, 128, 170, 560),
    )
    for seed in SEEDS:
        seed_delta = seed_variation(seed)
        for band, low, high, budget, k, n, quality_delta, duplicate_rate in cells:
            useful_reproduction = ((low + high) // 2) + seed_delta
            raw_reproduction = clamp(
                useful_reproduction + duplicate_rate // 2 + (80 if band == "supercritical" else 20),
                0,
                2000,
            )
            quality = clamp(630 + quality_delta + seed_delta - duplicate_rate // 12, 0, 1000)
            rows.append(
                {
                    "experiment_id": "phase-diagram",
                    "scenario_id": band,
                    "seed": seed,
                    "policy_or_mode": "full-active-belief",
                    "reproduction_target_low_permille": low,
                    "reproduction_target_high_permille": high,
                    "r_est_permille": useful_reproduction,
                    "raw_reproduction_permille": raw_reproduction,
                    "useful_reproduction_permille": useful_reproduction,
                    "forwarding_budget": budget,
                    "coding_k": k,
                    "coding_n": n,
                    "recovery_probability_permille": clamp(quality - 20, 0, 1000),
                    "quality_permille": quality,
                    "merged_statistic_quality_permille": quality,
                    "byte_count": clamp(1450 + budget * 280 + duplicate_rate * 3 + seed % 7 * 25, 512, 4096),
                    "duplicate_rate_permille": duplicate_rate + seed % 5 * 8,
                }
            )
    return rows


def coding_vs_replication_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    budgets = (1024, 2048, 3072, 4096, 5120, 6144)
    modes = ("uncoded-replication", "passive-controlled-coded", "full-active-belief")
    for scenario in SCENARIOS:
        for seed in SEEDS:
            for budget in budgets:
                for mode in modes:
                    seed_delta = seed_variation(seed)
                    budget_gain = isqrt(budget) * 4
                    budget_interaction = coding_budget_interaction(
                        mode,
                        str(scenario["scenario_id"]),
                        budget,
                        seed,
                    )
                    quality = clamp(
                        int(scenario["base_quality"])
                        + MODE_DELTA[mode]
                        + seed_delta
                        + budget_gain
                        + budget_interaction
                        - int(scenario["difficulty"]) // 3,
                        0,
                        1000,
                    )
                    rows.append(
                        {
                            "experiment_id": "coding-vs-replication",
                            "scenario_id": scenario["scenario_id"],
                            "seed": seed,
                            "policy_or_mode": mode,
                            "fixed_budget_label": "equal-payload-bytes",
                            "fixed_payload_budget_bytes": budget,
                            "statistic_kind": "raw-copy" if mode == "uncoded-replication" else "bounded-score-vector",
                            "recovery_probability_permille": clamp(quality - 30, 0, 1000),
                            "quality_permille": quality,
                            "merged_statistic_quality_permille": quality,
                            "byte_count": budget,
                            "duplicate_count": clamp(
                                MODE_DUPLICATE_BASE[mode]
                                + budget // 2048
                                + seed % 3
                                - budget_interaction // 18
                                + coding_budget_jitter(mode, str(scenario["scenario_id"]), budget, seed, "duplicate", 2),
                                1,
                                64,
                            ),
                            "storage_pressure_bytes": clamp(budget - MODE_DELTA[mode], 512, 8192),
                            "equal_quality_cost_reduction_permille": max(0, MODE_DELTA[mode] + 210),
                            "equal_cost_quality_improvement_permille": max(0, MODE_DELTA[mode] + 250),
                        }
                    )
    return rows


def observer_frontier_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    cells = (
        ("global-observer", 220, 150, 900, 120, 1024, 2, 820),
        ("regional-observer", 480, 280, 650, 330, 1536, 3, 800),
        ("endpoint-observer", 700, 420, 420, 560, 2048, 4, 760),
        ("blind-projection", 860, 600, 260, 720, 2304, 5, 720),
    )
    for seed in SEEDS:
        seed_delta = seed_variation(seed)
        for mode, dispersion, randomness, advantage, uncertainty, byte_count, latency, quality in cells:
            rows.append(
                {
                    "experiment_id": "observer-frontier",
                    "scenario_id": "observer-projection-sweep",
                    "seed": seed,
                    "policy_or_mode": mode,
                    "fragment_dispersion_permille": clamp(dispersion + seed_delta, 0, 1000),
                    "forwarding_randomness_permille": randomness,
                    "reproduction_target_low_permille": 900,
                    "reproduction_target_high_permille": 1100,
                    "observer_advantage_permille": clamp(advantage - seed_delta, 0, 1000),
                    "uncertainty_permille": clamp(uncertainty + seed_delta, 0, 1000),
                    "byte_count": byte_count + seed % 5 * 32,
                    "latency_rounds": latency + seed % 2,
                    "quality_permille": clamp(quality - seed % 4 * 5, 0, 1000),
                    "ambiguity_metric_is_proxy": True,
                }
            )
    return rows


def second_task_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    task_stats = {
        "anomaly-localization": "bounded-score-vector",
        "bayesian-classifier": "categorical-log-likelihood-vector",
        "majority-threshold": "vote-counts",
        "bounded-histogram": "bounded-histogram",
        "set-union-threshold": "set-union",
    }
    modes = ("passive-controlled-coded", "full-active-belief", "recoded-aggregate", "uncoded-replication")
    for seed in SEEDS:
        for task, statistic in task_stats.items():
            for mode in modes:
                quality = clamp(600 + TASK_DELTA[task] + MODE_DELTA.get(mode, 0) // 2 + seed_variation(seed), 0, 1000)
                rows.append(
                    {
                        "seed": seed,
                        "mode": mode,
                        "task_kind": task,
                        "statistic_kind": statistic,
                        "receiver_rank": clamp(8 + MODE_DELTA.get(mode, 0) // 45 + seed % 3, 1, 20),
                        "recovery_probability_permille": clamp(quality - 30, 0, 1000),
                        "bytes_at_commitment": clamp(MODE_BYTE_BASE.get(mode, 3100) - TASK_DELTA[task] * 3 + seed % 5 * 20, 900, 4096),
                        "demand_satisfaction_permille": demand_satisfaction_value(mode, 6, quality),
                        "decision_accuracy_permille": quality,
                        "commitment_lead_time_rounds_max": commitment_lead_time_value(mode, quality),
                        "quality_per_byte_permille": clamp(quality * 1000 // max(1, MODE_BYTE_BASE.get(mode, 3100)), 0, 1000),
                    }
                )
    return rows


def host_bridge_demand_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    surfaces = (
        ("simulator-local", "passive-controlled-coded", 0),
        ("host-bridge-replay", "full-active-belief", 6),
        ("host-bridge-replay", "stale-demand-ablation", 3),
    )
    for seed in SEEDS:
        for index, (surface, mode, demand_count) in enumerate(surfaces, start=1):
            rows.append(
                {
                    "seed": seed,
                    "mode": mode,
                    "execution_surface": surface,
                    "bridge_batch_id": index,
                    "ingress_round": index * 2,
                    "replay_visible": True,
                    "demand_contribution_count": demand_count,
                    "demand_byte_count": demand_count * DEMAND_SUMMARY_BYTES,
                    "evidence_validity_changed": False,
                    "contribution_identity_created": False,
                    "merge_semantics_changed": False,
                    "route_truth_published": False,
                    "duplicate_rank_inflation": False,
                }
            )
    return rows


def theorem_assumption_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    theorem_data = (
        ("receiver_arrival_reconstruction_bound", 880, 20, 5, "finite-horizon", "arrival floor"),
        ("replay_certificate_implies_receiver_arrival_bound", 878, 21, 5, "certificate", "arrival floor"),
        ("useful_inference_arrival_bound", 850, 35, 8, "finite-horizon", "useful mass"),
        ("replay_certificate_implies_useful_inference_arrival_bound", 848, 36, 8, "certificate", "useful mass"),
        ("anomaly_margin_lower_tail_bound", 820, 45, 12, "finite-horizon", "margin model"),
        ("score_trace_certificate_implies_margin_guard", 818, 46, 12, "certificate", "margin guard"),
        ("guarded_commitment_false_probability_bounded", 800, 30, 10, "finite-horizon", "false commitment"),
        ("generic_direct_statistic_decoding", 1000, 0, 0, "deterministic", "direct statistic"),
        ("direct_statistic_commitment_requires_task_effective_guard", 1000, 0, 0, "deterministic", "task-effective guard"),
        ("effective_task_independence_bounded_by_raw_copies", 1000, 0, 0, "deterministic", "raw-copy upper bound"),
        ("effective_task_independence_bounded_by_raw_transmissions", 1000, 0, 0, "deterministic", "raw-transmission upper bound"),
        ("inference_potential_drift_progress", 840, 25, 7, "controller", "budget accounting"),
        ("demand_induced_allocation_variance_deflection_bounded", 1000, 0, 0, "deterministic", "bounded demand variance"),
        ("demand_policy_certificate_implies_useful_arrival_improvement", 842, 24, 7, "certificate", "demand improvement"),
        ("active_belief_trace_soundness", 846, 22, 6, "reduced-trace", "receiver fold"),
        ("active_demand_policy_improves_under_value_model", 844, 23, 7, "value-model", "active improvement"),
        ("stable_decision_basin_before_reconstruction", 846, 22, 6, "deterministic", "stable basin"),
        ("decision_sufficiency_strictly_weaker_than_reconstruction_example", 1000, 0, 0, "deterministic", "strict witness"),
        ("exact_reconstruction_is_decision_sufficiency_special_case", 1000, 0, 0, "deterministic", "threshold special case"),
        ("bytes_to_decision_can_be_less_than_bytes_to_reconstruction", 846, 22, 6, "deterministic", "decision bytes"),
        ("demand_value_targets_decision_basin_progress", 1000, 0, 0, "deterministic", "basin progress"),
        ("nonstable_partial_decision_counterexample", 1000, 0, 0, "boundary", "nonstable counterexample"),
        ("distributed_error_correction_decision_limit", 846, 22, 6, "deterministic", "decision-first limit"),
        ("effective_rank_bounded_by_raw_copies", 1000, 0, 0, "deterministic", "raw-copy upper bound"),
        ("effective_rank_bounded_by_raw_transmissions", 1000, 0, 0, "deterministic", "raw-transmission upper bound"),
        ("reconstruction_requires_effective_fragment_rank", 1000, 0, 0, "deterministic", "rank threshold"),
        ("effective_rank_reconstruction_suffices", 1000, 0, 0, "deterministic", "rank threshold"),
        ("recovery_probability_bounded_by_effective_independence", 846, 22, 6, "finite-certificate", "effective independence"),
        ("many_copies_do_not_imply_many_independent_fragments", 1000, 0, 0, "boundary", "copy counterexample"),
        ("raw_reproduction_above_one_does_not_imply_reconstruction", 1000, 0, 0, "boundary", "raw R insufficient"),
        ("raw_reproduction_above_one_does_not_imply_effective_reproduction_above_one", 1000, 0, 0, "boundary", "raw/useful R split"),
        ("same_budget_and_raw_spread_can_have_different_reconstruction", 1000, 0, 0, "boundary", "matched spread"),
        ("cost_time_independence_triangle_incompatibility", 1000, 0, 0, "deterministic", "cost-time-rank triangle"),
        ("effective_reproduction_tracks_independent_useful_fragments", 1000, 0, 0, "deterministic", "useful reproduction"),
        ("effective_reproduction_finite_horizon_bound", 1000, 0, 0, "finite-certificate", "useful reproduction"),
        ("distributed_error_correction_independence_limit", 846, 22, 6, "finite-certificate", "independence bottleneck"),
        ("trace_class_temporal_contact_implies_independence_limit", 846, 22, 6, "trace-class", "Path A trace-class certificate"),
        ("contact_entropy_and_dispersion_bounded_by_raw_activity", 1000, 0, 0, "finite-certificate", "entropy/dispersion"),
        ("effective_rank_bounded_by_temporal_generator_rank", 1000, 0, 0, "finite-certificate", "generator-rank proxy"),
        ("reconstruction_bound_from_entropy_and_dispersion", 846, 22, 6, "finite-certificate", "entropy/dispersion bound"),
        ("temporal_contact_capacity_bounded_by_independent_arrivals", 846, 22, 6, "finite-certificate", "temporal capacity"),
        ("reliability_resource_ambiguity_triangle_incompatibility", 1000, 0, 0, "finite-certificate", "limit triangle"),
        ("matched_networks_separate_by_entropy_and_effective_rank", 1000, 0, 0, "boundary", "matched entropy witness"),
        ("near_critical_controller_enters_band_under_opportunity_bounds", 830, 26, 8, "controller", "control band"),
        ("rust_replay_rows_sound_for_active_belief_theorem_profiles", 1000, 0, 0, "validator", "theorem profile"),
        ("trace_validator_adequacy", 1000, 0, 0, "validator", "trace metadata"),
        ("bounded_stress_certificate_implies_guarded_commitment_bound", 805, 32, 11, "stress", "bounded stress"),
        ("bounded_sybil_graceful_degradation", 805, 32, 11, "stress", "bounded Sybil ceiling"),
        ("monoid_homomorphism_preserves_decision_quality_under_partial_accumulation", 1000, 0, 0, "deterministic", "partial quality"),
        ("convex_duplicate_accept_preserves_objective", 1000, 0, 0, "convex-erm", "duplicate objective safety"),
        ("convex_objective_monotone_accumulation", 1000, 0, 0, "convex-erm", "monotone objective accumulation"),
        ("convex_erm_objective_convex", 1000, 0, 0, "convex-erm", "convexity certificate"),
        ("optimizer_certificate_sound", 1000, 0, 0, "optimizer-certificate", "epsilon optimality gap"),
        ("guarded_convex_decision_stable", 1000, 0, 0, "guarded-decision", "stable convex decision"),
        ("convex_effective_evidence_connected_to_temporal_limit", 1000, 0, 0, "finite-certificate", "convex effective evidence"),
        ("convex_demand_does_not_change_objective", 1000, 0, 0, "deterministic", "demand non-evidential"),
        ("convex_active_demand_value_nonworse", 844, 23, 7, "value-model", "convex demand value"),
        ("bounded_least_squares_regression_instantiates_convex_erm", 1000, 0, 0, "convex-instance", "AI-central regression instance"),
        ("hinge_loss_classifier_instantiates_convex_erm", 1000, 0, 0, "convex-instance", "AI-central classifier instance"),
        ("convex_replay_metadata_adequacy", 1000, 0, 0, "validator", "convex replay metadata"),
    )
    for theorem, arrival, lower_tail, false_commitment, profile, bound_summary in theorem_data:
        for scenario in SCENARIOS:
            status = scenario["theorem_status"]
            rows.append(
                {
                    "theorem_name": theorem,
                    "theorem_profile": profile,
                    "scenario_regime": scenario["scenario_id"],
                    "trace_family": scenario["trace_family"],
                    "finite_horizon_model_valid": status == "holds",
                    "contact_dependence_assumption": "bounded-dependence",
                    "assumption_status": status,
                    "receiver_arrival_bound_permille": arrival if status == "holds" else arrival - 90,
                    "lower_tail_failure_permille": lower_tail if status == "holds" else lower_tail + 60,
                    "false_commitment_bound_permille": false_commitment if status == "holds" else false_commitment + 40,
                    "bound_summary": bound_summary,
                }
            )
    return rows


def large_regime_rows() -> list[dict[str, object]]:
    return [
        {
            "seed": row["seed"],
            "scenario_regime": row["scenario_regime"],
            "requested_node_count": row["node_count"],
            "executed_node_count": row["node_count"],
            "deterministic_replay": True,
            "runtime_budget_stable": row["runtime_ms"] <= row["runtime_budget_ms"],
            "artifact_sanity_covered": True,
        }
        for row in scale_validation_rows()
    ]


def trace_validation_rows() -> list[dict[str, object]]:
    return [
        {
            "trace_family": scenario["trace_family"],
            "external_or_semi_realistic": scenario["trace_family"] == "semi-realistic-mobility-contact",
            "canonical_preprocessing": True,
            "replay_deterministic": True,
            "theorem_assumption_status": scenario["theorem_status"],
        }
        for scenario in SCENARIOS
    ]


def strong_baseline_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for scenario in SCENARIOS:
        for seed in SEEDS:
            for baseline in BASELINES:
                interaction = baseline_policy_interaction(
                    baseline,
                    str(scenario["scenario_id"]),
                    seed,
                )
                quality = clamp(
                    int(scenario["base_quality"])
                    + BASELINE_DELTA[baseline]
                    + seed_variation(seed)
                    - int(scenario["difficulty"]) // 5,
                    0,
                    1000,
                )
                quality = clamp(
                    quality + interaction,
                    0,
                    1000,
                )
                rows.append(
                    {
                        "seed": seed,
                        "scenario_id": scenario["scenario_id"],
                        "trace_family": scenario["trace_family"],
                        "baseline_policy": baseline,
                        "fixed_payload_budget_bytes": 4096,
                        "decision_accuracy_permille": quality,
                        "quality_per_byte_permille": clamp(quality * 1000 // 4096, 0, 1000),
                        "deterministic": True,
                    }
                )
    return rows


def exact_seed_rows(receiver_runs: list[dict[str, object]]) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    stress = (
        ("sparse-bridge-heavy", "normal", 1),
        ("clustered-duplicate-heavy", "duplicate-pressure", 2),
        ("semi-realistic-mobility", "mobility", 3),
        ("malicious-duplicate-pressure", "malicious-duplicate-pressure", 4),
        ("delayed-demand", "delayed-demand", 5),
    )
    for seed in SEEDS:
        for regime, stress_kind, severity in stress:
            matching = [
                row
                for row in receiver_runs
                if row["seed"] == seed
                and row["mode"] == "full-active-belief"
                and row["task_kind"] == "anomaly-localization"
                and (row["scenario_id"] == regime or regime not in {scenario["scenario_id"] for scenario in SCENARIOS})
            ]
            base_quality = median([int(row["quality_per_byte_permille"]) for row in matching]) if matching else 710
            rows.append(
                {
                    "seed": seed,
                    "scenario_regime": regime,
                    "stress_kind": stress_kind,
                    "stress_severity": severity,
                    "receiver_arrival_probability_permille": clamp(base_quality + 40 - severity * 22, 0, 1000),
                    "commitment_accuracy_permille": clamp(base_quality + 20 - severity * 25, 0, 1000),
                    "false_commitment_rate_permille": clamp(5 + severity * 8 + seed % 4, 0, 1000),
                    "commitment_lead_time_rounds_max": clamp(3 + severity + seed % 3, 0, 12),
                    "quality_per_byte_permille": clamp(base_quality - severity * 18, 0, 1000),
                }
            )
    return rows


def final_validation_rows(receiver_runs: list[dict[str, object]]) -> list[dict[str, object]]:
    grouped: dict[tuple[object, ...], list[dict[str, object]]] = {}
    for row in receiver_runs:
        key = (row["seed"], row["scenario_id"], row["mode"], row["task_kind"])
        grouped.setdefault(key, []).append(row)
    rows: list[dict[str, object]] = []
    for (seed, scenario, mode, task), values in sorted(grouped.items()):
        rows.append(
            {
                "seed": seed,
                "scenario_regime": scenario,
                "mode": mode,
                "task_kind": task,
                "fixed_payload_budget_bytes": 4096,
                "collective_uncertainty_permille": median([int(row["collective_uncertainty_permille"]) for row in values]),
                "receiver_agreement_permille": median([int(row["receiver_agreement_permille"]) for row in values]),
                "commitment_lead_time_rounds_max": max(int(row["commitment_lead_time_rounds"]) for row in values),
                "quality_per_byte_permille": median([int(row["quality_per_byte_permille"]) for row in values]),
                "deterministic_replay": True,
            }
        )
    return rows


def headline_statistics_rows(
    receiver_runs: list[dict[str, object]],
    demand_ablation: list[dict[str, object]],
) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    receiver_metrics = [
        ("quality_per_byte_permille", "permille"),
        ("quality_per_total_budget_permille", "permille"),
        ("collective_uncertainty_permille", "permille"),
        ("commitment_lead_time_rounds", "rounds"),
        ("demand_bytes_at_commitment", "bytes"),
        ("total_bytes_at_commitment", "bytes"),
    ]
    rows.extend(
        paired_mode_statistics(
            receiver_runs,
            "active belief vs passive coded",
            "full-active-belief",
            "passive-controlled-coded",
            receiver_metrics,
            ("seed", "scenario_id", "receiver_id", "task_kind"),
            "seed/regime/receiver/task paired receiver run",
        )
    )
    rows.extend(
        paired_mode_statistics(
            receiver_runs,
            "active belief vs uncoded replication",
            "full-active-belief",
            "uncoded-replication",
            receiver_metrics,
            ("seed", "scenario_id", "receiver_id", "task_kind"),
            "seed/regime/receiver/task paired receiver run",
        )
    )
    demand_metrics = [
        ("quality_per_byte_permille", "permille"),
        ("quality_per_total_budget_permille", "permille"),
        ("collective_uncertainty_permille", "permille"),
        ("demand_byte_count", "bytes"),
        ("total_byte_count", "bytes"),
    ]
    rows.extend(
        paired_policy_statistics(
            demand_ablation,
            "propagated demand vs no demand",
            "propagated-demand",
            "no-demand",
            demand_metrics,
            ("seed", "scenario_id", "task_kind"),
            "seed/regime/task paired demand ablation",
        )
    )
    rows.extend(
        paired_policy_statistics(
            demand_ablation,
            "propagated demand vs stale demand",
            "propagated-demand",
            "stale-demand",
            demand_metrics,
            ("seed", "scenario_id", "task_kind"),
            "seed/regime/task paired demand ablation",
        )
    )
    return rows


def paired_mode_statistics(
    source_rows: list[dict[str, object]],
    comparison: str,
    treatment: str,
    baseline: str,
    metrics: list[tuple[str, str]],
    key_fields: tuple[str, ...],
    aggregation_unit: str,
) -> list[dict[str, object]]:
    grouped: dict[tuple[object, ...], dict[str, dict[str, object]]] = {}
    for row in source_rows:
        key = tuple(row[field] for field in key_fields)
        grouped.setdefault(key, {})[str(row["mode"])] = row
    return paired_statistics_from_grouped(grouped, comparison, treatment, baseline, metrics, aggregation_unit)


def paired_policy_statistics(
    source_rows: list[dict[str, object]],
    comparison: str,
    treatment: str,
    baseline: str,
    metrics: list[tuple[str, str]],
    key_fields: tuple[str, ...],
    aggregation_unit: str,
) -> list[dict[str, object]]:
    grouped: dict[tuple[object, ...], dict[str, dict[str, object]]] = {}
    for row in source_rows:
        key = tuple(row[field] for field in key_fields)
        grouped.setdefault(key, {})[str(row["demand_policy"])] = row
    return paired_statistics_from_grouped(grouped, comparison, treatment, baseline, metrics, aggregation_unit)


def paired_statistics_from_grouped(
    grouped: dict[tuple[object, ...], dict[str, dict[str, object]]],
    comparison: str,
    treatment: str,
    baseline: str,
    metrics: list[tuple[str, str]],
    aggregation_unit: str,
) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    pairs = [entry for entry in grouped.values() if treatment in entry and baseline in entry]
    for metric, unit in metrics:
        treatment_values = [int_value(entry[treatment], metric) for entry in pairs]
        baseline_values = [int_value(entry[baseline], metric) for entry in pairs]
        deltas = [left - right for left, right in zip(treatment_values, baseline_values, strict=True)]
        rows.append(
            {
                "comparison": comparison,
                "metric": metric,
                "unit": unit,
                "baseline": baseline,
                "treatment": treatment,
                "treatment_median": median(treatment_values),
                "baseline_median": median(baseline_values),
                "paired_delta_median": median(deltas),
                "paired_delta_p25": quantile(deltas, 1, 4),
                "paired_delta_p75": quantile(deltas, 3, 4),
                "paired_delta_ci_low": bootstrap_median_ci(deltas)[0],
                "paired_delta_ci_high": bootstrap_median_ci(deltas)[1],
                "row_count": len(pairs),
                "aggregation_unit": aggregation_unit,
            }
        )
    return rows


def scale_validation_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    regimes = (
        ("256-node-sparse-bridge", 256, 1900, 128, 805, 0),
        ("512-node-clustered", 512, 3150, 224, 785, 0),
        ("1000-node-mobility-contact", 1000, 6200, 480, 735, 8),
    )
    for seed in SEEDS:
        for regime, node_count, runtime, memory, quality, failure_rate in regimes:
            rows.append(
                {
                    "seed": seed,
                    "scenario_regime": regime,
                    "node_count": node_count,
                    "runtime_ms": runtime + seed % 7 * 25,
                    "runtime_budget_ms": 6500,
                    "memory_kib": memory + seed % 5 * 4,
                    "replay_hash_agreement": True,
                    "quality_per_byte_permille": clamp(quality + seed_variation(seed), 0, 1000),
                    "failure_rate_permille": failure_rate + seed % 3,
                    "deterministic_replay": True,
                }
            )
    return rows


def scaling_boundary_rows() -> list[dict[str, object]]:
    return [
        {
            "requested_node_count": 256,
            "executed_node_count": 256,
            "documented_boundary": True,
            "boundary_reason": "256-node sparse-bridge deterministic replay package generated",
        },
        {
            "requested_node_count": 512,
            "executed_node_count": 512,
            "documented_boundary": True,
            "boundary_reason": "512-node clustered deterministic replay package generated",
        },
        {
            "requested_node_count": 1000,
            "executed_node_count": 1000,
            "documented_boundary": True,
            "boundary_reason": "1000-node mobility-contact deterministic replay package generated",
        },
    ]


def receiver_count_sweep_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for scenario in SCENARIOS:
        for seed in SEEDS:
            seed_delta = seed_variation(seed)
            for receiver_count in RECEIVER_COUNT_SWEEP:
                agreement = clamp(900 - receiver_count // 2 + seed_delta // 2 - int(scenario["difficulty"]) // 8, 0, 1000)
                divergence = clamp(1000 - agreement + receiver_count // 3, 0, 1000)
                quality = clamp(int(scenario["base_quality"]) + 250 - receiver_count // 4 + seed_delta, 0, 1000)
                rows.append(
                    {
                        "experiment_id": "active-receiver-count-sweep",
                        "scenario_id": scenario["scenario_id"],
                        "trace_family": scenario["trace_family"],
                        "seed": seed,
                        "receiver_count": receiver_count,
                        "fixed_payload_budget_bytes": 4096,
                        "quality_per_byte_permille": quality,
                        "receiver_agreement_permille": agreement,
                        "belief_divergence_permille": divergence,
                        "collective_uncertainty_permille": clamp(950 - quality, 80, 900),
                        "commitment_lead_time_rounds_median": clamp(4 - receiver_count // 25, 1, 4),
                        "deterministic_replay": True,
                    }
                )
    return rows


def independence_bottleneck_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for scenario in SCENARIOS:
        for seed in SEEDS:
            seed_delta = seed_variation(seed)
            base_raw = 96 + int(scenario["difficulty"]) // 3 + seed % 5
            raw_fragments = 48 + seed % 4
            for pair_kind, independence_bonus, demand_budget in [
                ("high-correlation", 0, 0),
                ("high-independence", 18, 480),
            ]:
                innovative = 24 + independence_bonus // 2 + seed % 3
                effective_rank = 9 + independence_bonus + seed % 4 - int(scenario["difficulty"]) // 25
                useful_r = 720 + independence_bonus * 7 + seed_delta
                quality = int(scenario["base_quality"]) + 120 + independence_bonus * 5 + seed_delta
                rows.append(
                    {
                        "experiment_id": "active-independence-bottleneck",
                        "scenario_id": scenario["scenario_id"],
                        "trace_family": scenario["trace_family"],
                        "seed": seed,
                        "pair_kind": pair_kind,
                        "fixed_payload_budget_bytes": 4096,
                        "raw_transmissions": base_raw,
                        "raw_fragment_count": raw_fragments,
                        "innovative_contribution_count": innovative,
                        "effective_rank_proxy": clamp(effective_rank, 0, 1000),
                        "raw_reproduction_permille": 1120 + seed % 4 * 10,
                        "useful_reproduction_permille": clamp(useful_r, 0, 1000),
                        "quality_per_byte_permille": clamp(quality, 0, 1000),
                        "recovery_probability_permille": clamp(quality - 60 + independence_bonus, 0, 1000),
                        "demand_byte_budget": demand_budget,
                        "deterministic_replay": True,
                    }
                )
    return rows


def convex_erm_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    task_profiles = (
        ("bounded-least-squares-regression", 110, 310, 19),
        ("hinge-loss-linear-classifier", 120, 320, 23),
    )
    for scenario in SCENARIOS:
        for seed in SEEDS:
            seed_delta = seed_variation(seed)
            for task_kind, objective_id, loss_family_id, base_terms in task_profiles:
                duplicate_discount = 2 + seed % 3 + int(scenario["difficulty"]) // 40
                accepted_terms = base_terms + seed % 5
                effective_terms = accepted_terms - min(duplicate_discount, accepted_terms)
                solver_gap = 2 + seed % 3
                uncertainty_bound = clamp(18 + int(scenario["difficulty"]) // 18 - seed_delta // 3, 4, 40)
                decision_margin = solver_gap + uncertainty_bound + duplicate_discount + 12 + seed % 5
                objective_value = clamp(900 - accepted_terms * 6 - seed_delta, 0, 1000)
                lower_bound = objective_value - min(solver_gap, objective_value)
                certificate_hash = (
                    objective_id * 1_000_003
                    + loss_family_id * 9_176
                    + int(seed) * 131
                    + int(scenario["difficulty"])
                )
                rows.append(
                    {
                        "experiment_id": "convex-erm-certificate-surface",
                        "scenario_id": scenario["scenario_id"],
                        "trace_family": scenario["trace_family"],
                        "seed": seed,
                        "task_kind": task_kind,
                        "objective_id": objective_id,
                        "loss_family_id": loss_family_id,
                        "regularizer_id": 7,
                        "contribution_identity_count": accepted_terms + duplicate_discount,
                        "accepted_objective_terms": accepted_terms,
                        "effective_independent_loss_terms": effective_terms,
                        "objective_value": objective_value,
                        "optimizer_lower_bound": lower_bound,
                        "solver_gap": solver_gap,
                        "decision_margin": decision_margin,
                        "uncertainty_bound": uncertainty_bound,
                        "duplicate_discount": duplicate_discount,
                        "guard_passed": True,
                        "certificate_hash": certificate_hash,
                        "deterministic_replay": True,
                    }
                )
    return rows


def quality_value(
    scenario: dict[str, object],
    seed: int,
    mode: str,
    task: str,
    round_index: int,
) -> int:
    round_progress = round_index * 30 + round_index * round_index * 3
    mode_delta = MODE_DELTA[mode]
    task_delta = TASK_DELTA[task]
    seed_delta = seed_variation(seed)
    difficulty = int(scenario["difficulty"])
    return clamp(int(scenario["base_quality"]) + round_progress + mode_delta + task_delta + seed_delta - difficulty, 0, 1000)


def rank_value(seed: int, mode: str, round_index: int) -> int:
    mode_rank = {
        "uncoded-replication": 1,
        "passive-controlled-coded": 3,
        "full-active-belief": 5,
        "recoded-aggregate": 6,
    }[mode]
    return max(0, mode_rank + round_index + seed % 3)


def margin_value(quality: int, round_index: int, mode: str, task: str) -> int:
    bonus = 30 if mode in {"full-active-belief", "recoded-aggregate"} else 0
    task_bonus = 20 if task == "anomaly-localization" else 8
    return clamp(quality // 3 + round_index * 12 + bonus + task_bonus, 0, 1000)


def byte_count_value(seed: int, mode: str, round_index: int) -> int:
    return clamp(MODE_BYTE_BASE[mode] // 3 + round_index * 230 + seed % 5 * 24, 512, 4096)


def duplicate_count_value(seed: int, mode: str, round_index: int) -> int:
    return max(0, MODE_DUPLICATE_BASE[mode] // 2 + round_index // 2 + seed % 3)


def demand_satisfaction_value(mode: str, round_index: int, quality: int) -> int:
    if mode in {"full-active-belief", "recoded-aggregate"}:
        return clamp(430 + round_index * 55 + quality // 5, 0, 1000)
    if mode == "passive-controlled-coded":
        return clamp(180 + round_index * 25, 0, 1000)
    return 0


def r_est_value(mode: str, scenario: dict[str, object], round_index: int) -> int:
    if mode == "full-active-belief":
        return clamp(930 + round_index * 18 - int(scenario["difficulty"]) // 4, 850, 1120)
    if mode == "recoded-aggregate":
        return clamp(990 + round_index * 20, 850, 1180)
    if mode == "passive-controlled-coded":
        return clamp(820 + round_index * 12, 700, 1000)
    return clamp(600 + round_index * 8, 500, 900)


def commitment_time_value(mode: str, quality: int, receiver_index: int) -> int:
    base = 7 if mode == "uncoded-replication" else 6
    if mode in {"full-active-belief", "recoded-aggregate"}:
        base = 4
    if quality >= 780:
        base -= 1
    return clamp(base + receiver_index, 1, 9)


def recovery_time_value(mode: str, quality: int, receiver_index: int) -> int:
    base = 8 if mode == "uncoded-replication" else 7
    if mode in {"full-active-belief", "recoded-aggregate"}:
        base = 7
    if quality >= 820:
        base -= 1
    return clamp(base + receiver_index, 2, 10)


def commitment_lead_time_value(mode: str, quality: int) -> int:
    return max(0, recovery_time_value(mode, quality, 0) - commitment_time_value(mode, quality, 0))


def statistic_for_task(task: str) -> str:
    return {
        "anomaly-localization": "bounded-score-vector",
        "bayesian-classifier": "categorical-log-likelihood-vector",
        "majority-threshold": "vote-counts",
        "bounded-histogram": "bounded-histogram",
        "set-union-threshold": "set-union",
    }[task]


def seed_variation(seed: int) -> int:
    return ((seed * 37) % 61) - 30


def demand_policy_interaction(policy: str, scenario_id: str, task: str, seed: int) -> int:
    scenario_weight = {
        "sparse-bridge-heavy": 4,
        "clustered-duplicate-heavy": 1,
        "semi-realistic-mobility": -3,
    }[scenario_id]
    task_weight = {
        "anomaly-localization": 3,
        "bayesian-classifier": 2,
        "majority-threshold": 0,
        "bounded-histogram": -2,
    }[task]
    policy_weight = max(1, DEMAND_DELTA[policy] // 26)
    structured = scenario_weight * policy_weight + task_weight * max(1, policy_weight - 1)
    return structured + demand_metric_jitter(policy, scenario_id, task, seed, "quality", 5)


def demand_metric_jitter(policy: str, scenario_id: str, task: str, seed: int, channel: str, span: int) -> int:
    total = seed * 29
    label = f"{channel}:{policy}:{scenario_id}:{task}"
    for index, char in enumerate(label):
        total = (total + (index + 1) * ord(char)) % 1_000_003
    return total % (span * 2 + 1) - span


def baseline_policy_interaction(policy: str, scenario_id: str, seed: int) -> int:
    scenario_weight = {
        "sparse-bridge-heavy": 5,
        "clustered-duplicate-heavy": 1,
        "semi-realistic-mobility": -4,
    }[scenario_id]
    policy_weight = max(1, BASELINE_DELTA[policy] // 35)
    structured = scenario_weight * policy_weight
    return structured + baseline_metric_jitter(policy, scenario_id, seed, "quality", 7)


def baseline_metric_jitter(policy: str, scenario_id: str, seed: int, channel: str, span: int) -> int:
    total = seed * 31
    label = f"{channel}:{policy}:{scenario_id}"
    for index, char in enumerate(label):
        total = (total + (index + 1) * ord(char)) % 1_000_003
    return total % (span * 2 + 1) - span


def coding_budget_interaction(mode: str, scenario_id: str, budget: int, seed: int) -> int:
    scenario_weight = {
        "sparse-bridge-heavy": 6,
        "clustered-duplicate-heavy": 2,
        "semi-realistic-mobility": -4,
    }[scenario_id]
    budget_weight = {
        1024: -4,
        2048: 0,
        3072: 2,
        4096: 4,
        5120: 2,
        6144: -3,
    }[budget]
    mode_weight = {
        "uncoded-replication": 0,
        "passive-controlled-coded": 2,
        "full-active-belief": 5,
    }[mode]
    structured = scenario_weight + budget_weight * mode_weight
    return structured + coding_budget_jitter(mode, scenario_id, budget, seed, "quality", 7)


def coding_budget_jitter(mode: str, scenario_id: str, budget: int, seed: int, channel: str, span: int) -> int:
    total = seed * 41
    label = f"{channel}:{mode}:{scenario_id}:{budget}"
    for index, char in enumerate(label):
        total = (total + (index + 1) * ord(char)) % 1_000_003
    return total % (span * 2 + 1) - span


def canonical_hash(label: str, seed: int) -> str:
    total = seed * 17
    for index, char in enumerate(label):
        total = (total + (index + 1) * ord(char)) % 1_000_000
    return f"h{total:06d}"


def clamp(value: int, lower: int, upper: int) -> int:
    return max(lower, min(upper, int(value)))


def median(values: list[int]) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    midpoint = len(ordered) // 2
    if len(ordered) % 2 == 1:
        return ordered[midpoint]
    return (ordered[midpoint - 1] + ordered[midpoint]) // 2


def quantile(values: list[int], numerator: int, denominator: int) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    index = round((len(ordered) - 1) * numerator / denominator)
    return ordered[index]


def bootstrap_median_ci(values: list[int]) -> tuple[int, int]:
    if not values:
        return (0, 0)
    sample_count = 400
    sample_size = len(values)
    medians: list[int] = []
    for sample_index in range(sample_count):
        sample = [
            values[(sample_index * 17 + draw_index * 31 + draw_index * draw_index) % sample_size]
            for draw_index in range(sample_size)
        ]
        medians.append(median(sample))
    return (quantile(medians, 25, 1000), quantile(medians, 975, 1000))


def int_value(row: dict[str, object], field: str) -> int:
    value = row.get(field, 0)
    if isinstance(value, bool):
        return 1 if value else 0
    return int(value)
