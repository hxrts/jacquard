# Coded Diffusion Research Boundary

This page is the active implementation boundary for the coded-diffusion research reset initially hosted inside `jacquard-field`.

The research contribution is not named Jacquard or Field. Jacquard is the deterministic implementation framework, and Field is the reusable experimental engine workspace. The result should remain name-independent so it can be split out later without inheriting Jacquard's routing-engine identity or Field's old corridor-routing framing.

## Active Research Direction

The active direction is resource-bounded diffusion-coded inference in temporal networks.

The semantic center is:

- target identity,
- message identity,
- evidence identity,
- evidence-origin mode,
- fragment identity,
- coding width and k-of-n reconstruction,
- deterministic payload-byte budget metadata,
- contribution-ledger identity,
- independent receiver rank,
- reconstruction event tick,
- fragment custody,
- innovative versus duplicate arrivals,
- recoding parent and contribution lineage,
- diffusion pressure,
- storage pressure,
- rank deficit,
- observer-visible fragment movement,
- reconstruction quorum,
- inference task identity,
- anomaly-localization candidate hypotheses,
- deterministic integer score vectors,
- top-hypothesis margin,
- fixed-denominator uncertainty proxy,
- decision commitment tick,
- receiver inference-quality summary.

The initial Rust research boundary is `crates/field/src/research.rs`. It defines the coded-diffusion vocabulary used by new work:

- `CodedTargetId`
- `DiffusionMessageId`
- `CodedEvidenceId`
- `DiffusionFragmentId`
- `EvidenceOriginMode`
- `CodedEvidenceRecord`
- `CodedEvidenceRecordInput`
- `CodedEvidenceValidity`
- `CodingWindow`
- `CodingRankId`
- `LocalObservationId`
- `ContributionLedgerId`
- `ContributionLedgerKind`
- `ContributionLedgerRecord`
- `ContributionLedgerRecordInput`
- `PayloadBudgetKind`
- `PayloadBudgetMetadata`
- `FragmentCustody`
- `ReceiverRankState`
- `ReceiverRankError`
- `ReconstructionQuorum`
- `DiffusionPressure`
- `FragmentSpreadBelief`
- `DiffusionOrderParameters`
- `NearCriticalControlState`
- `FragmentRetentionPolicy`
- `DelayedFragmentEvent`
- `FragmentReplayEvent`
- `PrivateProtocolRole`
- `InferenceTaskId`
- `AnomalyClusterId`
- `AnomalyHypothesisSet`
- `AnomalyHypothesisScore`
- `AnomalyDecisionGuard`
- `AnomalyLandscape`
- `AnomalyLandscapeSummary`
- `AnomalyEvidenceClass`
- `EvidenceVectorRecord`
- `EvidenceVectorBatch`
- `LandscapeUpdateEvent`
- `LandscapeUpdateOutcome`
- `DecisionCommitmentState`
- `AnomalyDecisionProgressSummary`
- `EvidenceOriginUpdateCounts`
- `ReceiverInferenceQualitySummary`

The Lean theorem boundary is `verification/Field/CodedDiffusion.lean`, imported as `Field.CodedDiffusion`. It now owns the Phase 1 proof-facing core for k-of-n reconstruction, duplicate non-inflation, innovative rank growth, reconstruction monotonicity, parent-contribution recoding soundness, recoded duplicate non-inflation, observer projection preservation, rank-deficit and duplicate-pressure accounting, and finite deterministic work recurrence. Probability-heavy anomaly-margin and full observer-ambiguity claims remain measured experimental claims unless a later formal privacy theorem is actually proved.

The Phase 2 anomaly-localization surface is now implemented on top of the Phase 1 contribution gate. Locally generated evidence carries `LocalObservationId`, recoded aggregate evidence carries parent contribution lineage plus an aggregate-with-local-observation ledger path, and `ReceiverRankState` exposes canonical accepted contribution ids. `EvidenceVectorRecord` attaches one deterministic integer score vector to one `CodedEvidenceId` plus `ContributionLedgerId` for one `AnomalyLandscape`. The pure `reduce_landscape_updates` reducer canonicalizes contribution order, applies innovative vectors once with saturating integer arithmetic, leaves duplicate arrivals quality-neutral, and emits `LandscapeUpdateEvent` records for replay. `DecisionCommitmentState` records a separate typed `Tick` when the top-hypothesis margin and minimum independent-evidence guard first pass; this remains distinct from exact k-of-n reconstruction. `ReceiverInferenceQualitySummary` reports receiver rank, reconstruction tick, commitment tick, top/runner-up hypotheses, margin, uncertainty, energy gap, innovative and duplicate update counts, and source/local/recoded origin counts.

