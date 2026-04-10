# Field Verification Guide

## What This Stack Covers

The field verification stack currently has eleven proof surfaces:

- the deterministic local observer-controller model in `Field/Model`
- the information layer built on top of the finite belief object in `Field/Information`
- the private cooperative summary-exchange protocol in `Field/Protocol`
- the protocol-to-controller observational boundary in `Field/Model/Boundary`
- the reduced finite network semantics in `Field/Network`
- the reduced router-facing publication/admission/installation semantics in `Field/Router`
- the reduced async delivery layer in `Field/Async`
- the system-level summary and boundary layer in `Field/System`
- the reduced routing-quality / comparison layer in `Field/Quality`
- the runtime artifact bridge in `Field/Adequacy`
- the packaged proof-contract / assumption layer in `Field/Assumptions`

These surfaces are intentionally separated. The local controller is not a choreography. The private protocol does not own canonical route truth. The adequacy layer does not get to claim more than the runtime artifact boundary actually supports.

## Current Module Map

- `Docs/Model.md`
  - local state space, unified round semantics, information interpretation, decision layer, and blindness story
- `Docs/Protocol.md`
  - reduced choreography, protocol-machine surface, Telltale alignment, conservation/coherence/refinement story
- `Field/Boundary.lean`
  - observational boundary from protocol outputs / semantic objects into controller-visible evidence
- `Field/Network/*`
  - finite node/destination state, synchronous round buffer, and first network safety theorems
- `Field/Router/*`
  - router-facing publication, admission, installation, lifecycle boundary, and router-owned canonical route-selection spec
- `Field/Async/*`
  - reduced async delivery semantics, transport lifecycle lemmas, explicit delay/loss/retry assumptions, and first async safety theorems
- `Field/System/*`
  - aggregate system summaries, reduced end-to-end semantics, convergence theorems, canonical-router refinement theorems, and cross-layer boundary statements above the async model
- `Field/Quality/*`
  - reduced route-comparison views, reference-best semantics, destination-filtered ranking, support-only refinement, and system-facing quality theorems above the lifecycle view
- `Docs/Adequacy.md`
  - runtime artifact boundary, reduced router projection, reduced simulation witness, low-level runtime alignment, stronger projected runtime/system refinement, packaged assumptions, and parity-sensitive surfaces
- `Docs/Guide.md`
  - contributor guide and current maturity summary

The code map for the whole feature tree lives in `verification/Field/CODE_MAP.md`.

## What Is Proved Today

### Local Model

The local model currently gives:

- boundedness theorems for the destination-local state
- harmony theorems connecting posterior, mean-field, controller, regime, posture, scores, and public projection
- honesty theorems preventing public explicit-path claims without the right local knowledge
- small temporal theorems over repeated rounds
- refinement lemmas showing the composed round still keeps projection/support/knowledge subordinate to the round-updated posterior state
- one finite decision procedure over a representative evidence alphabet

### Information Layer

The information layer currently gives:

- a finite hypothesis space `FieldHypothesis`
- a finite belief object `FiniteBelief`
- a probability-simplex style wrapper `ProbabilitySimplexBelief`
- a concrete weight-normalized distribution with zero-mass fallback behavior
- first mass and entropy theorems
- a first public-projection blindness / erasure theorem
- first quantitative helpers such as `beliefL1Distance` and `localUncertaintyPotential`

This is no longer only a bounded surrogate story, but it is still an early information layer rather than a full probabilistic routing theory.

### Boundary Layer

The boundary layer currently gives:

- a compact adapter from `ProtocolOutput` into bounded `EvidenceInput`
- a trace-level adapter from `ProtocolSemanticObject` into controller-visible evidence
- theorems showing protocol exports stay observational-only at the controller boundary
- replay-style equal-export / equal-trace lemmas for controller evidence batches
- a fail-closed result showing failed-closed protocol snapshots produce no controller evidence

### Private Protocol

The protocol layer currently gives:

- a reduced global choreography
- controller and neighbor projections
- bounded machine stepping
- fail-closed cancellation
- observational-only export
- field-side conservation and coherence theorem packs
- a narrow receive-refinement hook aligned to Telltale subtype-replacement style
- an explicit statement that the current reduced protocol has no reconfiguration semantics

### Network And Router Layers

The network/router layers currently give:

- a finite node vocabulary and finite destination-class vocabulary
- a reduced synchronous round buffer that republishes one public corridor projection per sender/destination slot
- an explicit delivered-message view that can later be refined by async delivery semantics
- router-facing publication candidates that are still distinct from canonical route truth
- reduced observed/admitted/rejected admission semantics
- a minimal installed-route object that only exists above admission
- a reduced lifecycle object with observed/admitted/installed/withdrawn/expired/refreshed status
- a router-owned canonical support selector over eligible lifecycle routes, with support-best witness theorems
- first safety theorems showing:
  - local projection honesty lifts to published candidates
  - explicit-path installation cannot appear without explicit local knowledge
  - installed support remains conservative with respect to the supporting node's local evidence
- lifecycle maintenance theorems showing withdrawal / expiry do not strengthen claims and unchanged refreshes preserve shape/support conservativity

### Async Layer

The current network object is deliberately synchronous, and it is now paired with a first reduced async refinement:

- protocol layer
  - private summary exchange, blocked receives, replay-visible protocol traces, and protocol-machine side conditions
- network layer
  - synchronous publication buffer and neighbor-indexed delivered-message view
- async layer
  - in-flight envelopes, explicit delay/loss/retry assumptions, ready-message draining, transport lifecycle lemmas, and first publication-safety lemmas over the queue
- system layer
  - reduced end-to-end state combining async transport and router lifecycle state
  - a reduced end-to-end step that sequences transport progression, ready delivery, installation, and lifecycle maintenance
  - first theorems showing:
    - `produced_candidate_requires_explicit_sender_knowledge`
    - `produced_candidate_support_conservative`
    - `candidate_view_fixed_point_under_reliable_immediate_empty`
    - `candidate_view_iterate_stable_under_reliable_immediate_empty`
    - `no_spontaneous_explicit_path_promotion_over_iterated_steps`
- adequacy layer
  - correspondence between Rust-facing runtime artifacts, reduced traces, fragment traces, and controller-visible evidence

### Quality Layer

The quality layer currently gives:

- a reduced `RouteComparisonView` extracted from lifecycle-managed routes
- admissibility rules that only compare active installed/refreshed routes for one destination
- a reference admissibility and support-best semantics over the same exported route-view surface
- a small comparison-object vocabulary:
  - `supportDominance`
  - `hopBandConservativity`
  - `stableTieBreak`
  - `supportThenHopThenStableTieBreak`
- pairwise comparison objects that return only left/right/tie/inadmissible, never canonical route truth
- destination-filtered best-view selection over lifecycle/system-facing routes
- a support-only reference selector `referenceBestRouteView` and refinement theorems:
  - `bestRouteView_supportDominance_eq_referenceBestRouteView`
  - `bestRouteView_supportDominance_refines_reference`
  - `bestSystemRouteView_supportDominance_eq_referenceBestSystemRouteView`
  - `bestSystemRouteView_supportDominance_refines_reference`
- system-facing theorems showing:
  - `best_system_route_view_stable_under_reliable_immediate_empty`
  - `best_system_route_view_cannot_manufacture_explicit_path`
  - `best_system_route_view_support_conservative`
  - `best_system_route_view_explicit_path_requires_explicit_sender_knowledge`
- explicit boundary/counterexample theorems showing the non-support objectives remain reduced:
  - `stableTieBreak_can_prefer_lower_support_view`
  - `hopBandConservativity_can_prefer_lower_support_view`

### Router Canonical Truth

The router layer now also gives:

- a router-owned `CanonicalRouteEligible` predicate over lifecycle routes
- a router-owned support selector `canonicalBestRoute`
- a canonical support-best witness `CanonicalSupportBest`
- well-formedness theorems showing the canonical selector only returns eligible destination-local router routes
- support-best theorems such as:
  - `canonicalBestRoute_some_is_support_best`

This is the current owner of canonical route truth in the reduced stack. It lives in `Field/Router`, not in `Field/Quality`.

### System Statistics And Boundary

The system layer also currently gives:

- aggregate support summaries such as `aggregateSupport` and `averageSupport`
- ready-message support accounting through `readySupportMass`
- bounds such as `aggregateSupport_bounded`, `averageSupport_bounded`, and `ready_support_mass_bounded_by_inflight_budget`
- refinement theorems such as:
  - `canonicalSystemRoute_eq_router_canonical_under_reliable_immediate_empty`
  - `canonical_system_route_stable_under_reliable_immediate_empty`
  - `bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView`
- explicit boundary theorems `support_optimality_contract_does_not_claim_canonical_router_refinement_ready`, `canonical_router_contract_unlocks_canonical_router_refinement`, and `canonical_router_contract_still_does_not_claim_global_optimality_ready` stating that the stronger canonical-router contract unlocks only the current router-owned support refinement, not full global optimality

### Adequacy And Assumptions

The adequacy and assumptions layers currently give:

- reduced runtime artifacts
- extraction to reduced machine snapshots and traces
- evidence agreement between Rust-facing artifacts and Lean traces
- an explicit reduced simulation witness
- a stronger fragment-trace refinement theorem for runtime executions
- a packaged `ProofContract` for semantic, protocol, runtime, and optional strengthening assumptions
- a split assumptions layer where `Field/AssumptionCore.lean` owns the contract vocabulary, `Field/AssumptionTheorems.lean` owns the theorem packaging, and `Field/Assumptions.lean` stays a thin umbrella
- contract-level bridge theorems such as:
  - `contract_yields_runtime_evidence_agreement`
  - `contract_yields_observational_controller_boundary`
  - `contract_yields_protocol_trace_admitted`
  - `contract_yields_runtime_trace_simulation`
  - `contract_yields_reduced_quality_stability`
  - `contract_yields_reduced_quality_support_conservativity`
  - `contract_yields_explicit_path_quality_observer`
  - `contract_yields_support_optimality_refinement`
  - `contract_yields_canonical_router_refinement`
  - `contract_yields_runtime_canonical_refinement`
  - `contract_yields_runtime_system_canonical_refinement`

The runtime-canonical path is now explicit:

- Rust/runtime artifacts carry a reduced router-facing lifecycle projection
- `Field/Adequacy/Canonical.lean` relates that projection to the reduced system lifecycle view through `RuntimeSystemCanonicalAligned`
- `Field/Adequacy/Projection.lean` proves a reduced runtime artifact stream generated from `systemStep` satisfies that alignment and is admitted by the existing reduced runtime envelope
- under that stronger projected-runtime path, runtime canonical selection agrees with the same router-owned canonical selector without any extra alignment parameter

## Convergence Assumptions

The current convergence theorems are intentionally strong-hypothesis results, not ambient liveness claims.

They rely on:

- `reliableImmediateAssumptions`
  - `maxDelay = 0`
  - `retryBound = 0`
  - `lossPossible = False`
- an empty initial in-flight queue
- unchanged local/network state across the reduced end-to-end step, exposed by `system_step_preserves_network`

Under exactly that regime, the current theorems show a reduced fixed-point story for the installed candidate view. They do not claim convergence under arbitrary delay, retry, or loss behavior.

## Safety, Canonical Refinement, And Reduced Ranking

The new end-to-end and convergence results are still safety/stability theorems.

They show that:

- explicit-path installation still traces back to explicit local knowledge
- installed support remains conservative with respect to sender-local support
- the candidate view stabilizes under reliable-immediate transport with no queued backlog
- repeated end-to-end steps do not spontaneously promote to explicit-path when no sender has explicit-path knowledge

