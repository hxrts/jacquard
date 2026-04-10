# Field Verification Code Map

This map describes the current organization of `verification/Field`.

## Top-Level Theorem Packs

- `Field/LocalModel.lean`
  - imports the local observer-controller model, the finite-belief information layer, and the first decision procedure
- `Field/PrivateProtocol.lean`
  - imports the reduced private choreography/runtime layer and the Telltale-shaped protocol bridge
- `Field/Boundary.lean`
  - imports the observational controller-boundary theorems
- `Field/Adequacy.lean`
  - imports the Rust-runtime adequacy bridge, low-level runtime-to-canonical alignment theorems, stronger projected runtime/system refinement theorems, runtime-state execution refinement theorems, runtime/system safety-preservation results, first budgeted-optimality preservation theorems, and proof-facing fixture cases
- `Field/Network.lean`
  - imports the reduced finite network layer and its first safety theorems
- `Field/Router.lean`
  - imports the reduced publication, admission, installation, lifecycle, and canonical-selection layers
- `Field/Async.lean`
  - imports the reduced async delivery semantics, transport lifecycle lemmas, and first async safety theorems
- `Field/System.lean`
  - imports system-level summaries, reduced end-to-end semantics, convergence theorems, refinement to router-owned canonical selection above the async layer, and the first budgeted/reduced-context optimality theorems
- `Field/Quality.lean`
  - imports the reduced routing-quality / comparison, reference-best, and support-only refinement layer above the router and system boundaries
- `Field/Field.lean`
  - umbrella import for the whole current field verification stack

## Local Model

- `Field/Model/API.lean`
  - semantic state vocabulary, abstract round-step operations, boundedness/harmony laws
- `Field/Model/Instance.lean`
  - first bounded concrete realization, structural theorems, temporal theorems, and first quantitative ranking law
- `Field/Model/Decision.lean`
  - one-step finite exploration / decision procedure over a small evidence alphabet

## Information Layer

- `Field/Information/API.lean`
  - abstract probability-simplex style normalization and information-theoretic operations over `FiniteBelief`
- `Field/Information/Instance.lean`
  - first concrete probability-simplex belief object, weight-normalized distribution, and entropy/mass theorems
- `Field/Information/Blindness.lean`
  - field-side information-cost / blindness bridge over the normalized public projection, including a first erasure theorem

## Private Protocol

- `Field/Protocol/API.lean`
  - reduced protocol roles, labels, machine state, global choreography, abstract projection/step/export laws
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
- `Field/Protocol/Reconfiguration.lean`
  - fixed-participant audit note proving the current reduced protocol has no reconfiguration semantics

## Boundary And Adequacy

- `Field/Model/Boundary.lean`
  - controller-evidence boundary from protocol exports and traces
- `Field/Adequacy/API.lean`
  - abstract Rust-runtime artifact boundary, reduced router-facing runtime projection, and reduced runtime-to-trace simulation witness
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
- `Field/Adequacy/Refinement.lean`
  - runtime-state / system-state refinement relation, stuttering preservation of that relation under reduced runtime steps, and quiescent runtime-state consequences for canonical outcomes and first safety-preservation theorems
- `Field/Adequacy/Safety.lean`
  - runtime/system reduction-soundness results for support conservativity, no false explicit-path promotion, no route creation from silence, admissible lifecycle origin, and quiescent observational equivalence
- `Field/Adequacy/Fixtures.lean`
  - proof-facing reduced runtime fixture cases covering canonical support selection, stronger router-selection tie handling, empty-runtime silence, one explicit non-claim scenario, and a small fixture-generation path from runtime artifacts or projected system states into proof-facing fixture objects
- `Field/Adequacy/Instance.lean`
  - first concrete runtime extraction, execution-level observational trace theorem, reduced simulation theorem, router-projection honesty facts, and evidence-agreement theorems
- `Field/AssumptionCore.lean`
  - proof-contract vocabulary and default/strengthened contract builders for semantic, protocol-envelope, runtime-envelope, and optional refinement assumptions
- `Field/AssumptionTheorems.lean`
  - theorem packaging layer deriving adequacy, quality, canonical-router, runtime-canonical, runtime-state execution refinement, and resilience-boundary consequences from the shared proof-contract vocabulary
- `Field/Assumptions.lean`
  - thin umbrella importing the proof-contract vocabulary and theorem-packaging layers

## Network And Router