The simulator readiness export is `crates/simulator/src/diffusion/coded_inference.rs`. It writes coded-inference artifacts under `artifacts/coded-inference/readiness`, not under routing-analysis reports. Its `CodedInferenceLandscapeEvent`, `CodedReceiverEvidenceEvent`, and `CodedInferenceReadinessSummary` records provide the first deterministic data stream for the "landscape coming into focus" figure: target id, round, hypothesis id, scaled score, top hypothesis, runner-up hypothesis, margin, uncertainty proxy, energy gap, receiver rank, reconstruction tick, commitment tick, innovative/duplicate counts, and evidence-origin counts.

## Phase 3 Baseline Comparison Surface

Phase 3 comparison artifacts live under `artifacts/coded-inference/baselines`. The required baseline roster is:

- `uncoded-replication`
- `epidemic-forwarding`
- `spray-and-wait`
- `uncontrolled-coded-diffusion`
- `controlled-coded-diffusion`

The primary fairness rule is equal-payload-byte comparison. Every Phase 3 comparison summary and aggregate artifact carries the fixed budget label `equal-payload-bytes` and the fixed payload budget `4096` bytes. A secondary budget is not part of the Phase 3 required comparison; if one is added later, it must be named explicitly and emitted as separate figure/table input rather than silently replacing the primary budget.

Every baseline runs over the coded-inference readiness trace format with the same seed, scenario family, receiver, hidden target, and metric schema. The shared summary reports recovery probability, decision accuracy, reconstruction round, commitment round, receiver rank, top-hypothesis margin, bytes transmitted, forwarding events, peak stored payload units and bytes per node, duplicate rate, innovative arrival rate, duplicate arrival count, innovative arrival count, optional target reproduction band, and optional measured reproduction pressure.

The controlled and uncontrolled coded diffusion outputs are the Phase 4 starting surface for local evidence policy adaptation. Controlled output carries the target reproduction band separately from measured reproduction pressure; uncontrolled output reports measured pressure without a target band. Neither output is allowed to depend on route admission, corridor publication, private route witnesses, route-quality ranking, dominant-engine selection, or routing-analysis filters.

Deferred optional baselines are direct delivery, PRoPHET/contact-frequency forwarding, and legacy Field corridor behavior. If any of them are added later, they remain explicitly baseline-only and must not become active coded-inference research surfaces.

## Phase 4 Local Evidence Policy Placement

Phase 4 local evidence policy state and decision artifacts are simulator-owned while the policy is being evaluated. The state derives from coded-inference readiness traces, baseline comparison inputs, and Field research vocabulary already exposed for evidence, landscape, commitment, quality, and reproduction-pressure summaries. It does not become a shared routing contract.

Placement is:

- `jacquard-field` owns reusable research vocabulary and existing local inference records.
- `jacquard-simulator` owns local policy telemetry, score breakdowns, deterministic reducers, ablation variants, scenario fixtures, and comparison artifacts.
- docs own the research boundary and the meaning of artifact fields.

The policy surface must remain integer-only and deterministic. It must not publish routes, construct corridor plans, own transport, assign host time, use floating-point state, depend on host iteration order, or use ambient randomness. Any random-forwarding ablation must use explicit seeded or stable deterministic ordering and must carry the same budget metadata as the full policy.

The simulator local policy state records:

- per-peer contact-rate estimates in permille,
- per-peer bridge score in permille derived from bridge contacts and contact diversity,
- storage pressure in bounded byte units,
- recent duplicate rate over a bounded deterministic window,
- recent innovative-forward success rate over a bounded deterministic window,
- measured local reproduction estimate R_est,
- optional receiver-likelihood, destination-region belief, or anomaly-region belief only when a scenario explicitly supplies those inputs.

The policy score is serialized as named integer terms:

```text
total_score =
  expected_innovation_gain
  + bridge_value
  + landscape_value
  - duplicate_risk
  - byte_cost
  - storage_pressure_cost
  - reproduction_pressure_penalty
```

