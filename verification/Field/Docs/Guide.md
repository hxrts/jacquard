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
- `Field/Protocol/Boundary.lean`
  - thin protocol-boundary import surface for higher-layer boundary files
- `Field/Boundary.lean`
  - thin re-export surface for the controller-boundary family rooted in `Field/Model/Boundary.lean`
- `Field/Network/*`
  - finite node/destination state, synchronous round buffer, and first network safety theorems
- `Field/Router/*`
  - router-facing publication, admission, installation, lifecycle boundary, support-owned canonical route-selection spec, a posterior-owned confidence-threshold decision layer, and a stronger support-then-hop canonical selector
- `Field/Async/*`
  - reduced async delivery semantics, transport lifecycle lemmas, explicit delay/loss/retry assumptions, first async safety theorems, and a bounded-delay/bounded-retry theorem pack
- `Field/System/*`
  - aggregate system summaries, reduced end-to-end semantics, probabilistic evidence-flow theorems, canonical-router refinement theorems, and cross-layer boundary statements above the async model
- `Field/Quality/*`
  - reduced route-comparison views, reference-best semantics, destination-filtered ranking, support-only refinement, and system-facing quality theorems above the lifecycle view
- `Docs/Adequacy.md`
  - runtime artifact boundary, reduced router projection, reduced simulation witness, stronger runtime/system safety/refinement story, proof-facing fixtures, packaged assumptions, and parity-sensitive surfaces
- `Docs/Parity.md`
  - maintained Rust/Lean/docs compatibility ledger for field search, runtime,
    replay, and ownership boundaries
- `Docs/Guide.md`
  - contributor guide and current maturity summary

The code map for the whole feature tree lives in `verification/Field/CODE_MAP.md`.

## What Is Proved Today

### Local Model

The local model currently gives:

- boundedness theorems for the destination-local state
- harmony theorems connecting posterior, stored reduced summary, stored order parameter, mean-field, controller, regime, posture, scores, and public projection
- honesty theorems preventing public explicit-path claims without the right local knowledge
- small temporal theorems over repeated rounds
- exact reduction-preservation and compression-boundary theorems for the controller-facing `ReducedBeliefSummary`
- explicit round-state storage theorems showing `LocalState` now stores the reduced summary and order parameter directly, and that mean-field and regime updates consume those stored boundary objects
- sufficient-statistic style theorems showing which downstream control surfaces are determined by the reduced summary once exogenous controller pressure is fixed
- explicit non-sufficiency theorems showing the control path is not determined by the posterior reduction alone
- refinement lemmas showing the composed round still keeps projection/support/knowledge subordinate to the round-updated posterior state
- one finite decision procedure over a representative evidence alphabet

### Information Layer

The information layer currently gives:

- a finite hypothesis space `FieldHypothesis`
- a finite belief object `FiniteBelief`
- a probability-simplex style wrapper `ProbabilitySimplexBelief`
- a concrete weight-normalized distribution with zero-mass fallback behavior
- a richer probabilistic route-hypothesis space over route existence, quality, transport reliability, witness reliability, and local knowledge
- a Bayesian observation layer with explicit priors, likelihood factors, posterior normalization, and impossible-observation fallback
- a calibration/soundness layer centered on confidence-threshold validity, explicit-evidence support, and explicit correlated-regime non-claims
- a factorized likelihood theorem surface that states the current Bayesian model is the product of existence, knowledge, delivery, and witness terms
- first mass and entropy theorems
- a reduction-level blindness / erasure story from posterior state to `ReducedBeliefSummary`
- a public-projection blindness / erasure story from reduced summary and normalized belief to corridor macrostates
- first quantitative helpers such as `beliefL1Distance`, `natGap`, `reducedSupportGap`, `reducedUncertaintyGap`, and `localUncertaintyPotential`

This is no longer only a bounded surrogate story, but it is still an early information layer rather than a full probabilistic routing theory.

The intended probabilistic scope is now explicit:

- modeled probabilistic objects should include route existence, route quality, transport reliability, and observation noise
- Bayesian prior / likelihood / posterior assumptions should be named directly rather than hidden inside a score update
- posterior belief, confidence, expected utility, and exported quality/ranking views are distinct objects and should not be conflated
- support-style ranking remains non-probabilistic unless one theorem explicitly bridges it to posterior-based router truth
- the current Bayesian update is genuinely Bayesian only for the stated factorized prior / likelihood model; smoothed priors and evidence encoding remain reduced approximations of a richer runtime story
- correlated observation regimes are still boundary-marked as out of scope unless a theorem explicitly replaces the factorized likelihood model
- non-goals for the current probabilistic roadmap include arbitrary continuous distributions, unproved calibration claims, and full production-runtime probabilistic fidelity

The current probabilistic theorem surface is also more specific than the generic phrase
"probabilistic routing theory" suggests:

- decision objective:
  - the router-owned probabilistic objectives currently implemented are confidence-threshold routing plus secondary posterior expectation / cost / risk / regret objects in `Field/Router/Probabilistic.lean`
- Bayesian assumptions:
  - the active posterior semantics uses the factorized likelihood model from `Field/Information/Bayesian.lean`
  - the current local/runtime story still uses reduced evidence encoding and smoothed priors
- calibrated / sound today:
  - confidence-threshold decisions satisfy their stated threshold conditions
  - posterior probability equalities are exposed for the current normalized Bayesian update
  - expected-utility bounds and regret interpretation are exposed for the reduced posterior decision objects
  - trusted explicit evidence gives positive posterior mass to the explicit-path hypothesis
  - explicit posterior decisions on produced candidates imply positive latent explicit-path mass
  - the public corridor projection is a bounded weakening of positive-threshold posterior decisions
- still reduced / out of scope:
  - correlated-evidence calibration beyond the explicit boundary marker
  - full probabilistic convergence under broad async regimes
  - full production-runtime probabilistic fidelity
  - KL-style update inequalities and stronger divergence theory over the reduction
  - information-theoretic optimality claims for the local reduction boundary

### Boundary Layer

The boundary layer currently gives:

- a compact adapter from `ProtocolOutput` into bounded `EvidenceInput`
- a trace-level adapter from `ProtocolSemanticObject` into controller-visible evidence
- theorems showing protocol exports stay observational-only at the controller boundary
- replay-style equal-export / equal-trace lemmas for controller evidence batches
- a fail-closed result showing failed-closed protocol snapshots produce no controller evidence
- an explicit ownership split: `Field/Model/Boundary.lean` owns only protocol/controller extraction, while `Field/Adequacy/*` owns runtime-artifact/runtime-state extraction and composes with the controller boundary only after runtime reduction

### Private Protocol

The protocol layer currently gives:

- a reduced global choreography
- controller and neighbor projections
- bounded machine stepping
- fail-closed cancellation
- observational-only export
- field-side conservation and coherence theorem packs
- a narrow receive-refinement hook aligned to Telltale subtype-replacement style
- an explicit reduced protocol reconfiguration surface that remains
  observational-only and fixed-participant

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
- a router-owned posterior confidence-threshold selector over probabilistic belief, with admissibility, determinism, dominance-monotonicity, and non-claim theorems separating posterior truth from exported route views
- secondary posterior expectation / cost / risk / regret objects over the same probabilistic belief state for reduced expected-utility and min-regret reasoning
- a stronger router-owned support-then-hop-then-stable selector over eligible lifecycle routes
- first safety theorems showing:
  - local projection honesty lifts to published candidates
  - explicit-path installation cannot appear without explicit local knowledge
  - installed support remains conservative with respect to the supporting node's local evidence
- lifecycle maintenance theorems showing withdrawal / expiry do not strengthen claims and unchanged refreshes preserve shape/support conservativity
- stronger-selector theorems showing the support-then-hop canonical winner still stays inside the eligible lifecycle surface and remains stable under the reliable-immediate fixed-point regime
- bounded-dropout and bounded-non-participation resilience packs that keep the
  canonical-support story router/system-owned even under reduced participation
  faults

