# Field Verification Code Map

This map describes the current organization of `verification/Field`.

## Top-Level Theorem Packs

- `Field/Architecture.lean`
  - shared taxonomy for projection kinds, refinement-ladder stages, route/evidence/selector lineage, and semantic-versus-proof-artifact roles
- `Field/CostAPI.lean`
  - shared work-unit and budget vocabulary reused by router, system, and adequacy cost packs

- `Field/LocalModel.lean`
  - imports the local observer-controller model, the finite-belief information layer, and the first decision procedure
- `Field/PrivateProtocol.lean`
  - imports the reduced private choreography/runtime layer and the Telltale-shaped protocol bridge
- `Field/Boundary.lean`
  - imports the observational controller-boundary theorems
- `Field/Adequacy.lean`
  - imports the Rust-runtime adequacy bridge, low-level runtime-to-canonical alignment theorems, stronger projected runtime/system refinement theorems, runtime-state execution refinement theorems, runtime/system safety-preservation results, probabilistic preservation theorems, first budgeted-optimality preservation theorems, and proof-facing fixture cases
- `Field/Network.lean`
  - imports the reduced finite network layer and its first safety theorems
- `Field/Router.lean`
  - imports the reduced publication, admission, installation, lifecycle, canonical-selection, and posterior-decision layers
- `Field/Async.lean`
  - imports the reduced async delivery semantics, transport lifecycle lemmas, and first async safety theorems
- `Field/System.lean`
  - imports system-level summaries, reduced end-to-end semantics, probabilistic evidence-flow theorems, refinement to router-owned canonical selection above the async layer, and the first budgeted/reduced-context optimality theorems
- `Field/Quality.lean`
  - imports the reduced routing-quality / comparison, reference-best, and support-only refinement layer above the router and system boundaries
- `Field/Field.lean`
  - umbrella import for the whole current field verification stack

## Local Model

- `Field/Model/API.lean`
  - semantic state vocabulary, explicit `ReducedBeliefSummary` reduction boundary, explicit `LocalOrderParameter` vocabulary, abstract round-step operations, and boundedness/harmony laws
- `Field/Model/Instance.lean`
  - first bounded concrete realization, structural theorems, temporal theorems, the Bayesian posterior companion view, the executable posterior-reduction boundary, explicit order-parameter extraction, explicit control-fusion step from reduced summary into mean-field state, and regime classification over that order-parameter surface
- `Field/Model/Refinement.lean`
  - reduction-preservation, order-parameter preservation, sufficiency, conservativity, boundedness/monotonicity, and exogenous-control-dependence theorems for the controller-facing summary, plus the composed-round honesty/refinement pack
- `Field/Model/Decision.lean`
  - one-step finite exploration / decision procedure over a small evidence alphabet

## Information Layer

- `Field/Information/API.lean`
  - abstract probability-simplex style normalization and information-theoretic operations over `FiniteBelief`
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
  - abstract Rust-runtime artifact boundary, reduced router-facing runtime projection, reduced probabilistic slice, reduced runtime-to-trace simulation witness, and adequacy-side projection/lineage taxonomy hooks
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
- `Field/Adequacy/Refinement.lean`
  - runtime-state / system-state refinement relation, stuttering preservation of that relation under reduced runtime steps, and quiescent runtime-state consequences for canonical outcomes and first safety-preservation theorems; the semantic runtime-state object stays distinct from theorem-pack packaging and fixtures
- `Field/Adequacy/Safety.lean`
  - runtime/system reduction-soundness results for support conservativity, no false explicit-path promotion, no route creation from silence, admissible lifecycle origin, and quiescent observational equivalence
- `Field/Adequacy/Fixtures.lean`
  - proof-facing reduced runtime fixture cases covering canonical support selection, stronger router-selection tie handling, empty-runtime silence, one explicit non-claim scenario, and a small fixture-generation path from runtime artifacts or projected system states into proof-facing fixture objects
- `Field/Adequacy/ProbabilisticFixtures.lean`
  - proof-facing probabilistic fixtures covering explicit-evidence posterior support, correlated-evidence boundary marking, miscalibrated-likelihood divergence, and a sparse-evidence confidence guardrail
- `Field/Adequacy/Instance.lean`
  - first concrete runtime extraction, execution-level observational trace theorem, reduced simulation theorem, router-projection honesty facts, and evidence-agreement theorems