Reducer artifacts rank deterministic `(peer, fragment)` candidates by this score, break equal scores by lower duplicate risk, lower byte cost, lower peer id, then lower fragment id, and emit selected or rejected decision rows. Each row carries `policy_id`, peer id, fragment id, selected/rejected status, optional budget rejection reason, total score, and every named score term. Rejections cover payload-byte budget, storage budget, reproduction budget, and forwarding-decision limit.

Phase 4 ablation artifacts use the same trace inputs and budget surface as the full policy. The required variants are `local-evidence-policy-no-bridge`, `local-evidence-policy-no-duplicate-risk`, `local-evidence-policy-no-landscape`, `local-evidence-policy-no-reproduction-control`, and `deterministic-random-forwarding`. Disabled terms are listed in each ablation row. The random-forwarding baseline uses an explicit seed and stable pseudo-random ordering; it never uses ambient randomness.

The Phase 4 scenario matrix contains sparse reproduction-pressure, clustered duplicate-heavy, and bridge-heavy fixtures. These fixtures exist to prove that bridge value, duplicate risk, landscape value, and reproduction control change behavior in visible, replayable ways before Phase 5 report generation.

## Phase 5 Near-Critical Control Placement

Phase 5 near-critical control remains simulator-owned. The simulator owns rolling reproduction-pressure accounting, controller state, controller ablations, potential summaries, sweep definitions, and plot-ready artifacts. Field continues to own reusable coded-diffusion research vocabulary only; the controller does not become a shared route, transport, or protocol contract.

The accounting surface is deterministic and integer-only:

- rolling R_est is computed from active forwarding opportunities and innovative successor opportunities,
- raw copies, innovative copies, receiver-arrival opportunities, duplicate arrivals, and decision-quality improvements are tracked separately,
- controller target bands use permille bounds R_low and R_high,
- hard caps cover storage units, transmissions, and payload bytes,
- W_infer and W_diff are named-term weighted integer potentials,
- sweep cells carry scenario identity, target band, forwarding budget, controller mode, caps, and seed.

Rolling reproduction pressure uses a bounded deterministic window. An empty window reports zero pressure. Window rollover drops the oldest event before adding the new one. R_est is emitted in permille from innovative successor opportunities divided by active forwarding opportunities, using widened integer arithmetic and a bounded result. Raw copies are accounting input, while innovative copies, receiver-arrival opportunities, duplicate arrivals, and decision-quality improvements remain separately observable so copy volume cannot masquerade as useful reproduction.

The near-critical controller compares measured R_est against R_low and R_high. Below the target band it tries to add one forwarding opportunity; inside the band it preserves the candidate opportunity count; above the band it suppresses forwarding. Storage, transmission, and payload-byte caps are checked before band adjustment and clamp emitted opportunities after adjustment, so the controller cannot spend beyond the hard caps. Decision records carry R_est, the target band, selected action, cap saturation flags, input opportunities, emitted opportunities, suppressed opportunities, and added opportunities.

The controller ablation disables only the near-critical adjustment path. It preserves the same trace inputs, target-band fields, copy counters, potential terms, resource caps, and budget schema as the full controller, and identifies its controller mode as disabled. This isolates whether the target-band feedback changes behavior without changing the accounting surface.

W_infer is the weighted integer sum of uncertainty, wrong-basin mass, duplicate pressure, storage pressure, and transmission pressure. W_diff is the weighted integer sum of rank deficit, active fragment pressure, storage pressure, and duplicate pressure for exact k-of-n reconstruction experiments. Both potential records emit named terms, named weights, per-round totals, and per-scenario summaries with initial, final, and maximum values.

The target-band and budget sweep covers subcritical, near-critical, and supercritical regions across low, nominal, and high forwarding budgets. Each cell runs the full controller and the controller ablation, then records recovery, commitment, quality, byte cost, transmission cost, storage pressure, duplicate pressure, W_infer, and W_diff. Plot-ready rows expose per-round R_est, target-band state, controller action, cap state, W_infer, W_diff, byte cost, and transmission cost; summary rows expose boundedness, recovery, commitment, cost, duplicate, storage, and potential maxima.

