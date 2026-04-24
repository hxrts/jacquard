# Field Verification Code Map

This map describes the current organization of `verification/Field`.

## Top-Level Theorem Packs

- `Field/CodedDiffusion.lean`
  - active coded-diffusion theorem path for Phase 1 evidence-origin modes, contribution ledgers, k-of-n reconstruction, duplicate non-inflation, recoding soundness, observer projection, diffusion-potential accounting, and finite deterministic work recurrence
- `Field/ActiveBelief.lean`
  - active belief diffusion theorem path for Phase 8 receiver-indexed belief state, first-class bounded demand messages, evidence messages, demand soundness, duplicate non-inflation under demand-driven forwarding, commitment lead-time accounting, stale-demand safety, and multi-receiver compatibility
- `Field/Architecture.lean`
  - shared enum vocabulary for projection kinds, refinement-ladder stages, lineage stages, and semantic-versus-proof-artifact roles
- `Field/CostAPI.lean`
  - shared work-unit and budget vocabulary reused by router, system, and adequacy cost packs

- `Field/LocalModel.lean`
  - imports the local observer-controller model, the probabilistic information layer, the local refinement theorems, and the first decision procedure
- `Field/Information.lean`
  - imports the information API, concrete probabilistic realization, Bayesian update layer, calibration/blindness packs, and quantitative difference lemmas
- `Field/PrivateProtocol.lean`
  - imports the reduced private choreography/runtime layer, conservation/coherence packs, concrete fixtures, the protocol-closure theorem pack, and the Telltale-family bridge
- `Field/Boundary.lean`
  - imports the observational controller-boundary theorems
- `Field/Adequacy.lean`
  - imports the Rust-runtime adequacy bridge, low-level runtime-to-canonical alignment theorems, search-aware adequacy closure, stronger projected runtime/system refinement theorems, runtime-state execution refinement theorems, runtime/system safety-preservation results, probabilistic preservation theorems, first budgeted-optimality preservation theorems, and proof-facing fixture cases
- `Field/Network.lean`
  - imports the reduced finite network layer and its first safety theorems
- `Field/Router.lean`
  - imports the reduced publication, admission, installation, lifecycle, canonical-selection, and posterior-decision layers
- `Field/Search.lean`
  - imports the proof-facing reduced field search boundary
- `Field/Async.lean`
  - imports the reduced async delivery semantics, transport lifecycle lemmas, and first async safety theorems
- `Field/Retention.lean`
  - imports the reduced payload-retention policy/custody layer, executable bounded retention instance, separation/refinement theorems, and proof-facing fixtures
- `Field/System.lean`
  - imports system-level summaries, reduced end-to-end semantics, probabilistic evidence-flow theorems, refinement to router-owned canonical selection above the async layer, and the first budgeted/reduced-context optimality theorems
- `Field/Quality.lean`
  - imports the reduced routing-quality / comparison, reference-best, and support-only refinement layer above the router and system boundaries
- `Field/Assumptions.lean`
  - imports the proof-contract vocabulary and theorem-packaging surface used across the field stack
- `Field/Field.lean`
  - umbrella import for the current field verification stack; imports `Field/CodedDiffusion.lean` and `Field/ActiveBelief.lean` as the active research theorem path and keeps older route/corridor packs as legacy baseline context

## Active Coded-Diffusion Path