### Async Layer

The current network object is deliberately synchronous, and it is now paired with a first reduced async refinement:

- protocol layer
  - private summary exchange, blocked receives, replay-visible protocol traces, and protocol-machine side conditions
- network layer
  - synchronous publication buffer and neighbor-indexed delivered-message view
- async layer
  - in-flight envelopes, explicit delay/loss/retry assumptions, ready-message draining, transport lifecycle lemmas, first publication-safety lemmas over the queue, and a bounded-delay/bounded-retry no-strengthening / queue-growth / drain-bound theorem pack
- system layer
  - reduced end-to-end state combining async transport and router lifecycle state
  - a reduced end-to-end step that sequences transport progression, ready delivery, installation, and lifecycle maintenance
  - a reduced probabilistic evidence-flow layer that maps async envelopes and lifecycle routes into Bayesian observations, with explicit delayed/lossy/repeated/correlated vocabulary
  - a probabilistic soundness layer showing explicit posterior decisions on produced candidates imply positive latent explicit-path mass, stable repeated evidence preserves posterior-supported choice, dropout degradation stays bounded in the reduced observation-strength model, and sparse evidence must remain explicitly marked as sparse
  - first theorems showing:
    - `produced_candidate_requires_explicit_sender_knowledge`
    - `produced_candidate_support_conservative`
    - `produced_explicit_candidate_requires_positive_explicit_bayesian_mass`
    - `candidate_view_fixed_point_under_reliable_immediate_empty`
    - `candidate_view_iterate_stable_under_reliable_immediate_empty`
    - `no_spontaneous_explicit_path_promotion_over_iterated_steps`
    - `systemStep_inflight_length_bounded_by_current_plus_publications`
    - `systemStep_lifecycle_length_bounded_by_transport_ready_queue`
    - `system_step_route_never_amplifies_source_projection`
- adequacy layer
  - correspondence between Rust-facing runtime artifacts, reduced traces, fragment traces, controller-visible evidence, and a reduced probabilistic leading-evidence view used for confidence-threshold preservation, min-regret decision preservation, and expected-utility order preservation
  - proof-facing probabilistic fixtures for explicit-evidence posterior support, correlated-evidence boundaries, miscalibrated-likelihood boundaries, and sparse-evidence confidence guardrails

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
  - `best_system_route_view_idempotent_under_lifecycle_maintenance`
  - `ready_installed_route_eventually_appears_in_system_destination_views`
- explicit boundary/counterexample theorems showing the non-support objectives remain reduced:
  - `stableTieBreak_can_prefer_lower_support_view`
  - `hopBandConservativity_can_prefer_lower_support_view`

This audit point is deliberate: `Field/Quality` owns exported-view comparison only. In the current stack, only the explicit support/canonical refinement theorems connect a quality objective back to router-owned truth. No other `Field/Quality` objective should be read as canonical unless one theorem states that bridge directly.

### Router Canonical Truth

The router layer now also gives:

- a router-owned `CanonicalRouteEligible` predicate over lifecycle routes
- a router-owned support selector `canonicalBestRoute`
- a stronger support-then-hop-then-stable selector `canonicalBestRouteSupportThenHopThenStableTieBreak`
- a canonical support-best witness `CanonicalSupportBest`
- well-formedness theorems showing the canonical selector only returns eligible destination-local router routes
- support-best theorems such as:
  - `canonicalBestRoute_some_is_support_best`

This is the current owner of canonical route truth in the reduced stack. It lives in `Field/Router`, not in `Field/Quality`. The stronger selector exists now, but the project still does not claim that every observational quality objective has been promoted to router-owned truth.

During the probabilistic migration there are now two distinct router-owned truths:

- support-owned canonical lifecycle selection
- posterior-owned confidence-threshold routing over probabilistic belief

The Rust field lineage that these proof surfaces sit beneath is now explicit:

- local field evidence updates destination-local observer state
- field search selects one private result
- that result yields one public corridor-envelope candidate
- router admission/materialization owns the installed-route truth above that
- field quality/reference objects do not become canonical unless one router
  theorem or router rule says so explicitly

Those are intentionally different objects. The support-owned selector is still about reduced lifecycle support truth. The posterior-owned selector is about a routing decision justified by Bayesian posterior mass. Neither one should be read off exported `Field/Quality` views unless a theorem bridges them.

### System Statistics And Boundary

The system layer also currently gives:

- aggregate support summaries such as `aggregateSupport` and `averageSupport`
- ready-message support accounting through `readySupportMass`
- bounds such as `aggregateSupport_bounded`, `averageSupport_bounded`, and `ready_support_mass_bounded_by_inflight_budget`
- refinement theorems such as:
  - `canonicalSystemRoute_eq_router_canonical_under_reliable_immediate_empty`
  - `canonicalSystemRoute_eq_none_of_no_active_destination_match`
  - `canonicalSystemRoute_eq_some_of_unique_eligible`
  - `canonical_system_route_stable_under_reliable_immediate_empty`
  - `canonical_system_route_no_oscillation_under_reliable_immediate_empty`
  - `canonical_system_route_recovers_within_one_step_under_reliable_immediate_empty`
  - `canonicalSystemSupportAtLeast_of_dominating_route`
  - `not_canonicalSystemSupportAtLeast_of_all_eligible_below_threshold`
  - `canonicalSystemSupportAtLeast_stable_under_reliable_immediate_empty`
  - `bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView`
- explicit boundary theorems `support_optimality_contract_does_not_claim_canonical_router_refinement_ready`, `canonical_router_contract_unlocks_canonical_router_refinement`, and `canonical_router_contract_still_does_not_claim_global_optimality_ready` stating that the stronger canonical-router contract unlocks only the current router-owned support refinement, not full global optimality

### Adequacy And Assumptions

The adequacy and assumptions layers currently give:

- reduced runtime artifacts
- reduced runtime states and one-step runtime execution semantics
- extraction to reduced machine snapshots and traces
- evidence agreement between Rust-facing artifacts and Lean traces
- an explicit reduced simulation witness
- a stronger fragment-trace refinement theorem for runtime executions
- a runtime-state / system-state stuttering refinement layer above the projected-artifact bridge
- a runtime/system safety-preservation layer above the stuttering refinement theorem
- proof-facing runtime fixture cases for canonical outcomes and one explicit non-claim
- a packaged `ProofContract` for semantic, protocol, runtime, and optional strengthening assumptions
- explicit convergence, resilience, and search profile-family accessors over transport, participation, budget, refinement, and regime/profile assumptions
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
  - `contract_yields_runtime_state_system_canonical_refinement`
  - `contract_yields_runtime_state_support_safety`
  - `contract_yields_runtime_state_no_false_explicit_path_promotion`
  - `contract_yields_runtime_state_no_route_creation_from_silence`
  - `contract_yields_runtime_state_admissible_origin`

The runtime-canonical path is now explicit:

- Rust/runtime artifacts carry a reduced router-facing lifecycle projection plus
  reduced search-linkage metadata
- `Field/Adequacy/Canonical.lean` relates that projection to the reduced system lifecycle view through `RuntimeSystemCanonicalAligned`
- `Field/Adequacy/Projection.lean` proves a reduced runtime artifact stream generated from `systemStep` satisfies that alignment and is admitted by the existing reduced runtime envelope
- `Field/Adequacy/Runtime.lean` lifts the artifact story to reduced runtime states and runtime steps
- `Field/Adequacy/Search.lean` carries the combined runtime/search bundle plus
  optional reduced protocol reconfiguration