- `Field/Network/API.lean`
  - finite node/destination vocabulary, synchronous round buffer, delivered-message view, and local-harmony lift
- `Field/Network/Safety.lean`
  - first reduced network safety theorems connecting local honesty to publication, admission, and installation
- `Field/Router/Publication.lean`
  - router-facing publication candidates and publication honesty / well-formedness theorems
- `Field/Router/Admission.lean`
  - reduced observed/admitted/rejected boundary and first admission conservativity theorems
- `Field/Router/Installation.lean`
  - minimal canonical installed-route object and installation honesty theorems
- `Field/Router/Lifecycle.lean`
  - reduced observed/admitted/installed/withdrawn/expired/refreshed lifecycle object plus maintenance and conservativity theorems
- `Field/Router/Canonical.lean`
  - router-owned destination-local canonical support selector over lifecycle routes, plus support-best, eligibility, destination-scope containment, unique-eligible selection, a destination-local sparse-scaling theorem for off-destination route growth, a threshold discontinuity example, and threshold-emergence/disappearance theorems for canonical route truth
- `Field/Router/CanonicalStrong.lean`
  - stronger router-owned support-then-hop-then-stable selector over eligible lifecycle routes, plus membership and eligibility theorems for the stronger canonical surface
- `Field/Router/Cost.lean`
  - proof-facing linear search-cost model for the canonical selector, including worst-case, incremental, stable-input, search-space, and maintenance-invariance bounds
- `Field/Router/Optimality.lean`
  - budgeted support-only canonical search, explicit support-regret vocabulary, anytime monotonicity, deadline-safety, and threshold-region theorems for the current router-owned objective
- `Field/Router/Resilience.lean`
  - first participation-fault vocabulary, silence-only dropout budget, surviving-route projection, bounded-dropout support-stability theorems, and an explicit dishonest-publication non-claim

## Async And System Layers

- `Field/Async/API.lean`
  - reduced async envelopes, explicit delay/retry/loss assumptions, queue stepping, ready-message view, and observer view
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
  - reduced reliable-immediate fixed-point and no-spontaneous-promotion theorems over iterated end-to-end steps
- `Field/System/Canonical.lean`
  - system-facing refinement theorems connecting `supportDominance` winners to the router-owned canonical selector, plus underconnected and unique-eligible sparse cases, thresholded canonical-support theorems, an explicit critical-threshold boundary, canonical support/knowledge conservativity for winners, a threshold-1 vanishing-support limit, reliable-immediate stability, global support-optimum packaging, one-step recovery and bounded-convergence theorems, and a no-oscillation theorem for the canonical system route in the current reliable-immediate bounded-delay corner
- `Field/System/CanonicalStrong.lean`
  - system-facing stronger router selector based on support-then-hop-then-stable lifecycle choice, plus reliable-immediate stability and basic membership/eligibility theorems
- `Field/System/Resilience.lean`
  - system-facing bounded-dropout and bounded-non-participation stabilization/degradation theorems connecting reduced participation loss to canonical support behavior under the clean async regime, including reduced participation-cut and unique-bridge disappearance theorems
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
  - `Field/Quality` compares exported route views
  - `Field/Adequacy` owns reduction and runtime projection
  - `Field/Assumptions` packages contracts and theorem access
  - only explicit support/canonical refinement theorems connect `Field/Quality` objectives back to router-owned truth; all other ranking objectives remain observational unless a theorem says otherwise

- `Field/Docs/Model.md`
  - mathematical description of the local field model, plus its place in the wider field stack
- `Field/Docs/Protocol.md`
  - protocol, Telltale mapping, and replay/authority notes
- `Field/Docs/Adequacy.md`
  - runtime artifact bridge, reduced runtime state/step layer, reduced runtime router projection, low-level alignment theorem, stronger projected runtime/system adequacy note, and the split assumptions-layer packaging note
- `Field/Docs/Guide.md`
  - contributor guidance, maturity summary, router-canonical truth versus quality/comparison scope, convergence assumptions, stack-level module map including the network/router/async/system layers, and the cleaned-up assumptions ownership split

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
  - stronger extracted-Rust runtime correctness theorem beyond the current reduced simulation bridge and projected runtime/system refinement
  - convergence beyond the reliable-immediate / empty-queue / unchanged-network regime
  - stronger global routing optimality theorem beyond the current router-owned support and support-then-hop selectors and their reduced system refinements
  - deeper Telltale-native reuse of conservation and subtype-replacement families