- `Field/CodedDiffusion.lean`
  - owns the first active coded-diffusion proof vocabulary:
    - `EvidenceOriginMode` for source-coded, locally generated, and recoded/aggregated evidence
    - `EvidenceId`, `ContributionId`, and `LocalObservationId` for Phase 1 proof-facing ids
    - `CodingWindow` for k-of-n reconstruction requirements
    - `ReceiverRank` for independent receiver contribution ids and duplicate/innovative arrival accounting
    - `ReconstructionQuorum` for valid k-of-n reconstruction quorums
    - `ContributionLedgerKind` and `ContributionLedgerRecord` for source, local, parent-ledger-union, and aggregate-with-local-observation contribution validity
    - `FragmentObservation` and `ObserverProjection` for observer-visible fragment/rank/custody projection
    - `DiffusionPotential` for rank-deficit, duplicate-pressure, and storage-pressure accounting
    - `finiteWork` for deterministic finite-horizon work recurrence support
  - completed theorem names:
    - `coding_window_valid_k_pos`
    - `coding_window_valid_k_le_n`
    - `k_of_n_reconstruction`
    - `valid_quorum_implies_reconstruction`
    - `duplicate_non_inflation`
    - `innovative_arrival_increases_rank_by_one`
    - `innovative_evidence_increases_rank_exactly_when_new`
    - `duplicate_evidence_preserves_rank_when_present`
    - `reconstruction_monotonicity_innovative`
    - `recoding_soundness_parent_contribution_ledger`
    - `aggregate_contribution_requires_local_observation`
    - `recoded_duplicate_non_inflation`
    - `source_and_local_evidence_share_rank_accounting`
    - `observer_projection_preserves_rank`
    - `observer_projection_preserves_duplicate_count`
    - `observer_projection_preserves_custody_count`
    - `innovative_step_rank_deficit_nonincreasing`
    - `duplicate_step_preserves_rank_deficit`
    - `duplicate_step_increases_duplicate_pressure`
    - `phase1_potential_accounting_innovative`
    - `phase1_potential_accounting_duplicate`
    - `finite_work_recurrence`
    - `finite_work_step_monotone`
  - explicit Phase 2+ placeholders:
    - `phase2_anomaly_margin_concentration_placeholder`
    - `phase2_observer_erasure_noninterference_placeholder`
  - Telltale-family mapping:
    - Reuses conceptually, but does not import directly in the Phase 1 local model, `Distributed/Families/DataAvailability.*` for reconstruction quorum and retrievability vocabulary.
    - Emulates locally the finite, deterministic subset of `Runtime/Proofs/Lyapunov.lean`, `Runtime/Proofs/ProtocolMachinePotential.lean`, and `Classical/Families/FosterLyapunovHarris.lean` through `DiffusionPotential`, `phase1_potential_accounting_*`, and `finiteWork`.
    - Reuses conceptually `Runtime/Proofs/ObserverProjection.lean`, `Protocol/InformationCost.lean`, and `Protocol/Noninterference*.lean` for the observer projection/erasure story; only Phase 1 projection preservation is proved here.
    - Leaves probability-heavy concentration support in `Classical/Families/ConcentrationInequalities.lean` as an explicit Phase 2 target.
  - Rust alignment:
    - `EvidenceOriginMode`, `ContributionLedgerKind`, `ContributionLedgerRecord`, `CodingWindow`, `ReceiverRank`, and reconstruction/recoding theorem names intentionally mirror `crates/field/src/research.rs`.

## Active Belief Diffusion Path

- `Field/ActiveBelief.lean`
  - owns the Phase 8 active belief diffusion proof vocabulary:
    - `ReceiverId`, `HypothesisId`, and `DemandEntryId` for receiver-indexed active belief objects
    - `QualitySummary` for proof-facing uncertainty, margin, and evidence-count summaries
    - `ReceiverBeliefState` for audited receiver state over `ReceiverRank`
    - `DemandEntry` and `DemandSummary` for bounded advisory demand control data
    - `validDemandSummary` and `expiredDemandSummary` for demand caps and lifetime semantics
    - `EvidenceProposal` and `demandAwareAccept` for demand-aware forwarding through the ordinary contribution gate
    - `ActiveMessage` for the first-class exchanged-message surface covering evidence and demand
    - `CommitmentTimeline` and `commitmentLeadTime` for logged lead-time accounting
    - `GuardedCommitment` and `compatibleCommitments` for multi-receiver compatibility without consensus
    - `demandPriorityScore` for proof-facing priority metadata that does not affect evidence acceptance
  - completed theorem names:
    - `demand_bounded_by_entry_cap`
    - `demand_bounded_by_byte_cap`
    - `valid_demand_is_live`
    - `demand_message_carries_no_contribution`
    - `evidence_message_carries_contribution`
    - `demand_cannot_validate_invalid_evidence`
    - `demand_accepts_only_through_valid_evidence`
    - `demand_duplicate_non_inflation`
    - `expired_demand_does_not_accept_invalid_evidence`
    - `commitment_lead_time_soundness`
    - `same_guarded_basin_compatible`
    - `compatible_commitments_have_same_hypothesis`
    - `demand_priority_does_not_change_acceptance`
  - non-claims:
    - demand is first-class replay-visible communication data, but it is not evidence
    - receiver compatibility is agreement on a guarded local decision, not consensus, common knowledge, or globally identical beliefs
    - commitment lead time is a replay metric over logged events, not a correctness theorem
    - active demand is not claimed optimal under arbitrary mobility or adversarial traces
  - Rust alignment target:
    - `DemandSummary`, `DemandEntry`, receiver-indexed belief summaries, commitment lead-time rows, receiver agreement rows, demand satisfaction rows, and stale-demand rejection counters should be mirrored by Phase 9 Rust/replay artifacts before Phase 10 experiments expand.

