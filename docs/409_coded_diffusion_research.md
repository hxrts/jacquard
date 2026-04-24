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

The Lean theorem boundary is `verification/Field/CodedDiffusion.lean`, imported as `Field.CodedDiffusion`. It now owns the Phase 1 proof-facing core for k-of-n reconstruction, duplicate non-inflation, innovative rank growth, reconstruction monotonicity, parent-contribution recoding soundness, recoded duplicate non-inflation, observer projection preservation, rank-deficit and duplicate-pressure accounting, and finite deterministic work recurrence. Probability-heavy anomaly-margin and observer-erasure claims remain explicit Phase 2+ placeholders.

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
