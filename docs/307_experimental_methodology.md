# Experimental Methodology

This page describes the methodology behind Jacquard's maintained experiment corpus. It explains what an experiment is in this codebase, which variables the corpus varies deliberately versus measures as outcomes, why engines are tuned before comparative experiments, how tuned configurations flow into those experiments, and the pipeline from simulator run to final report.

Readers who want to run an experiment before reading the methodology should go directly to [Running Experiments](502_running_experiments.md). For the simulator architecture the methodology sits on, see [Simulator Architecture](306_simulator_architecture.md).

## What An Experiment Is

An experiment in Jacquard is a maintained scenario family combined with a parameter sweep, a set of measured outputs, and a reduction into stable summary artifacts. A scenario family fixes the qualitative regime under test. A parameter sweep varies one or more engine or policy parameters inside that regime. A reduction aggregates per-run measurements into summary tables and boundary surfaces.

The unit of record is the scenario family, not a single scenario. Comparing two single runs invites noise from seed and regime idiosyncrasy. Comparing two families across multiple seeds averages over those idiosyncrasies and produces claims about engine or policy behavior rather than individual runs.

The simulator lanes described in [Simulator Architecture](306_simulator_architecture.md) are how these experiments execute. The full-stack lane drives the real router and engines for behavioral claims. The model lane runs pure planner and reducer fixtures for determinism checks that do not need host wiring.

## Independent And Dependent Variables

A Jacquard experiment varies deliberate dimensions and measures separate outcomes. The dimensions and outcomes are kept disjoint so any correlation between them is a claim the experiment supports rather than a tautology.

The independent variables in the maintained corpus are topology and density, delivery pressure, medium pressure, directional mismatch, topology movement, local node pressure, and workload class. See [Running Experiments](502_running_experiments.md) for the exact bands each variable takes.

The dependent variables are run outcomes. The report surfaces activation success, route presence, first materialization, first loss, recovery timing, route churn, engine handoffs, stress boundary, and first breakdown family. Per-engine cost and replay metrics extend the outcome set where applicable.

## Why Engines Are Tuned First

Every in-tree engine exposes parameters that strongly affect its behavior. Examples include the BATMAN decay window, the Pathway search budget, the Babel decay window, and the Field regime policy knobs.

Running a comparative experiment before locking those parameters would conflate per-engine tuning choices with per-engine capability. A comparison that pits one engine against another at arbitrary parameter settings measures both the relative engine choice and the tuning gap between them.

The tuning sweep isolates parameter effects under one engine at a time. Its output is a representative parameter set per engine, chosen as a defensible operating point in the regimes the corpus targets. Subsequent comparative experiments hold those parameters fixed, so the contrast measures engines rather than tuning.

## How Tuned Configurations Feed Into Experiments

Once the tuning phase selects a representative parameter set per engine, comparative and head-to-head experiments use those fixed settings. The corpus encodes the choice in the experiment suite definitions themselves. A reader auditing a comparison can trace which engines use which tuned parameters without a second lookup.

A 3rd party running an experiment that introduces a new engine should either contribute a tuning family for that engine first, or declare the fixed operating point it uses. The declared operating point must be defensible for the regimes in question. Skipping the tuning-first sequence is permitted only when the experiment is explicitly about tuning behavior itself.

The same discipline applies to policy parameters when a comparison covers policies that expose tuning knobs. A head-to-head that varies policy as well as engine should not simultaneously vary policy tuning unless tuning is the variable of interest.

## The Pipeline From Experiment To Report

A simulator run produces a per-run log plus aggregate and breakdown JSON files under the run directory. Diffusion runs add their own per-run log plus diffusion aggregate and boundary summaries. Head-to-head reductions are exported into the generated report directory. Model-lane runs add model artifacts as validation companions rather than scoring inputs.

The `analysis/` Python package reads those artifacts. `data.py` loads them into Polars frames. `scoring.py` derives per-run metrics. `tables.py` produces CSV tables. `plots.py` produces vector plots. `sections.py` and `document.py` compose report sections and lay them out. `report.py` is the entry that assembles the PDF.

Report outputs are stable across releases subject to explicit schema versioning. A 3rd party can rely on the artifact shape to build custom reductions or alternate reports without waiting on changes to the included pipeline.

Coded-diffusion observer ambiguity is reported as a measured frontier, not as a formal privacy guarantee. Its independent variables include observer projection, coding rate, fragment dispersion, deterministic forwarding-randomness mode, path-diversity preference, and reproduction target band. Its dependent variables include attacker top-1 accuracy, posterior uncertainty, mutual-information-style trace proxies, ambiguity-cost frontier area, cost, latency, and inference quality.

The coded-diffusion core experiments are simulator-local research fixtures, not
route-continuity tuning runs. Their central claim is path-free inference from
partial, independent evidence under deterministic temporal contacts. Exact
`k`-of-`n` recovery is treated as the set-union threshold case. The anomaly
localization task is treated as additive integer score-vector merging, where
the merged statistic directly determines margin, uncertainty, and decision
quality. The methodology does not claim a new erasure code or arbitrary machine
learning inference; it claims deterministic transport and merging of supported
mergeable sufficient statistics, plus measured near-critical cost control and
measured observer ambiguity proxies.

Active belief diffusion extends that methodology with first-class demand
summaries. Demand is measured as replay-visible control data, not as evidence.
The active artifact bundle reports an active belief grid, demand trace rows,
active-versus-passive rows, a no-central-encoder panel, compact second-task rows
for set-union and majority-threshold tasks, a recoding frontier, bounded
robustness rows, final validation rows, scaling-boundary rows, and figure sanity
rows. Its dependent variables include commitment lead time per receiver,
receiver agreement, belief divergence, collective uncertainty, demand
satisfaction, demand-response lag, evidence overlap, quality per byte, bytes at
commitment, duplicate and innovative arrivals, stale-demand ignored count,
false-confidence count, censored full-recovery status, commitment accuracy, and
measured R_est.

The active comparison keeps the same equal-payload-byte discipline as the
passive coded-diffusion baseline. Active comparison is not a
formula-derived offset from a passive summary. Passive controlled coded
diffusion, demand disabled, local-only demand, piggybacked demand, stale-demand
ablation, and full active belief diffusion are separate reduced causal runs over
the same deterministic event stream and fixed payload budget. Demand is
generated before forwarding choices, live demand feeds policy scoring through
demand value, and emitted, received, forwarded, piggybacked, expired,
ignored-stale, and satisfied demand rows are replay-visible. No-central-encoder
rows use oracle evaluation only after the run, so the simulator can score the
hidden target without giving any node a global input during the trace.

The active validation claim is bounded to compact mergeable tasks under the
modeled temporal-network assumptions. Agreement, divergence, collective
uncertainty, evidence overlap, commitment, bytes at commitment, and stress
outcomes are computed from receiver-indexed run state. The methodology supports
the empirical claim that demand can improve allocation and belief formation
while preserving the non-evidential safety boundary; it does not claim arbitrary
machine learning inference, consensus, common knowledge, or production-network
robustness.

Final proposal validation rows add a multi-seed and multi-regime layer over the
reduced causal runner. They cover sparse bridge-heavy and clustered
duplicate-heavy regimes, passive and active modes, set-union and
majority-threshold task rows, and deterministic replay assertions. The 500-node
item is represented as a scaling-boundary row unless a later experiment adds a
full 500-node run.