### Phase 8 Theorem Dependency Table

| Paper claim | Lean object | Depends on | Rust/replay target |
| --- | --- | --- | --- |
| Demand is bounded first-class communication data | `DemandSummary`, `validDemandSummary`, `ActiveMessage.demand` | entry, byte, and ttl caps | demand emitted/received replay rows with caps |
| Demand carries no contribution identity | `demand_message_carries_no_contribution` | `ActiveMessage.contributionId?` | demand rows with no contribution id field or an explicit empty contribution slot |
| Evidence carries audited contribution identity | `evidence_message_carries_contribution` | `EvidenceProposal`, `ContributionId` | evidence rows with contribution id and validity fields |
| Demand cannot validate invalid evidence | `demand_cannot_validate_invalid_evidence` | `demandAwareAccept`, `EvidenceProposal.validEvidence` | invalid evidence rejection counters under active policy |
| Demand-driven duplicates do not inflate rank | `demand_duplicate_non_inflation` | `acceptContribution`, `ReceiverRank` | duplicate arrival rows and receiver-rank stability checks |
| Stale demand cannot justify invalid evidence | `expired_demand_does_not_accept_invalid_evidence` | `expiredDemandSummary`, `demandAwareAccept` | stale-demand ignored/rejected replay rows |
| Commitment lead time is a replay metric | `commitment_lead_time_soundness` | `CommitmentTimeline` | commitment and full-recovery event rows |
| Compatible decisions are guarded local decisions | `same_guarded_basin_compatible`, `compatible_commitments_have_same_hypothesis` | `GuardedCommitment` | receiver agreement rows over committed hypotheses |

### Phase 8 Non-Claim Note

Active belief diffusion exchanges two bounded replay-visible message classes:
coded evidence and demand summaries. They are symmetric as communication
objects, but not semantically symmetric. Evidence can carry audited contribution
identity into the mergeable statistic. Demand can describe uncertainty and shape
priority, custody, recoding, and allocation. Demand cannot validate evidence,
create contribution identity, change merge semantics, or directly change a
belief statistic. The Phase 8 theorem pack also does not claim consensus,
common knowledge, globally identical receiver beliefs, active-policy optimality,
formal privacy, or robustness against arbitrary adversaries.

## Legacy Route/Corridor Baseline Packs

These packs remain in the repository for comparison, regression, and proof-scaffold reuse, but they are no longer the active research theorem path:

- `Field/Router/*`
  - legacy router-owned publication, admission, installation, lifecycle, selector, canonical-route, cost, optimality, probabilistic, and resilience stack
- `Field/Search/*`
  - legacy proof-facing private route-search boundary
- `Field/Quality/*`
  - legacy route-view comparison and support-only refinement stack
- route/canonical portions of `Field/Adequacy/*` and `Field/System/*`
  - reusable only after conversion to reconstruction-facing runtime projection, fragment movement, or observer-projection statements

## Local Model

- `Field/Model/API.lean`
  - retained for coded diffusion as the reduced local controller model; semantic state vocabulary, explicit `ReducedBeliefSummary` reduction boundary, explicit `LocalOrderParameter` vocabulary, abstract round-step operations, and boundedness/harmony laws
- `Field/Model/Instance.lean`
  - first bounded concrete realization, structural theorems, temporal theorems, the Bayesian posterior companion view, a `LocalState` that stores `ReducedBeliefSummary` and `LocalOrderParameter` explicitly, explicit control-fusion from the stored summary into mean-field state, and regime classification over the stored order-parameter surface
- `Field/Model/Refinement.lean`
  - reduction-preservation, order-parameter preservation, stored-summary / stored-order-parameter chain theorems, sufficiency, conservativity, boundedness/monotonicity, and exogenous-control-dependence theorems for the controller-facing summary, plus the composed-round honesty/refinement pack
- `Field/Model/Decision.lean`
  - one-step finite exploration / decision procedure over a small evidence alphabet

## Information Layer

- `Field/Information/API.lean`
  - retained for coded diffusion as the observer ambiguity layer; abstract probability-simplex style normalization and information-theoretic operations over `FiniteBelief`
- `Field/Information/Instance.lean`
  - first concrete probability-simplex belief object, weight-normalized distribution, and entropy/mass theorems
- `Field/Information/Probabilistic.lean`
  - finite probabilistic route-hypothesis space, retained aggregate-mass helpers, and public-macrostate/blindness lemmas showing how the current public projection forgets latent quality/reliability structure; the controller-facing reduced summary still lives separately in `Field/Model/*`
