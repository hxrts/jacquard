# Active Belief Diffusion Strong Paper Artifact

This page is the reproducible paper artifact for the strong active belief
diffusion claim. It is a manuscript-facing companion to
[Coded Diffusion Research Boundary](409_coded_diffusion_research.md), not a new
runtime contract.

## Claim Boundary

Active belief diffusion is a compact mergeable-inference primitive for temporal
networks. Agents exchange two bounded replay-visible objects:

- coded evidence carrying audited sufficient-statistic contributions,
- demand summaries carrying audited uncertainty and need.

The two objects have symmetric communication status and asymmetric semantics.
Demand can shape forwarding, custody, recoding priority, and allocation. Demand
cannot validate evidence, create contribution identity, alter merge semantics,
update a belief statistic, publish route truth, or assign time.

The strong claim is limited to compact mergeable inference tasks under explicit
finite-horizon contact assumptions. It does not claim consensus, common
knowledge, formal privacy, optimal active policy, arbitrary ML inference, or
robustness against arbitrary adaptive adversaries.

The theorem-backed claims apply only when the theorem-assumption rows mark the
finite-horizon contact, update-bound, guard, and controller assumptions as
satisfied. Rows marked empirical-only are evidence for the implemented regime,
not proof instances.

## Strong Abstract

Active belief diffusion turns intermittent contact into an anytime
multi-agent inference process. Instead of routing raw observations to a central
aggregator, agents exchange bounded coded evidence and bounded demand
summaries. Evidence contributes audited sufficient-statistic updates; demand
describes which missing evidence would most reduce uncertainty. For compact
mergeable tasks, receivers can reconstruct thresholds or commit to guarded
decisions before full raw-observation recovery, without stable end-to-end paths,
central aggregation, consensus, or shared global state. The implementation
provides deterministic replay, host/bridge-visible demand exchange, explicit
resource accounting, theorem-assumption rows, and multi-regime experiments
covering anomaly localization, majority threshold, bounded histogram, and exact
set-union reconstruction.

## Introduction Shape

The paper opens with the active belief diffusion primitive, not Jacquard,
Field, routing, or MPST. The first figure should show anomaly-localization
beliefs sharpening over temporal contacts while receivers exchange evidence and
demand. The introduction then states the two-message interface, the mergeable
task algebra, the safety boundary around demand, and the result that useful
belief can form before full recovery under explicit assumptions.

## Theorem Assumption Table

| Result | Lean theorem | Assumptions | Artifact rows |
| --- | --- | --- | --- |
| k-of-n receiver arrival | `receiver_arrival_reconstruction_bound` | finite horizon, success floor, dependence mode, arrived rank at least `k` | `active_belief_theorem_assumptions.csv`, `active_belief_exact_seed_summary.csv` |
| useful inference arrival | `useful_inference_arrival_bound` | task-relevant contribution floor, quality threshold, finite horizon | `active_belief_theorem_assumptions.csv`, `active_belief_final_validation.csv` |
| anomaly margin | `anomaly_margin_lower_tail_bound` | bounded updates, correct-cluster advantage, lower-tail failure bound | `active_belief_theorem_assumptions.csv` |
| guarded false commitment | `guarded_commitment_false_probability_bounded` | margin threshold, evidence guard, false-commitment bound | `active_belief_exact_seed_summary.csv` |
| inference drift | `inference_potential_drift_progress` | controller progress credit and pressure debit | `active_belief_theorem_assumptions.csv` |
| host/bridge demand safety | `propagated_demand_cannot_validate_invalid_evidence`, `propagated_demand_duplicate_non_inflation` | replay-visible host/bridge demand record, valid bounded summary | `active_belief_host_bridge_demand.csv` |

## Task Breadth Table