- `Field/AssumptionCore.lean`
  - proof-contract vocabulary and default/strengthened contract builders for semantic, protocol-envelope, runtime-envelope, transport, participation, refinement, budget, and regime-profile assumption families
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
  - router-facing publication candidates, publication-lineage vocabulary, and publication honesty / well-formedness theorems
- `Field/Router/Selector.lean`
  - shared selector-family abstraction for lifecycle-route selection, covering candidate domain, eligibility filtering, and fold-based best-route extraction
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
  - only explicit support/canonical refinement theorems connect `Field/Quality` objectives back to router-owned truth; all other ranking objectives remain observational unless a theorem says otherwise

- current seam notes:
  - `PosteriorState`, `ReducedBeliefSummary`, and the Bayesian belief bridge are now explicit; the remaining local-model seam is that the reduced summary is still an intermediate object rather than a stored component of `LocalState`
  - `compressMeanFieldImpl` now owns only control fusion from `ReducedBeliefSummary` plus exogenous `controllerPressure`, instead of hiding posterior reduction internally
  - `Field/Model/Refinement.lean` now makes the intended theorem boundary explicit: the reduced summary is sufficient for the mean-field/controller surfaces only under fixed exogenous control inputs, and the theorem pack also records that the reduction alone does not determine the whole downstream control path
  - `LocalOrderParameter` is now the explicit local phase/order-parameter surface between posterior reduction and control fusion
  - `projection` is still overloaded across protocol projection, local public projection, and runtime/adequacy projection; file ownership is clean, but the shared taxonomy is still documentation-level rather than API-level
  - the corridor/coarse-graining story is present across `Field/Information/*` and `Field/Model/*`, but retained aggregates, public macrostates, and controller-facing reduction are still not one explicit end-to-end interface

- state taxonomy:
  - epistemic state: `FiniteBelief`, `PosteriorState`, `ProbabilisticRouteBelief`
  - control state: `ReducedBeliefSummary`, `MeanFieldState`, `ControllerState`, `RegimeState`, `PostureState`, `ScoredContinuationSet`
  - publication/public-observable state: `CorridorEnvelopeProjection`, `PublishedCandidate`, `AdmittedCandidate`
  - lifecycle state: `LifecycleRoute`
  - execution state: `AsyncState`, `EndToEndState`, `RuntimeState`
  - current caveat: `ReducedBeliefSummary` is the posterior-derived summary object, but it is not yet stored as a first-class component of `LocalState`

- projection taxonomy:
  - protocol projection: choreography/session structure -> local protocol surface (`Field/Protocol/*`)
  - local public projection: local field semantics -> corridor/public observable surface (`Field/Model/*`, `Field/Information/*`, `Field/Model/Boundary.lean`)
  - runtime projection / adequacy reduction: runtime artifacts or runtime state -> reduced Lean protocol/router/system surface (`Field/Adequacy/*`)
  - current caveat: the code now documents these as distinct kinds, but several function/theorem names still use plain `projection` without the kind encoded directly in the identifier

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

- probabilistic scope:
  - modeled: route existence, route quality, transport reliability, and observation noise
  - explicitly separate from that scope: support ranking, exported quality views, and runtime extraction convenience layers
  - the current posterior-based router objectives are confidence-threshold routing plus reduced expectation / cost / risk / regret objects in `Field/Router/Probabilistic.lean`; they coexist with the older support-owned canonical selectors and are not implied by exported route views or support ranking unless a theorem says so
  - current Bayesian theorems are for the factorized likelihood model in `Field/Information/Bayesian.lean`; correlated evidence remains boundary-marked unless a replacement theorem says otherwise
  - current calibration/soundness results are confidence-threshold validity, posterior-probability equalities for the normalized update, expected-utility bounds, regret interpretation, explicit-evidence posterior support, produced-candidate latent-mass soundness, and a bounded public-projection weakening theorem; broad correlated calibration still remains out of scope
  - current GF1-style non-claims remain explicit: stronger divergence/update inequalities over the reduction, sharper mutual-information bounds for public observables, and information-theoretic optimality claims for the controller-facing summary are still open
  - explicit non-goals for the current probabilistic roadmap: arbitrary continuous distributions, unproved calibration claims, and full production-runtime probabilistic fidelity

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