- `Field/Adequacy/Refinement.lean` defines a runtime-state / system-state stuttering refinement relation and proves quiescent runtime-state agreement with router-owned canonical truth
- `Field/Adequacy/Safety.lean` packages reduction-soundness, safety-preservation, observational-equivalence, and projected-information order-insensitivity theorems on top of that runtime/system relation
- `Field/Adequacy/Cost.lean` now packages the first cost-preservation results too: projected runtime artifacts preserve the canonical-search input, input size, search space, and linear search-work class exactly
- `Field/Adequacy/Optimality.lean` now packages the first projected-runtime budgeted-optimality results too: once the reduced budget covers the projected canonical-search surface, projected runtime search agrees exactly with the same router-owned canonical result and has zero regret
- `Field/Adequacy/Fixtures.lean` pins the reduced canonical story to concrete runtime artifacts and one explicit non-claim
- `Field/AssumptionTheorems.lean` now packages `contract_yields_runtime_execution_canonical_refinement` as the preferred execution-state theorem above the older projected-artifact surface
- under that stronger runtime-state path, runtime canonical selection agrees with the same router-owned canonical selector without talking only about one synthetic artifact list

## Structural Harmonization

The stack now has a more explicit internal taxonomy than the older reduced
model did.

### Unique Feature

The distinctive feature of the current implementation is that it has one
theorem-backed compact reduction from Bayesian posterior state to a controller-
facing summary that is intentionally lossy but still sufficient for the current
deterministic control path under fixed exogenous inputs.

That is stronger than a heuristic compression story and weaker than a full
information-theoretic optimality claim. The docs and theorem surfaces now keep
that distinction explicit.

### Selector Family And Search Boundary

The router selector story now has one shared family boundary:

- base support selector
- stronger support-then-hop-then-stable selector
- explicit selector semantics and execution-policy vocabulary in `Field/Router/Selector.lean`
- system refinements of those selectors
- runtime/adequacy refinements of those selectors

The important rule is:

- selector semantics own candidate domain, eligibility, objective, and tie-break
- search/execution policy may vary budget, traversal profile, or caching policy
- posture or regime may influence execution policy, but not router-owned truth

So posture can affect how a search is executed in a richer runtime story, but
it does not get to redefine canonical selector semantics in the reduced stack.

The current Rust engine now has a private Telltale-backed search substrate on
top of that selector story:

- exact node objectives run `SearchQuery::single_goal`
- gateway and service objectives run selected-result candidate-set search over
  frontier neighbors
- evidence changes can trigger snapshot reconfiguration and explicit reseeding
  within one shared route epoch

The Lean proof stack now also has a direct reduced field search object in
`Field/Search/API.lean` and a search-aware adequacy layer in
`Field/Adequacy/Search.lean`. What is proved is:

- objective-to-query mapping
- snapshot identity and reduced reconfiguration metadata
- selected-result and replay-style surface lemmas
- execution-policy/selector-semantics separation
- reduced runtime-search adequacy packaging and canonical-route refinement

What remains out of scope for the proof stack is the richer full Rust
frozen-snapshot Telltale machine and its operational internals.

### Stat-Mech-Like Story

The local stack now has an explicit order-parameter layer:

- posterior semantics
- reduced summary semantics
- order-parameter / local phase semantics
- exogenous controller fusion
- regime classification

This is intentionally only a local reduced phase/regime interface. It is not
yet a large-network mean-field limit or a fluid-limit theorem pack.

### Classical Versus Distributed Surfaces

The theorem organization now distinguishes:

- local quantitative/classical theorem surfaces
- distributed/profile-envelope theorem surfaces
- bridge theorems connecting them

Convergence and threshold claims should now be read through that split. Local
order-parameter interpretation is not the same thing as a distributed
convergence claim.

### Corridor And Coarse Graining

The corridor/public story is now described as:

- private probabilistic microstate
- retained aggregate masses
- public macrostate / corridor observable

That makes the erasure story explicit: corridor publication is a coarse-grained
macro-observable, not a direct exposure of the latent belief state.

### Negative Boundaries

Several negative boundaries remain deliberate and important:

- posterior confidence is not router truth
- quality comparison is not router truth
- projection is not installation
- adequacy is not semantic ownership
- broader async envelopes do not silently replace clean-regime convergence
- reduced runtime refinement is not full extracted-Rust forward simulation