- `Field/Information/Bayesian.lean`
  - Bayesian priors, factorized likelihoods, normalized posterior update, support/fallback theorems, and explicit boundary markers for correlated regimes outside the current factorized model
- `Field/Information/Calibration.lean`
  - confidence-threshold, posterior-probability, expected-utility, and regret-interpretation targets, plus decision-validity theorems, trusted explicit-observation soundness, public-projection distortion bounds, and an explicit correlated-regime calibration non-claim
- `Field/Information/Blindness.lean`
  - field-side information-cost / blindness bridge over the reduction-to-public-observer chain, including reduction-level erasure theorems, public-macrostate erasure, and aggregate-mass macrostate stability facts
- `Field/Information/Quantitative.lean`
  - L1 belief distance, small reduced-summary aggregate-gap objects, and first quantitative lemmas connecting posterior aggregate differences to reduction-level differences

## Private Protocol

- `Field/Protocol/API.lean`
  - retained for coded diffusion as bounded summary exchange and fragment-control coordination; reduced protocol roles, labels, machine state, global choreography, abstract projection/step/export laws
- `Field/Protocol/Boundary.lean`
  - thin boundary-facing import surface exposing the protocol API plus the current reduced instance for higher-layer boundary modules
- `Field/Protocol/Instance.lean`
  - first reduced summary-exchange instance
- `Field/Protocol/Bridge.lean`
  - Telltale-shaped reduced protocol-machine fragment and replay/observer bridge
- `Field/Protocol/Conservation.lean`
  - field-side conservation pack for evidence, authority, and replay-equivalent fragment traces, with direct-family instantiations kept separate from remaining local glue
- `Field/Protocol/Coherence.lean`
  - reduced updated-edge / incident-edge / unrelated-edge coherence lemmas
- `Field/Protocol/ReceiveRefinement.lean`
  - first typed receive-refinement hook aligned to `Consume` / subtype-replacement shape
- `Field/Protocol/Fixtures.lean`
  - proof-facing concrete summary/ack fixtures, fragment-trace observer-projection agreement, and fixed-participant/supported-reconfiguration examples
- `Field/Protocol/Closure.lean`
  - final reduced protocol-boundary theorem pack covering family alignment, receive-refinement witness closure, and fixed-participant/observational-reconfiguration closure
- `Field/Protocol/Reconfiguration.lean`
  - reduced protocol reconfiguration vocabulary covering owner transfer, checkpoint/restore, and continuation shift under a fixed participant set

## Boundary And Adequacy

- `Field/Model/Boundary.lean`
  - protocol/controller boundary from protocol exports and traces, with no runtime-artifact ownership
- `Field/Adequacy/API.lean`
  - abstract Rust-runtime artifact boundary, reduced router-facing runtime projection, runtime/search linkage metadata, reduced probabilistic slice, and reduced runtime-to-trace simulation witness
- `Field/Adequacy/Runtime.lean`
  - reduced runtime state, one-step runtime execution semantics, artifact extraction from runtime states/steps, and state-level adequacy/admission preservation lemmas
- `Field/Adequacy/Canonical.lean`
  - runtime-to-canonical refinement theorems connecting extracted runtime lifecycle routes to the system/router canonical selector under an explicit alignment boundary
- `Field/Adequacy/Cost.lean`
  - runtime/system cost-preservation theorems for projected artifacts, including exact preservation of canonical-search input, input size, search space, and search work units under the reduced runtime projection
- `Field/Adequacy/Optimality.lean`
  - projected-runtime budgeted-optimality theorems showing exact canonical agreement and zero regret once the reduced search budget covers the projected canonical-search surface
- `Field/Adequacy/Projection.lean`
  - reduced runtime artifact projection generated from `systemStep`, admission/honesty lemmas for that projection, and stronger runtime/system canonical refinement theorems with no extra alignment hypothesis
- `Field/Adequacy/Probabilistic.lean`
  - leading-evidence posterior extraction from runtime artifacts, runtime/trace confidence-threshold preservation, min-regret decision preservation, expected-utility order preservation, decision-relevant completeness for the reduced probabilistic projection, and an explicit erased-tail non-claim for the current reduced runtime view
- `Field/Adequacy/Search.lean`
  - reduced search projection, runtime-search adequacy object, optional reduced protocol reconfiguration, search-projection extraction functions, canonical-route refinement over quiescent runtime-search bundles, and negative-boundary theorems keeping router truth runtime-owned