Near-critical control must not publish routes, construct corridor plans, own transport, assign host time, use floating-point control state, depend on host iteration order, or use ambient randomness. Any controller ablation or sweep uses the same deterministic trace and hard-cap schema as the full controller unless the ablation explicitly disables only the near-critical adjustment path.

## Phase 6 Observer Ambiguity Placement

Phase 6 observer ambiguity remains simulator-owned and measured. The simulator owns observer projections, the first attacker model, ambiguity metric proxies, ambiguity knob sweeps, robustness summaries, and plot-ready observer artifacts. Field and Lean remain definition and proof-scaffolding surfaces only; they do not turn the measured ambiguity frontier into a formal privacy claim.

The implemented observer projections are read-only views over coded-inference traces:

- `global` sees all contact and forwarding events,
- `regional` sees events touching a configured node subset,
- `endpoint` sees only endpoint-local contacts and forwarding arrivals,
- `blind` preserves coarse event timing and cost while erasing selected forwarding choices.

The first attacker is deterministic and targets anomaly-region inference from one projection at a time. It receives policy-family knowledge as explicit configuration and never reads hidden simulator state except for post-run scoring of the true target rank. Attacker result rows carry projection identity, target kind, policy family, hidden cluster id for scoring, top guess, true-target rank, top score, posterior-uncertainty proxy, and per-cluster hypothesis scores.

The ambiguity metrics are empirical proxies:

- attacker top-1 accuracy,
- posterior uncertainty in permille,
- a mutual-information-style hidden-variable/trace proxy,
- a mutual-information-style forwarding/contact graph proxy,
- area under the ambiguity-cost frontier.

Every proxy label includes `proxy` in the artifact-facing field so report text does not imply formal privacy. Exact mutual information is not claimed by these metrics.

The ambiguity sweep covers coding rate `k/n`, fragment dispersion, deterministic forwarding randomness, path-diversity preference where supported by the policy surface, reproduction target band, and observer projection. Forwarding randomness is represented only by explicit stable or seeded deterministic ordering identities. It never uses ambient randomness.

The theorem-backed explanation is intentionally narrow: the blind projection erases forwarding-choice fields such as receiver id, fragment id, evidence id, and policy id from forwarding rows. Projection tests prove those fields are absent from blind rows, and the attacker consumes only projection rows. This supports an erasure/noninterference explanation for that selected projection. It does not prove end-to-end privacy, differential privacy, or exact mutual-information bounds for the full temporal-network process.

## Phase 7 Core Experiments

Phase 7 core experiments live in
`crates/simulator/src/diffusion/core_experiment.rs` and write under the
`artifacts/coded-inference/core-experiments` namespace when exported. The rows
are figure inputs, not route-analysis rows, and they reuse the readiness,
baseline, near-critical, and observer surfaces described above.

The experiment set is:

- Experiment A, `coded_landscape_focus`: score landscape, receiver rank, margin,
  uncertainty, bytes, duplicates, and merged-statistic quality over time.
- Experiment A2, `coded_evidence_origin_modes`: source-coded reconstruction,
  distributed local evidence, and in-network recoding or aggregation, each with
  its merge algebra and useful contribution accounting.
- Experiment B, `coded_path_free_recovery`: recovery probability, cost to
  recover, and path-free successful reconstruction fraction.
- Experiment C, `coded_phase_diagram`: reproduction target band, forwarding
  budget, coding rate `k/n`, exact reconstruction, additive-inference quality,
  bytes, duplicate pressure, and measured R_est.
- Experiment D, `coded_vs_replication`: equal-payload-byte
  coding-versus-replication comparison with fixed budget metadata on every row.
- Experiment E, `coded_observer_frontier`: observer ambiguity proxy frontier
  over fragment dispersion, deterministic forwarding randomness, reproduction
  band, cost, latency, and quality.

The Phase 7 claim boundary is deliberately stronger than route availability but
narrower than general inference. The core fixtures machine-check that no
instantaneous source-to-receiver path exists in the core window while a
time-respecting evidence journey still exists. Useful inference can therefore
appear before any full payload or full raw observation set transits as a
continuous path. This is a claim about temporal evidence accumulation and
mergeable sufficient statistics, not about maintaining a stable route.

Exact `k`-of-`n` recovery is the set-union threshold case. The anomaly
localization surface is the additive integer score-vector case. Recoding and
aggregation are valid only when canonical contribution lineage prevents
duplicate rank inflation and duplicate statistic inflation. Arbitrary machine
learning inference, new erasure-code construction, and formal privacy claims are
outside this phase unless a later theorem or experiment explicitly adds them.