### Gap-Family Status

The harmonization work structurally addresses:

- local reduction / blindness / macrostate clarity
- assumption-surface modularity
- selector-family factoring
- publication/refinement lineage clarity
- shared cost vocabulary

The harmonization work still leaves explicit open gaps for:

- stronger divergence and information-theoretic reduction results
- broader async convergence and transport correctness
- stronger extracted-Rust runtime correctness over richer runtime internals
- richer canonical objectives and global optimality
- large-network asymptotic theory
- production-controller correctness

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

The async layer now also has one broader explicit regime:

- `boundedDelayRetryAssumptions`
  - `maxDelay = 1`
  - `retryBound = 1`
  - `lossPossible = True`

Under that broader regime, the current theorems are intentionally narrower. They show queue-growth and no-strengthening results for existing in-flight claims. They do not claim the same fixed-point or convergence results as the reliable-immediate / empty-queue regime.

That assumption boundary is now explicit in the theorem surface as well: the broader async regime does not silently replace the reliable-immediate hypotheses used by the fixed-point and stronger canonical stability theorems.

The new bounded system layer keeps that story honest at the delivery boundary too:

- queue size after one end-to-end step remains bounded by current backlog plus fresh publications
- lifecycle output cardinality stays bounded by the ready transport queue
- `system_step_work_units_bounded_by_transport_volume` gives the current abstract per-step latency bound: one reduced `systemStep` costs at most a constant multiple of current in-flight backlog plus fresh publications in the proof-facing work-unit model
- every delivered lifecycle route preserves the shape/support of some ready transport envelope, so overload and stale transport evolution can delay or suppress information but do not make it stronger
- every ready installed route is processed in the same reduced end-to-end step, so the current scheduler model does not admit starvation or priority inversion at the ready-installed boundary
- under the bounded-delay/bounded-retry regime, a retry-eligible dropped envelope becomes ready after one retry cycle and can then be processed into `readyInstalledRoutes` in the next reduced end-to-end step if admission succeeds
- `systemStep_inflight_length_bounded_by_congestion_loss_budget` gives the first mixed saturation/loss budget theorem: one reduced end-to-end step keeps in-flight backlog within the current congestion/loss budget plus fresh publications
- `system_queue_drains_after_one_retry_cycle_without_new_publications` gives the current queue-drain horizon: if there are no fresh publications and the backlog consists only of retry-eligible dropped envelopes, one full retry cycle drains the queue
- under that same regime, `single_retry_loss_preserves_canonical_support_after_one_retry_cycle` gives the first bounded-loss canonical theorem: one retry-eligible dropped envelope with a support-dominance condition recovers the same canonical support after one retry cycle
- `redundancy_threshold_one_preserves_canonical_support` makes the first quorum-style statement explicit for the current reduced model: threshold `1` is enough when the recovered admissible update support-dominates every eligible competitor after the retry cycle
- `single_retry_loss_graceful_degradation_envelope` makes the first graceful-degradation statement explicit: after one retry cycle the recovered update either restores the same canonical support winner or clears the canonical route to `none`, but it does not create a stronger winner than the recovered evidence justifies
- `intermittent_loss_eventually_converges_after_recovery` makes the current intermittent-loss claim explicit: once the reduced state has returned to a reliable-immediate empty-queue recovery state, canonical selection reconverges after one reduced step and stays fixed on later iterates
- `partial_delivery_does_not_oscillate_after_recovery_threshold` packages the current no-oscillation claim for partial delivery under load: once the execution has crossed the explicit recovery threshold, later iterates cannot keep flipping the canonical winner
- `recovery_threshold_resumes_convergence` names that threshold directly: when backlog has drained to `[]` and the async regime has returned to `reliableImmediateAssumptions`, canonical convergence resumes after one reduced step and remains fixed thereafter
- `recovered_invalid_update_clears_canonical_route_after_one_retry_cycle` gives the first withdrawal-safety theorem under loss: once a recovered invalidating update is processed after one retry cycle and no other eligible competitor remains, the canonical route becomes `none`
- once the queue has actually cleared and the regime is back to reliable-immediate, the current candidate view and canonical support winner recover within one reduced end-to-end step and stay fixed on later iterates