- `Field/Adequacy/Refinement.lean`
  - runtime-state / system-state refinement relation, stuttering preservation of that relation under reduced runtime steps, and quiescent runtime-state consequences for canonical outcomes and first safety-preservation theorems; the semantic runtime-state object stays distinct from theorem-pack packaging and fixtures
- `Field/Adequacy/Safety.lean`
  - runtime/system reduction-soundness results for support conservativity, no false explicit-path promotion, no route creation from silence, admissible lifecycle origin, and quiescent observational equivalence
- `Field/Adequacy/Fixtures.lean`
  - proof-facing reduced runtime fixture cases covering canonical support selection, stronger router-selection tie handling, empty-runtime silence, one explicit non-claim scenario, and a small fixture-generation path from runtime artifacts or projected system states into proof-facing fixture objects
- `Field/Adequacy/ReplayFixtures.lean`
  - reduced replay-derived fixture vocabulary mirroring the maintained Rust replay export surface across search projection, protocol reconfiguration, runtime linkage, and recovery outcome scenarios
- `Field/Adequacy/ProbabilisticFixtures.lean`
  - proof-facing probabilistic fixtures covering explicit-evidence posterior support, correlated-evidence boundary marking, miscalibrated-likelihood divergence, and a sparse-evidence confidence guardrail
- `Field/Adequacy/Instance.lean`
  - first concrete runtime extraction, execution-level observational trace theorem, reduced simulation theorem, router-projection honesty facts, and evidence-agreement theorems
- `Field/AssumptionCore.lean`
  - proof-contract vocabulary, default/strengthened contract builders, and explicit convergence/resilience/search profile-family accessors over semantic, protocol-envelope, runtime-envelope, transport, participation, refinement, budget, and regime-profile assumption families
- `Field/AssumptionTheorems.lean`
  - theorem packaging layer deriving adequacy, quality, canonical-router, runtime-canonical, runtime-state execution refinement, and resilience-boundary consequences from the shared proof-contract vocabulary

## Network And Router

- `Field/Network/API.lean`
  - finite node/destination vocabulary, synchronous round buffer, delivered-message view, and local-harmony lift
- `Field/Network/Safety.lean`
  - first reduced network safety theorems connecting local honesty to publication, admission, and installation
- `Field/Router/Publication.lean`
  - router-facing publication candidates and publication honesty / well-formedness theorems
- `Field/Router/Selector.lean`
  - shared selector-family abstraction for lifecycle-route selection, covering candidate domain, eligibility filtering, fold-based best-route extraction, explicit selector-semantics metadata, explicit search execution-policy vocabulary, and a posture-to-execution-policy mapping that preserves selector semantics
- `Field/Router/Admission.lean`
  - reduced observed/admitted/rejected boundary and first admission conservativity theorems
- `Field/Router/Installation.lean`
  - minimal canonical installed-route object and installation honesty theorems
- `Field/Router/Lifecycle.lean`
  - reduced observed/admitted/installed/withdrawn/expired/refreshed lifecycle object plus maintenance and conservativity theorems
- `Field/Router/Canonical.lean`
  - router-owned destination-local canonical support selector over lifecycle routes, shared selector-family wrappers, support-best, eligibility, destination-scope containment, unique-eligible selection, a destination-local sparse-scaling theorem for off-destination route growth, a threshold discontinuity example, and threshold-emergence/disappearance theorems for canonical route truth
- `Field/Router/CanonicalStrong.lean`
  - stronger router-owned support-then-hop-then-stable selector over eligible lifecycle routes, plus shared selector-family wrappers and membership/eligibility theorems for the stronger canonical surface
- `Field/Router/Cost.lean`
  - proof-facing linear search-cost model for the canonical selector, including worst-case, incremental, stable-input, search-space, and maintenance-invariance bounds
- `Field/Router/Optimality.lean`
  - budgeted support-only canonical search, explicit support-regret vocabulary, anytime monotonicity, deadline-safety, and threshold-region theorems for the current router-owned objective
- `Field/Router/Probabilistic.lean`
  - router-owned confidence-threshold decision semantics over posterior belief, secondary posterior expectation / cost / risk / regret objects, threshold admissibility, dominance-monotonicity theorems, and explicit non-claim theorems separating posterior truth from support ranking and exported route views
- `Field/Router/Resilience.lean`
  - first participation-fault vocabulary, silence-only dropout budget, surviving-route projection, bounded-dropout support-stability theorems, and an explicit dishonest-publication non-claim

## Search Boundary