## Active Belief Diffusion Paper Package

The paper-facing research name is active belief diffusion. It should be framed
as a decentralized-inference primitive, not as Jacquard, Field, routing, or
MPST. In temporal networks without stable paths or a central aggregator, agents
exchange two bounded, replay-visible objects:

- mergeable coded evidence describing what they have learned,
- bounded demand summaries describing what would most reduce uncertainty.

The two objects are symmetric as communication objects but not semantically
symmetric. Coded evidence carries audited contribution identity and can update a
mergeable statistic. Demand summaries are non-evidential control data. They may
shape priority, custody, recoding, and allocation, but they must not validate
evidence, create contribution identity, alter merge semantics, directly change a
belief statistic, or publish route truth.

The paper structure should be organized around the primitive and its evidence:

1. Communication as inference.
2. Motivating anomaly-localization figure.
3. Active belief diffusion primitive.
4. Mergeable task model.
5. Demand, control, and resource accounting.
6. Theory and proof boundary.
7. Implementation and replay substrate.
8. Evaluation organized by claims.
9. Related work and positioning.
10. Limits and future work.

The theorem package is intentionally compact. `Field.CodedDiffusion` covers
evidence-origin modes, contribution ledgers, exact threshold reconstruction,
duplicate non-inflation, recoding soundness, observer projection preservation,
diffusion-potential accounting, inference-potential accounting, a
majority-threshold second-task boundary, and finite deterministic work
recurrence. `Field.CodedDiffusionStrong` adds the strong finite-horizon
assumption surface for receiver-arrival reconstruction, useful-inference
arrival, anomaly-margin lower-tail, guarded false-commitment, and
inference-potential drift claims. These theorems are assumption-explicit:
experiment artifacts must label the contact-dependence model, permille floors,
and whether each regime satisfies the theorem assumptions or remains
empirical-only.
`Field.ActiveBelief` covers bounded first-class demand, demand/evidence semantic
separation, demand soundness, duplicate non-inflation under demand-aware
forwarding, stale-demand safety, commitment lead-time accounting,
multi-receiver compatibility as guarded local agreement, and propagated
host/bridge demand soundness. The theorem dependency table and Rust
correspondence live in `verification/Field/CODE_MAP.md`.

The active experiment surface is `ActiveBeliefExperimentArtifacts` in
`crates/simulator/src/diffusion/core_experiment.rs`. Phase 13 replaced the
earlier scaffold with a reduced simulator-local causal runner. Each policy mode
is executed as a separate run over the same deterministic event stream and
payload-byte budget; active metrics are computed from receiver state, not from
fixed offsets against the passive summary. Demand is generated before
forwarding choices, feeds candidate scoring through demand value, and is then
tracked through replay-visible lifecycle rows. Oracle access is used only after
the run to score the hidden target.

It exports:

- the active belief grid over receiver, time, top hypothesis, margin,
  uncertainty, commitment, demand satisfaction, lag, agreement, divergence,
  evidence overlap, bytes at commitment, and measured R_est,
- demand trace rows for emitted, received, forwarded, piggybacked, expired,
  ignored-stale, and satisfied demand summaries,
- host/bridge demand replay rows that distinguish simulator-local demand from
  replay-visible host/bridge demand while proving demand remains non-evidential,
- active-versus-passive comparison rows under equal payload-byte budget,
- a no-central-encoder panel with oracle evaluation only after the run,
- compact second-task rows for set-union threshold, majority-threshold, and
  bounded-histogram mergeable tasks,
- a recoding frontier for forwarding-only, in-network aggregation, and active
  demand plus aggregation,
- bounded robustness rows for duplicate spam, selective withholding, biased
  observations, bridge-node loss, and stale recoded evidence.
- theorem-assumption rows mapping strong Lean theorem names to trace families,
  finite-horizon assumption status, receiver-arrival bounds, lower-tail failure
  bounds, and false-commitment bounds,
- 500-node large-regime rows with deterministic replay, runtime-stability, and
  artifact-sanity coverage,
- final proposal validation rows covering three deterministic seeds, sparse
  bridge-heavy, clustered duplicate-heavy, and semi-realistic mobility regimes,
  active modes, and all compact task kinds,