The first explicit resource/complexity layer is now in place too:

- `Field/System/Cost.lean` defines proof-facing communication, queue, storage, and compute work-unit budgets and proves they are bounded by the current transport-volume budget
- `explicit_transport_volume_budget_preserves_next_state` packages the current budget-preservation result for the next reduced system state
- `maintenance_work_units_amortized_under_reliable_immediate_empty` packages the first amortized maintenance statement: repeated maintenance passes do not grow work after the first pass on the current stabilized lifecycle surface
- `communication_work_units_stable_under_reliable_immediate_empty` and `transport_volume_budget_stable_under_reliable_immediate_empty` package the current stable-input communication-volume bound
- `transport_volume_budget_dominates_system_step_work` packages the current local computability and scalability law: one reduced `systemStep` is bounded by a constant multiple of local queue plus publication volume
- `system_step_work_bottlenecked_by_max_queue_or_communication` makes the current bottleneck story explicit: the worst-case work is dominated by the larger of queue backlog and fresh publication volume
- `per_destination_storage_bounded_by_system_lifecycle` packages the current per-destination storage bound over the canonical-search surface
- `resource_pressure_does_not_strengthen_claims` packages the current graceful-resource-degradation claim: tight budgets may suppress or delay information, but the lifecycle output remains transport-derived rather than stronger than its ready-envelope source
- `Field/Router/Cost.lean` defines the current canonical-search cost model and proves it is linear in the lifecycle input size, with explicit worst-case, incremental, stable-input, search-space, and maintenance-invariance bounds
- `Field/Adequacy/Cost.lean` ties that back to the runtime-facing projection by proving the projected artifact list preserves the canonical-search input, input size, search space, and work units exactly

The first explicit time-bounded / reduced-context optimality layer is now in place too:

- `Field/Router/Optimality.lean` defines a budgeted support-only canonical search over the eligible-route surface and packages the current exact-within-budget, anytime-monotone, deadline-safe, and threshold-region theorems for that router-owned objective
- `budgetedCanonicalBestRoute_some_is_support_best_within_budget` makes the “best found within budget” claim explicit for the current reduced budget model
- `budgetedCanonicalSupportRegret_bounded` and `budgetedCanonicalSupportRegret_eq_zero_of_budget_covers` package the current regret story: regret is always bounded by the full optimum support value and drops to `0` once the budget covers the eligible canonical-search surface
- `budgetedCanonicalPareto_frontier` now packages the current budget tradeoff story explicitly: larger reduced budgets weakly increase search work and weakly decrease support regret for the current support-only objective
- `budgetedCanonicalBestRoute_stable_after_exact_threshold` makes the current reduced-search stability boundary explicit: once the budget has crossed the exact eligible-search threshold, larger budgets cannot change the canonical answer
- `Field/System/Optimality.lean` lifts that to the system layer and makes the reduced-view story explicit: `canonicalSystemRouteView_supportDominance_is_sufficient_statistic` packages the current sufficient-statistic theorem for the support-only objective, while `supportDominance_reduction_preserves_dominance` and `supportDominance_reduction_has_no_rank_inversion` package exact preservation on the reduced route-view surface
- `Field/Adequacy/Optimality.lean` ties the same story back to projected runtime artifacts with `projected_runtime_budgeted_canonical_route_eq_canonicalSystemRoute_of_budget_covers` and the matching zero-regret / threshold-region / post-threshold-stability theorems
- `Field/Adequacy/Fixtures.lean` now also exposes a small fixture-generation path: `fixtureRuntimeStateOfArtifacts`, `generatedFixtureArtifactsOfSystem`, and `generatedFixtureRuntimeStateOfSystem` turn admitted runtime artifacts or projected system states into proof-facing fixture objects with admission/projection theorems

The first resilience layer is now explicit too:

- `Field/Router/Resilience.lean` defines a first participation-fault vocabulary that separates silence/dropout, non-cooperation, and dishonest publication
- the current proved theorem pack starts with silence-only dropout and a quantitative budget over dropped publishers
- `Field/System/Resilience.lean` lifts that to router-owned canonical support stability/degradation under the clean reliable-immediate / empty-queue regime, and now also includes a second bounded non-participation regime keyed to the separate `nonCooperation` fault class
- destination-scope fault containment is now explicit in the router-owned selector too: `canonicalBestRoute_ignores_off_destination_route` says an off-destination route does not affect canonical selection for the current destination
- the current sparse-connectivity / low-support theorems are still reduced, but they now cover:
  - no false confidence without active destination-local evidence via `canonicalSystemRoute_eq_none_of_no_active_destination_match`
  - minimal-connectivity correctness when exactly one eligible route remains via `canonicalSystemRoute_eq_some_of_unique_eligible`
  - reduced participation-cut disappearance and unique-bridge fragility via `dropoutCanonicalSystemSupportValue_eq_none_of_all_eligible_publishers_dropped` and `dropoutCanonicalSystemSupportValue_eq_none_of_unique_bridge_publisher_dropped`
  - threshold emergence, threshold disappearance, and near-threshold stability via `canonicalSystemSupportAtLeast_of_dominating_route`, `not_canonicalSystemSupportAtLeast_of_all_eligible_below_threshold`, and `canonicalSystemSupportAtLeast_stable_under_reliable_immediate_empty`
  - delayed sparse visibility via `ready_installed_route_eventually_appears_in_system_destination_views`: one positive non-opaque ready route appears in the next system destination view even when participation is minimal
  - no amplification / partial-observation robustness via `canonicalSystemRoute_support_conservative` and `canonicalSystemRoute_explicit_path_requires_explicit_sender_knowledge`: canonical winners stay bounded by the sender-local support/knowledge that justified them
  - the first vanishing-support limit via `not_canonicalSystemSupportAtLeast_of_all_eligible_below_threshold 1`: if every eligible destination-local support is below `1`, there is no positive-support canonical outcome
  - the current sparse-network scaling law is destination-local rather than asymptotic: `canonicalBestRoute_front_off_destination_routes_irrelevant` says unrelated-destination route growth or sparsity does not change canonical selection for the destination being analyzed
  - `canonicalSystemSupport_threshold_boundary` packages the current critical-threshold story explicitly as the threshold-emergence / threshold-disappearance boundary
  - `threshold_one_discontinuity_example` makes the first discontinuity result explicit: crossing the support threshold from `0` to `1` can flip the thresholded canonical-support predicate
- an explicit boundary theorem states that these silence-only dropout theorems do not extend to dishonest publication

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
- bounded-recovery theorems now make the current horizon explicit: once one changed input has been absorbed by one reduced end-to-end step under the clean regime, later iterates keep the same candidate view and canonical support winner
- canonical-router refinement says the current support-only system winner agrees with the router-owned canonical selector
- exact support-optimum theorems say that same router-owned selector is globally support-best over the full reduced lifecycle surface for its current objective
- low-level runtime-to-canonical refinement says an admitted runtime artifact stream with explicit reduced lifecycle alignment agrees with that same router-owned canonical selector
- stronger projected runtime/system refinement says the reduced runtime artifact stream generated from `systemStep` agrees with that same router-owned canonical selector without a free alignment parameter
- the preferred packaged adequacy theorem is now the runtime-state execution refinement theorem above that projected-artifact bridge
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

Truth ladder:

- posterior confidence is local/private semantics
- canonical route is router truth
- quality is exported-view comparison
- adequacy is a semantic bridge into reduced system/router layers, not a truth owner

Negative boundaries:

- quality is not truth
- posterior confidence is not truth
- projection is not installation
- adequacy is not semantic ownership

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
- keep field-local glue explicit when the repo intentionally stops at a reduced
  Telltale-family instantiation
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