- `Field/Search/API.lean`
  - proof-facing reduced search boundary covering objective-to-query mapping, snapshot identity, execution-policy vocabulary, selected-result shape, reconfiguration metadata, and first replay-style lemmas

## Retention Boundary

- `Field/Retention/API.lean`
  - retained for coded diffusion as fragment custody and bounded holding policy; reduced payload-token, retention-policy input, retention-state, and abstract retention-step vocabulary plus boundary law bundles
- `Field/Retention/Instance.lean`
  - first bounded concrete retention instance with token aging, retain/carry/forward/drop policy, and executable state transitions
- `Field/Retention/Refinement.lean`
  - separation from local posterior/publication/canonical-route truth plus custody-conservation and forwarding-admissibility theorems
- `Field/Retention/Fixtures.lean`
  - proof-facing reduced retention scenarios covering retain, forward, drop, and checkpoint-restore cases

## Async And System Layers

- `Field/Async/API.lean`
  - retained for coded diffusion as delayed fragment delivery and forwarding; reduced async envelopes, explicit delay/retry/loss assumptions, queue stepping, ready-message view, and observer view
- `Field/Async/Safety.lean`
  - first async publication-safety theorems and queue-drain facts connecting the async layer back to local honesty
- `Field/Async/Transport.lean`
  - transport lifecycle lemmas for retry/delivery/drop behavior, publication injection, and the reliable-immediate refinement to the synchronous publication model
- `Field/Async/Bounded.lean`
  - one broader bounded-delay/bounded-retry regime, queue-growth and drain-after-transport bounds, ready-count bounds, no-strengthening theorems for existing in-flight claims, and one-retry-cycle fairness theorems for retry-eligible envelopes
- `Field/System/Statistics.lean`
  - aggregate local-support summaries and in-flight support-mass bounds over the async layer
- `Field/System/Bounded.lean`
  - system-facing queue and lifecycle-cardinality bounds for the broader async regime, plus source-projection preservation theorems, an explicit congestion/loss backlog budget, a proof-facing per-step work-unit bound, a one-retry-cycle queue-drain theorem under no-fresh-publication retry-only backlog assumptions, one-retry-cycle processing fairness for admissible retry-eligible updates, a first single-loss canonical-support stability theorem, a threshold-1 redundancy theorem for recovered support-dominating updates, a first graceful-degradation envelope, explicit intermittent-loss recovery and no-oscillation theorems after the recovery threshold, invalid-update withdrawal safety after retry recovery, and queue-clear recovery aliases back to the reliable-immediate canonical/convergence theorems
- `Field/System/Cost.lean`
  - proof-facing compute, communication, queue, and storage budget model for one reduced `systemStep`, including next-state preservation under the explicit transport-volume budget, stable communication/transport volume under the reliable-immediate fixed-point regime, amortized maintenance invariance, per-destination storage bounds, local/linear computability, max-bottleneck characterization, and a transport-derived graceful-resource-degradation theorem
- `Field/System/Optimality.lean`
  - system-facing wrappers for the budgeted support-only objective, including exact-within-budget, anytime monotonicity, deadline safety against the full optimum, route-view sufficiency, dominance preservation, no-rank-inversion, and threshold-region theorems
- `Field/System/Boundary.lean`
  - thin system-level assumption-boundary summary above the async/runtime stack, including projected-information order-insensitivity unlocks and explicit reliable-immediate fixed-point boundaries
- `Field/System/EndToEnd.lean`
  - reduced end-to-end state and step relation combining async transport, router lifecycle installation, and lifecycle maintenance, plus first safety/observer lemmas
- `Field/System/Convergence.lean`
  - reduced reliable-immediate fixed-point and no-spontaneous-promotion theorems over iterated end-to-end steps, plus a profile-indexed convergence interface separating local quantitative versus distributed/profile claims
- `Field/System/Canonical.lean`
  - system-facing refinement theorems connecting `supportDominance` winners to the router-owned canonical selector, plus shared selector-family wrappers, underconnected and unique-eligible sparse cases, thresholded canonical-support theorems, an explicit critical-threshold boundary, canonical support/knowledge conservativity for winners, a threshold-1 vanishing-support limit, reliable-immediate stability, global support-optimum packaging, one-step recovery and bounded-convergence theorems, and a no-oscillation theorem for the canonical system route in the current reliable-immediate bounded-delay corner