- figure sanity rows for the paper's eleven expected figures,
- a documented 500-node scaling boundary row retained for scoped-package
  compatibility.

The active modes are passive controlled coded diffusion, demand disabled,
local-only demand, piggybacked demand, stale-demand ablation, and full active
belief diffusion. The reduced runner maintains three receiver-indexed belief
landscapes with distinct accepted contribution histories. Agreement,
divergence, collective uncertainty, evidence overlap, demand satisfaction,
stale-demand ignored counts, false-confidence counts, bytes at commitment, and
measured reproduction pressure are all derived from those run states.

The evaluation must state fixed budgets, caps, and replay assumptions wherever
results are shown. The primary comparison fixes payload bytes. Demand summaries
carry explicit entry, byte, and lifetime caps. All metrics are deterministic
integer or fixed-denominator values under typed time/order and canonical
ordering.

The Phase 13 conclusion is conditional but positive: under the modeled
temporal-network assumptions and the compact mergeable tasks implemented here,
the causal active runs validate the active-belief thesis. Full active belief
diffusion improves a central collective metric over passive controlled coded
diffusion under equal payload bytes, the gain shrinks under a demand ablation,
stale demand affects only policy behavior, and demand never changes evidence
validity, contribution identity, duplicate accounting, or commitment guards.
The validated claim remains bounded to these mergeable tasks and reduced
simulator fixtures; it is not a claim about arbitrary ML inference or a
production network protocol.

The strong proposal closure adds a replay-visible host/bridge demand artifact
surface. It remains deliberately narrow: host bridges batch and replay demand
metadata, but demand still cannot validate evidence, create contribution
identity, alter merge semantics, publish route truth, or assign Jacquard time.

Paper non-claims:

- no consensus or common knowledge,
- no globally identical receiver beliefs,
- no optimal active-demand policy,
- no arbitrary compactness for all machine-learning tasks,
- no new erasure code claim,
- no formal privacy claim from observer-ambiguity proxies,
- no broad adversarial-security claim from bounded stress rows,
- no Telltale proof of empirical diffusion performance.

## Legacy Field Baseline

`docs/406_field_routing.md` is legacy context. It documents the old corridor-envelope Field engine that still exists as a runnable baseline.

The legacy baseline may still compile and run for comparison:

- corridor-envelope route candidate generation,
- private Telltale route search,
- route admission and materialization,
- bootstrap and continuity maintenance,
- route-shaped replay fixtures,
- reference-client and simulator profiles that still instantiate `FieldEngine`.

The baseline is not the research contribution. New research code should not depend on planner admission, route search, selected private paths, route-quality ranking, or corridor publication.

## Removed Or Renamed From The Research Path

The reset removed active research-facing corridor terminology from the coded-diffusion path:

- old route/search/replay exports are grouped under `jacquard_field::baseline`,
- replay narrowing counters use continuity-facing names,
- simulator diffusion forwarding uses `ContinuityBiased` instead of corridor-aware naming,
- diffusion reuse and persistence metrics use continuity and cluster-pair naming,
- Router/Search/Quality Lean packs are marked as legacy baseline context,
- Field corridor docs are marked baseline-only.

Some compatibility re-exports remain at the crate root while downstream simulator and reference-client code is converted. Those exports are compatibility surface, not the active research interface.

## Implementation, Proof, And Experiment Split

Implementation work lives initially in `jacquard-field` because it already has deterministic runtime, observer, control, replay, retention, and private protocol scaffolding.

Proof scaffolding lives under `verification/Field`. The active coded-diffusion proof entry point is `verification/Field/CodedDiffusion.lean`; reusable support remains in Information, Model, Retention, Async, and Protocol modules after their statements are converted to fragment/reconstruction semantics.

Experimental evaluation lives in `jacquard-simulator` and the analysis pipeline. Field's old corridor baseline remains useful as a comparator, but the experimental metrics for the research path should report fragment spread, reconstruction progress, duplicate pressure, storage pressure, and diffusion-potential behavior.

## Hard Boundary

New coded-diffusion research code must use the research boundary and may only use legacy Field route machinery as an explicit baseline comparator. If a route-centered surface is retained, it must be documented as baseline-only or converted to fragment/reconstruction semantics before it is used in the research path.