| Task | Statistic | Merge | Decision | Quality | Duplicate rule |
| --- | --- | --- | --- | --- | --- |
| Exact reconstruction | set of contribution ids | set union | `rank >= k` | receiver rank | duplicate ids ignored |
| Anomaly localization | bounded integer score vector | vector addition | top hypothesis with margin and guard | uncertainty and margin | contribution ids counted once |
| Majority threshold | positive/negative vote counts | vote addition | majority sign | vote margin | contribution ids counted once |
| Bounded histogram | fixed bucket counts | histogram addition | top bucket | bucket margin | contribution ids counted once |

## Figure Manifest

Every figure must be regenerated from replayable artifact rows. The source CSVs
below are the required inputs for the final manuscript plots.

| Figure | Name | Source artifact |
| --- | --- | --- |
| 1 | landscape coming into focus | `coded_inference_experiment_a_landscape.csv` |
| 2 | path-free recovery | `coded_inference_experiment_b_path_free_recovery.csv` |
| 3 | three-mode comparison | `coded_inference_experiment_a2_evidence_modes.csv`, `active_belief_final_validation.csv` |
| 4 | active belief grid | `active_belief_final_validation.csv` |
| 5 | task algebra table | `active_belief_second_tasks.csv` |
| 6 | phase diagram | `coded_inference_experiment_c_phase_diagram.csv` |
| 7 | active versus passive | `active_belief_final_validation.csv` |
| 8 | coding versus replication | `coded_inference_experiment_d_coding_vs_replication.csv` |
| 9 | recoding frontier | `active_belief_final_validation.csv` |
| 10 | robustness boundary | `active_belief_exact_seed_summary.csv` |
| 11 | observer ambiguity frontier | `coded_inference_experiment_e_observer_frontier.csv` |
| 12 | host/bridge demand safety | `active_belief_host_bridge_demand.csv` |
| 13 | theorem assumptions by regime | `active_belief_theorem_assumptions.csv` |
| 14 | large-regime validation | `active_belief_large_regime.csv` |
| 15 | trace validation | `active_belief_trace_validation.csv` |
| 16 | opportunistic baseline comparison | `active_belief_strong_baselines.csv` |

Regenerate report artifacts with:

```bash
just tuning-report <artifact-dir>
just report-sanity
```

Focused development checks:

```bash
cargo test -p jacquard-simulator active_belief -- --nocapture
cargo test -p jacquard-simulator core_experiment -- --nocapture
python3 -m unittest analysis.tests.test_sanity
```

## Submission Artifact Boundary

This page is the manuscript-facing source artifact for the strong paper. The
reproducible report PDF is generated by `just tuning-report <artifact-dir>`, and
the documentation manuscript is checked through `just book`. Venue-specific
typesetting, page limits, bibliography style, and camera-ready PDF packaging are
submission work, not remaining research validation.

## Evaluation Structure

The evaluation should be claim-by-claim:

1. Direct statistic decoding works for compact mergeable tasks.
2. Demand improves allocation under equal payload-byte budget.
3. Demand remains non-evidential on simulator-local and host/bridge replay
   surfaces.
4. Multi-receiver guarded commitments are compatible without consensus.
5. Strong theorem assumptions are satisfied or explicitly labeled empirical.
6. 500-node large-regime rows replay deterministically.
7. Semi-realistic mobility-contact traces are canonically preprocessed and
   replay checked.
8. Robustness boundaries show where the primitive stops helping.

## Caption Audit Rules

Captions must name the fixed budget, seed set, regime set, trace family, and
theorem-assumption status. Captions must not use privacy, consensus, optimal,
general AI, adversarially robust, or deployment-ready language unless the
specific plotted artifact and theorem table support that claim.

## Limitations

The strong paper still does not claim arbitrary ML inference. It covers compact
mergeable sufficient statistics. Observer ambiguity remains an empirical proxy
unless a separate formal privacy definition is added. The contact-frequency
baseline is deterministic and useful for opportunistic-forwarding comparison,
but it is not a complete survey of all delay-tolerant routing strategies.