The router and quality layers now split the truth story.

They show that:

- router-owned lifecycle objects admit a canonical support selector
- `bestSystemRouteView .supportDominance` agrees with that router-owned canonical selector
- pairwise and destination-filtered comparisons depend only on exported lifecycle route fields
- quality results stay conservative with respect to installed support and sender-local knowledge
- reliable-immediate stable-input regimes produce stable reduced ranking outcomes
- `supportDominance` agrees both with a reference support-best semantics and with the current router-owned canonical support selector

The distinction matters:

- safety/conservativity theorems say exported route views do not strengthen shape/support/knowledge claims
- stabilization/fixed-point theorems say the installed candidate view and its reduced rankings stop changing under the reliable-immediate empty-queue regime
- canonical-router refinement says the current support-only system winner agrees with the router-owned canonical selector
- low-level runtime-to-canonical refinement says an admitted runtime artifact stream with explicit reduced lifecycle alignment agrees with that same router-owned canonical selector
- stronger projected runtime/system refinement says the reduced runtime artifact stream generated from `systemStep` agrees with that same router-owned canonical selector without a free alignment parameter
- reduced comparison/ranking theorems say exported candidates can be compared or selected without turning them into canonical route truth
- support-only refinement says `supportDominance` matches a reference support-max view, but only for that objective
- full routing optimality would require a stronger objective story than the current router-canonical support selector and its refinement provide

They do not show:

- best-route selection for the non-support objectives
- path-quality optimality
- asymptotic convergence under realistic transport dynamics
- equivalence to the production Rust router/runtime

## What Is Not Proved

The current stack does not prove:

- global routing optimality
- router-owned canonical route correctness
- full Rust controller correctness
- full Rust choreography runtime correctness
- transport correctness
- full asynchronous transport correctness
- large asymptotic mean-field or fluid-limit theorems

The current system is best read as:

- a strong reduced local-model and protocol-boundary proof stack
- an early but real information-theoretic layer
- a router-owned canonical selector plus a reduced ranking/comparison layer above system-facing lifecycle outputs
- a reduced runtime simulation bridge
- a stronger projected runtime/system canonical-refinement theorem above that lower-level alignment lemma
- a reduced end-to-end safety/stability model under explicit assumptions

## Maturity Summary

| Area | Status | Notes |
|---|---|---|
| Local model boundedness, harmony, honesty | Stable | main reduced semantic object is in place |
| Private protocol projection and observational boundary | Stable | reduced but structurally coherent |
| Conservation and coherence packs | Moderate | partly direct-family style, partly field-local glue |
| Receive refinement | Moderate | narrow subtype-replacement shaped result exists |
| Information layer | Moderate | finite normalized belief object and first blindness theorem exist |
| Boundary layer | Moderate | protocol-export-to-controller-evidence boundary is explicit and observational-only |
| One-step decision layer | Early | useful but intentionally small |
| Reduced network and router layers | Moderate | explicit publication/admission/installation/lifecycle boundary, router-owned canonical support selector, and first safety theorems exist |
| Reduced async layer | Moderate | explicit delay/loss/retry assumptions, transport lifecycle lemmas, and first async publication safety theorems exist |
| System summaries and boundaries | Moderate | aggregate support summaries, reduced end-to-end safety/observer theorems, reliable-immediate stabilization results, and canonical-router refinement exist |
| Quality layer | Moderate | reduced route-comparison semantics, support-only reference refinement, and first system-facing ranking stability/conservativity theorems exist |
| Runtime adequacy | Early | reduced simulation witness, low-level runtime-to-canonical alignment theorem, and stronger projected runtime/system refinement exist, but not full extracted-Rust runtime refinement |
| Packaged assumptions | Early | structure is in place and exports useful bridge lemmas, including projected runtime/system refinement, but theorem dependence is still selective |

## Ownership Rules