- `Field/System/Probabilistic.lean`
  - reduced probabilistic evidence-flow semantics over async envelopes and lifecycle routes, delayed/lossy/repeated/correlated observation vocabulary, message-to-observation update lemmas, stable-evidence posterior-choice preservation, bounded dropout-degradation and sparse-evidence guardrail theorems, and a system theorem connecting produced explicit candidates back to positive Bayesian explicit-path mass under the clean async regime
- `Field/System/Calibration.lean`
  - system-facing soundness theorem showing an explicit posterior decision on a produced candidate implies positive latent explicit-path mass in the reduced probabilistic state
- `Field/System/CanonicalStrong.lean`
  - system-facing stronger router selector based on support-then-hop-then-stable lifecycle choice, plus reliable-immediate stability and basic membership/eligibility theorems
- `Field/System/Resilience.lean`
  - system-facing bounded-dropout and bounded-non-participation stabilization/degradation theorems connecting reduced participation loss to canonical support behavior under the clean async regime, including reduced participation-cut and unique-bridge disappearance theorems
- `Field/System/Retention.lean`
  - system-facing retention/custody bridge above the async/runtime layer, including silence, no-delivery-without-custody, bounded retention work, and non-strengthening theorems for retained payloads
- `Field/Quality/API.lean`
  - reduced route-comparison views, admissibility rules, objective vocabulary, pairwise comparison objects, destination-filtered best-view selection, and maintenance-idempotence facts for exported route views
- `Field/Quality/Reference.lean`
  - reference admissibility and support-best semantics over exported route views, plus a destination-filtered support-only reference selector
- `Field/Quality/Refinement.lean`
  - support-only refinement theorems connecting `supportDominance` to the reference-best semantics, plus explicit counterexamples showing why tie-break and hop-band objectives are not promoted to global optimality
- `Field/Quality/System.lean`
  - system-facing quality theorems over `systemStep` lifecycle outputs, including stability, explicit-path non-manufacture, sender-local support/knowledge observer results, lifecycle-maintenance idempotence, and one-step appearance theorems for sparse active ready-installed evidence

## Notes

- layering rule:
  - `Field/Router` owns canonical route truth
  - `Field/Router/Probabilistic` owns posterior-based router decision truth
  - `Field/Quality` compares exported route views
  - `Field/Adequacy` owns reduction and runtime projection
  - `Field/Assumptions` packages contracts and theorem access
  - `Field/Information` and `Field/Model` own probabilistic local state, priors, likelihoods, and Bayesian posterior-update semantics
  - `Field/Retention` owns reduced payload custody, retention policy, and bounded retention execution state below router-owned route truth
  - only explicit support/canonical refinement theorems connect `Field/Quality` objectives back to router-owned truth; all other ranking objectives remain observational unless a theorem says otherwise

- stable architecture notes:
  - `PosteriorState`, `ReducedBeliefSummary`, `LocalOrderParameter`, and the Bayesian belief bridge are all explicit, and the reduced summary/order-parameter boundary is now stored directly in `LocalState`
  - `compressMeanFieldImpl` now owns only control fusion from `ReducedBeliefSummary` plus exogenous `controllerPressure`, instead of hiding posterior reduction internally
  - `Field/Model/Refinement.lean` now makes the intended theorem boundary explicit: the reduced summary is sufficient for the mean-field/controller surfaces only under fixed exogenous control inputs, and the theorem pack also records that the reduction alone does not determine the whole downstream control path
  - `LocalOrderParameter` is the explicit local phase/order-parameter surface between posterior reduction and control fusion
  - the corridor/coarse-graining story is now explicit end-to-end across `Field/Information/*` and `Field/Model/*`: retained aggregates feed the stored reduced summary, then the stored order parameter, then controller-facing fusion and public macrostate reasoning

- state taxonomy:
  - epistemic state: `FiniteBelief`, `PosteriorState`, `ProbabilisticRouteBelief`
  - control state: `ReducedBeliefSummary`, `LocalOrderParameter`, `MeanFieldState`, `ControllerState`, `RegimeState`, `PostureState`, `ScoredContinuationSet`
  - publication/public-observable state: `CorridorEnvelopeProjection`, `PublishedCandidate`, `AdmittedCandidate`
  - lifecycle state: `LifecycleRoute`
  - execution state: `AsyncState`, `RetentionState`, `EndToEndState`, `RuntimeState`

- projection taxonomy:
  - protocol projection: choreography/session structure -> local protocol surface (`Field/Protocol/*`)
  - local public projection: local field semantics -> corridor/public observable surface (`Field/Model/*`, `Field/Information/*`, `Field/Model/Boundary.lean`)
  - retention projection: controller/runtime signals -> payload-custody decisions and retained-token execution state (`Field/Retention/*`)
  - runtime projection / adequacy reduction: runtime artifacts or runtime state -> reduced Lean protocol/router/system surface (`Field/Adequacy/*`)