When adding new proofs, keep these boundaries intact.

Short version:

- router owns canonical truth
- quality compares exported views
- adequacy owns reduction and runtime projection
- assumptions package contracts and theorem access instead of re-owning lower-layer logic

- If the statement is about posterior, regime, posture, scores, or corridor projection, it belongs in `Field/Model` or `Field/Information`.
- If the statement is about choreography, projection, blocked receives, semantic objects, or protocol traces, it belongs in `Field/Protocol`.
- If the statement is about protocol outputs or semantic objects becoming controller-visible evidence, it belongs in `Field/Model/Boundary`.
- If the statement is about node-indexed local states, reduced message delivery, or network-level safety, it belongs in `Field/Network`.
- If the statement is about router-facing publication, admission, installation, lifecycle maintenance, or canonical handling eligibility, it belongs in `Field/Router`.
  Router-owned canonical route truth belongs there too.
- If the statement is about in-flight envelopes, delay, retry, ready delivery, or async publication safety, it belongs in `Field/Async`.
- If the statement is about end-to-end sequencing, installed-route observer results, fixed points, or cross-layer proof-boundary summaries above the async model, it belongs in `Field/System`.
- If the statement is about comparing, ranking, or selecting between exported lifecycle/system route views, it belongs in `Field/Quality`.
  Support-only reference-best semantics and refinement stay in `Field/Quality` too.
- If the statement is about Rust-facing runtime artifacts, extracted traces, or runtime simulation, it belongs in `Field/Adequacy`.
  Runtime-to-canonical alignment and projected runtime/system refinement theorems belong there too.
- If the statement is about the global assumption contract used across theorem packs, it belongs in `Field/Assumptions`.

## How To Extend The Stack

Use the API/instance pattern consistently.

1. Decide whether the new concept is an abstract interface or a first concrete realization.
2. Put proof-facing vocabulary and laws in the API file first if downstream proofs should depend on an abstraction.
3. Put the first concrete realization and instance-level proofs in the companion instance file.
4. Only then update downstream theorem packs.
5. Update the relevant field doc if the new result changes the public mental model of the stack.

## Telltale Reuse Discipline

The current field stack is Telltale-aligned, not fully Telltale-derived.

That means:

- use Telltale theorem-family structure where it genuinely fits
- do not restate Telltale theorems under field names unless the field theorem is genuinely narrower
- keep field-local glue explicit when the full generic Telltale theorem is not yet being instantiated directly
- do not overclaim proof reuse

`Docs/Protocol.md` is the authoritative place for the current Telltale alignment story.

## Contributor Checklist

Before landing a meaningful field-proof change:

1. Build the field root:
   - `nix develop --command bash -lc 'cd verification && lake build Field.Field'`
2. Check that the docs still match the code.
3. Update `verification/Field/CODE_MAP.md` if module responsibilities moved.
4. If the change affects the overall roadmap, update `work/lean.md` or `work/lean2.md`.
5. If the change affects Rust/Lean parity, update `Docs/Adequacy.md`.

## What To Avoid

- Do not move router-owned canonical truth into the protocol proof object.
- Do not force the deterministic controller into a choreography encoding.
- Do not claim full runtime adequacy when the actual theorem is a reduced simulation witness.
- Do not claim full extracted-Rust runtime correctness when the actual theorem is only a projected reduced runtime/system refinement.
- Do not forget that the lower-level alignment theorem still needs an explicit hypothesis when used directly.
- Do not describe the reliable-immediate convergence lemmas as routing-quality or optimality results.
- Do not let the reduced quality layer smuggle in canonical route truth or production-optimality claims.
- Do not let canonical-router refinement be restated as full global optimality.
- Do not describe the support-only refinement theorem as full route optimality for tie-break or hop-band objectives.
- Do not introduce transport-specific details into the local controller model unless they are genuinely proof-relevant there.
- Do not bypass the API/instance split just to make one downstream theorem shorter.