- truth ladder:
  - posterior confidence is local/private semantics
  - reduced summary and local order parameter are controller-facing reduced semantics, not public truth
  - canonical route is router-owned truth
  - quality is exported-view comparison
  - adequacy is a semantic bridge into reduced system/router layers, not a truth owner
  - negative boundaries kept explicit: quality is not truth, posterior confidence is not truth, projection is not installation, adequacy is not semantic ownership

- classical versus distributed split:
  - local quantitative/classical surfaces live primarily in `Field/Model/*` and `Field/Information/*`
  - distributed/profile-envelope surfaces live primarily in `Field/Async/*`, `Field/System/*`, and packaged assumption families
  - bridge theorems connecting local order-parameter interpretation to system convergence should state that boundary explicitly

- semantic versus proof-artifact split:
  - semantic core objects: runtime artifacts, runtime states, lifecycle routes, canonical selectors, probabilistic beliefs
  - theorem packaging: contract unlock theorems, boundary forwarding theorems, refinement wrappers
  - synthetic fixtures: adequacy fixture files and probabilistic fixture files

- docs:
  - `Field/Docs/Model.md`
    - local-model specification, stored posterior-to-reduction boundary, order-parameter interpretation, corridor coarse-graining story, and the explicit note that deferred payload retention stays runtime-facing rather than entering the local model
  - `Field/Docs/Protocol.md`
    - protocol, Telltale mapping, and replay/authority notes
  - `Field/Docs/Adequacy.md`
    - runtime artifact bridge, reduced runtime state/step layer, refinement ladder, and semantic-versus-fixture split in the adequacy stack
  - `Field/Docs/Guide.md`
    - contributor guidance, maturity summary, ownership rules, convergence assumptions, and stack-wide harmonization notes

- probabilistic scope:
  - modeled: route existence, route quality, transport reliability, and observation noise
  - explicitly separate from that scope: support ranking, exported quality views, and runtime extraction convenience layers
  - the current posterior-based router objectives are confidence-threshold routing plus reduced expectation / cost / risk / regret objects in `Field/Router/Probabilistic.lean`; they coexist with the older support-owned canonical selectors and are not implied by exported route views or support ranking unless a theorem says so
  - current Bayesian theorems are for the factorized likelihood model in `Field/Information/Bayesian.lean`; correlated evidence remains boundary-marked unless a replacement theorem says otherwise
  - current calibration/soundness results are confidence-threshold validity, posterior-probability equalities for the normalized update, expected-utility bounds, regret interpretation, explicit-evidence posterior support, produced-candidate latent-mass soundness, and a bounded public-projection weakening theorem; broad correlated calibration still remains out of scope
  - current GF1-style non-claims remain explicit: stronger divergence/update inequalities over the reduction, sharper mutual-information bounds for public observables, and information-theoretic optimality claims for the controller-facing summary are still open
  - explicit non-goals for the current probabilistic roadmap: arbitrary continuous distributions, unproved calibration claims, and full production-runtime probabilistic fidelity

## Maturity Snapshot

- most mature:
  - local boundedness/harmony/honesty theorems
  - reduced private protocol and observational boundary
- moderate:
  - reduced finite network, publication, admission, installation, and lifecycle semantics
  - first network-level safety theorems
  - reduced async semantics, transport lifecycle lemmas, and first async safety theorems
  - router-owned canonical selection over lifecycle routes
  - system-level aggregate summaries, reduced end-to-end safety/observer theorems, reliable-immediate convergence results, and canonical-router refinement
  - first silence-only bounded-dropout resilience theorems
  - reduced route-comparison / ranking semantics and support-only reference refinement above system-facing lifecycle outputs
  - projected reduced runtime/system refinement to router-owned canonical truth
  - runtime/system safety-preservation theorems and proof-facing fixture cases
  - probability-simplex information layer
  - normalized public-projection blindness bridge
  - one-step decision layer
  - reduced protocol-machine fragment
- earliest:
  - stronger extracted-Rust runtime correctness theorem beyond the reduced simulation bridge and projected runtime/system refinement
  - convergence beyond the reliable-immediate / empty-queue / unchanged-network regime
  - stronger global routing optimality theorem beyond the current router-owned support and support-then-hop selectors and their reduced system refinements
  - deeper Telltale-native reuse of conservation and subtype-replacement families
