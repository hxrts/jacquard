# Field Local Model Specification

This note describes the current formal object for the field engine in mathematical and proof-structural terms. It is the authoritative description of the current reduced local model, not a sketch of the full production Rust system. The wider field verification stack now also has reduced network and router layers in `Field/Network/*` and `Field/Router/*`, but this document stays focused on the destination-local semantic object they build on.

## Scope

The current field verification stack is no longer only a destination-local model. It now includes:

- a destination-local deterministic observer-controller model
- a finite normalized information layer over the reduced hypothesis space
- a reduced private protocol layer
- a reduced finite network and router boundary
- a reduced async delivery layer
- a reduced end-to-end system semantics
- a reduced routing-quality / comparison layer above system-facing route views
- first fixed-point / stabilization results under strong reduced assumptions
- a reduced runtime adequacy layer
- first system-level summary and boundary results

This document still focuses on the local semantic object at the center of that stack. That local object is a destination-local, deterministic, bounded state machine for one node and one local round, and it remains intentionally smaller than the full Rust field engine.

It does model:

- one destination-local observer-controller state
- one local round transition
- a finite belief object over reduced reachability hypotheses
- a reduced information layer built on top of that belief object
- a small local refinement layer over the composed round step
- a first quantitative layer over the normalized belief object
- a public corridor-envelope projection derived from local state
- a small finite decision layer over a representative evidence alphabet

The local model by itself still does not model:

- asynchronous transport behavior
- router lifecycle maintenance
- end-to-end sequencing or stabilization
- routing-quality or optimality claims

Those wider concerns are now split as follows:

- reduced finite network semantics
  - `Field/Network/*`
- reduced publication/admission/installation semantics
  - `Field/Router/*`
- reduced router lifecycle semantics
  - `Field/Router/Lifecycle.lean`
- reduced async delivery semantics, transport lifecycle lemmas, and first async publication-safety theorems
  - `Field/Async/*`
- reduced end-to-end sequencing, observer results, and convergence theorems
  - `Field/System/EndToEnd.lean`
  - `Field/System/Convergence.lean`
- reduced route-comparison and ranking semantics over exported lifecycle/system views
  - `Field/Quality/API.lean`
  - `Field/Quality/System.lean`
- system-level aggregate summaries and assumption-boundary results
  - `Field/System/*`
- runtime extraction and reduced simulation witness
  - `Field/Adequacy/*`

## Core State Space

Let the bounded scalar domain be:

```text
B := { n ∈ Nat | 0 ≤ n ≤ 1000 }.
```

The concrete instance represents all bounded scalar quantities with clamped naturals in `B`.

The finite enumerated domains are:

- `RefreshSignal = { unchanged, explicitRefresh }`
- `EvidenceFeedback = { none, weakReverse, strongReverse }`
- `ReachabilitySignal = { preserve, unknown, unreachable, corridorOnly, explicitPath }`
- `ObservationFreshness = { stale, fresh }`
- `ReachabilityKnowledge = { unknown, unreachable, corridor, explicitPath }`
- `FieldHypothesis = { unknown, unreachable, corridor, explicitPath }`
- `OperatingRegime = { sparse, congested, retentionFavorable, unstable, adversarial }`
- `RoutingPosture = { opportunistic, structured, retentionBiased, riskSuppressed }`
- `CorridorShape = { opaque, corridorEnvelope, explicitPath }`

The local observational input space is:

```text
EvidenceInput :=
  RefreshSignal
  × ReachabilitySignal
  × B
  × B
  × B
  × EvidenceFeedback
```

The destination-local semantic state is:

```text
LocalState :=
  PosteriorState
  × MeanFieldState
  × ControllerState
  × RegimeState
  × PostureState
  × ScoredContinuationSet
  × CorridorEnvelopeProjection
```

with components:

```text
PosteriorState :=
  FiniteBelief × ObservationFreshness × ReachabilityKnowledge

FiniteBelief :=
  B × B × B × B
  -- unknown, unreachable, corridor, explicit-path weights

MeanFieldState :=
  B × B × B

ControllerState :=
  B × B

RegimeState :=
  OperatingRegime × B

PostureState :=
  RoutingPosture

ScoredContinuationSet :=
  B × B

CorridorEnvelopeProjection :=
  CorridorShape × B × Nat × Nat
```

The concrete instance uses a small fixed family of hop bands:

- explicit path: `(2, 2)`
- corridor envelope: `(1, 3)`
- opaque: `(0, 4)`

## Unified Round Semantics

The local model is intentionally one unified observer-controller pipeline rather than a bag of disconnected submodels.

At the semantic level, the one-round flow is:

```text
evidence
  → posterior update
  → posterior-derived reduced summary
  → mean-field summary
  → controller update
  → regime classification
  → posture selection
  → continuation scoring
  → public corridor projection
```

This is why `LocalState` carries all of these components together and why the harmony laws matter: they prevent later stages from inventing stronger support or stronger public claims than earlier stages justify.

The current implementation now makes one additional quotient boundary explicit:

```text
posterior + Bayesian companion belief
  → ReducedBeliefSummary
  → mean-field / controller-facing control fusion
```

`ReducedBeliefSummary` keeps only support mass, uncertainty mass, and one
public macrostate label. It deliberately does not retain full freshness, the
full finite belief vector, or the Bayesian posterior object.

The local pipeline now makes a second boundary explicit as well:

```text
ReducedBeliefSummary
  → LocalOrderParameter
  → mean-field / controller-facing control fusion
  → regime classification
```

`LocalOrderParameter` is the intended stat-mech-like interface for the current
reduced model. It is not a second posterior object and it is not the controller
state. It is the local phase/regime surface derived from the reduced summary.

The current coordinates are deliberately simple:

- support-like field strength
- uncertainty-adjacent burden
- retained public macrostate
- threshold proximity / instability indicators carried as reduced scalar views

That gives the code a principled order-parameter layer without claiming a full
thermodynamic or asymptotic mean-field theory.

## Order Parameter And Regime Story

`Field/Model/API.lean` now names the reduced mean-field interface directly:

- `ReducedBeliefSummary`
  - posterior quotient used for controller-facing reduction
- `LocalOrderParameter`
  - local phase/order-parameter surface extracted from that quotient
- `MeanFieldState`
  - control-fused reduced state after exogenous controller inputs are applied

The intended interpretation is:

```text
posterior belief
  -> reduced summary
  -> local order parameter
  -> exogenous control fusion
  -> mean field / controller state
  -> regime classification
```

This matters because the reduced summary is intentionally lossy, while the
controller-facing mean field still depends on exogenous control pressure. The
proof surface now makes both of those facts explicit:

- the reduced summary is sufficient for the current mean-field/controller path
  only under fixed exogenous control inputs
- the reduction alone does not determine the whole downstream control path
- `UncertaintyBurden` is currently treated as an order-parameter-adjacent
  control quantity, not as proved Lyapunov data

The current `GF2` / `GF7` boundary is therefore explicit in the local API:

- stronger convergence claims remain profile-indexed system work
- large-network / fluid-limit claims remain future work
- the order-parameter interface is designed so those later theorems can land
  on a named surface instead of introducing a second local vocabulary

## Finite Belief And Information Layer

The information story now has two explicit layers:

- `Field/Model/*`
  - the bounded local controller model
- `Field/Information/*`
  - the explicit probabilistic normalization layer built over `FiniteBelief`

### Finite Belief

`FiniteBelief` assigns bounded nonnegative weight to the four reduced hypotheses:

```text
FieldHypothesis = { unknown, unreachable, corridor, explicitPath }.
```

The API exposes:

```text
weight : FiniteBelief → FieldHypothesis → Nat
totalWeight : FiniteBelief → Nat
supportMass : FiniteBelief → Nat
uncertaintyMass : FiniteBelief → Nat
```

where the current concrete observables are:

```text
supportMass(belief) := min(corridorWeight + explicitPathWeight, 1000)
uncertaintyMass(belief) := min(unknownWeight + unreachableWeight, 1000)
```

### Probability-Simplex Style Boundary

The information API now defines:

```text
ProbabilitySimplexBelief
simplexBelief : FiniteBelief → ProbabilitySimplexBelief
normalizeBelief : FiniteBelief → Distribution FieldHypothesis
```

The compatibility law `SimplexMatchesFiniteBelief` requires:

- nonnegative mass
- total mass one
- zero-total fallback to a point mass on `unknown`
- nonzero-total normalization by finite weights

So the current information layer is no longer only “normalized integer weights” in prose. It has an explicit probability-simplex style API boundary.

### What Is Proved In The Information Layer

The current concrete instance proves:

- normalized pmf nonnegativity
- normalized pmf sums to one
- explicit-path mass matches normalized explicit-path probability
- corridor-capable mass matches normalized corridor-plus-explicit-path probability
- Shannon uncertainty is nonnegative
- explicit-path mass is bounded by corridor-capable mass
- explicit-path mass is exactly one when all belief mass sits on explicit-path

This is still an early information layer, but it is a real one.

## Coarse-Grained Corridor Macrostate

The current public corridor story is now organized as a three-level coarse
graining:

```text
private probabilistic microstate
  -> retained aggregate masses
  -> public corridor macrostate
```

The microstate is the full probabilistic belief over route existence, quality,
transport reliability, witness reliability, and local knowledge.

The retained aggregate layer keeps only the aggregates that the current reduced
stack needs as one explicit semantic boundary:

- explicit-path mass
- corridor-capable mass
- quality-band masses
- transport-reliability masses
- observation-reliability masses

The public corridor macrostate is then a further coarse-grained observable. It
forgets latent distinctions on purpose. The main erased axes are:

- quality split inside corridor-capable mass
- transport-reliability split
- observation-reliability split
- some unknown versus unreachable distinctions once corridor-capable mass
  collapses

This is why the corridor projection should be read as a macro-observable map,
not just as a weaker route-shape heuristic.

## Blindness And Erasure

`Field/Information/Blindness.lean` now treats the local field story as a chain
of lossy observers:

```text
posterior / Bayesian belief
  → ReducedBeliefSummary
  → public macrostate observer
  → public corridor projection
```

The key current results are now split by boundary.

### Posterior To Reduced Summary

The reduction itself is intentionally lossy. The current theorem surface makes
that explicit:

- `reduction_erases_probabilistic_belief_choice`
- `reduction_erases_freshness_under_fixed_belief_and_knowledge`
- `reduction_erases_unknown_unreachable_distinction_under_equal_uncertainty`

So the reduced summary is not "the posterior with fewer fields". It is a
controller-facing quotient that forgets:

- the full Bayesian posterior distribution
- freshness once support / uncertainty / macrostate are fixed
- the unknown-versus-unreachable split once the reduced summary is opaque and
  carries the same uncertainty mass

### Reduced Summary To Public Macrostate

The public observer over the reduced summary is coarser still:

- `publicProjectionOfReducedSummary`
- `public_projection_of_reduced_summary_forgets_support_and_uncertainty`

Once the public macrostate label is fixed, public observation no longer sees
the reduced support or uncertainty coordinates.

### Public Projection Of The Normalized Belief Object

The normalized-belief public observer now has aggregate-mass stability theorems:

- `explicit_projection_of_positive_explicit_mass`
- `corridor_projection_of_zero_explicit_and_positive_corridor_mass`
- `opaque_projection_of_zero_corridor_mass`

Those say exactly which aggregate mass differences matter for macrostate
changes and which belief differences are invisible at the corridor-macrostate
level.

The older first genuine erasure theorem remains:

```text
opaque_projection_erases_unknown_unreachable_split
```

Informally, once corridor-capable mass is zero on both sides, the public projection forgets how the remaining mass is split between `unknown` and `unreachable`.

This is intentionally still narrow. It does not yet give:

- full mutual-information bounds for the public corridor projection
- KL-style update inequalities
- a full divergence theory over the reduction itself
- information-theoretic optimality claims for the controller or router

But it is now a mathematically meaningful blindness chain over the reduced
local model, from posterior belief to reduction to public projection.

## Decision Layer

`Field/Model/Decision.lean` adds one small finite decision layer:

- a representative evidence alphabet
- one-step exploration from a root state
- a decidable question for explicit-path publication in one round
- soundness and completeness for that bounded decider

This is not a planner and not a global routing decision system. It is a small proof-oriented decision procedure over the reduced local round.

## Refinement Layer

`Field/Model/Refinement.lean` now packages the first small theorem family over the composed round itself.

The current file proves:

- exact reduction-preservation results:
  - `reduced_summary_preserves_support_mass`
  - `reduced_summary_preserves_uncertainty_mass`
  - `reduced_summary_preserves_public_macrostate`
- sufficiency / compression-boundary results:
  - `equal_reduced_summaries_yield_equal_mean_field_under_equal_pressure`
  - `equal_reduced_summaries_yield_equal_controller_updates_under_equal_pressure`
  - `reduced_summary_is_sufficient_for_mean_field_given_evidence`
  - `reduced_summary_is_sufficient_for_controller_update_given_evidence`
- conservative / bounded / monotone reduction results:
  - `reduced_summary_support_conservative`
  - `reduced_summary_uncertainty_conservative`
  - `reduced_summary_bounded`
  - `reduced_summary_support_monotone_of_posterior_support_monotone`
  - `reduced_summary_uncertainty_monotone_of_posterior_uncertainty_monotone`
- explicit non-sufficiency of the reduction alone for the control path:
  - `exogenous_controller_pressure_can_change_mean_field_after_same_reduction`
- `round_projection_support_conservative`
- `round_mean_field_tracks_posterior_support`
- `explicit_projection_requires_explicit_round_knowledge`
- `repeated_explicit_path_rounds_stabilize`

These are still reduced local theorems, not end-to-end system theorems. Their role is to show that the fully composed local round step preserves the same honesty/conservativity story promised by the API-level harmony laws.

## Quantitative Layer

`Field/Information/Quantitative.lean` adds the first small quantitative objects and lemmas above the normalized belief boundary.

The current file defines:

- `beliefL1Distance`
- `natGap`
- `reducedSupportGap`
- `reducedUncertaintyGap`
- `reducedSummaryAggregateGap`
- `localUncertaintyPotential`

and proves:

- `beliefL1Distance_nonneg`
- `beliefL1Distance_eq_zero_of_equal`
- `reducedSupportGap_matches_posterior_support_gap`
- `reducedUncertaintyGap_matches_posterior_uncertainty_gap`
- `equal_reduced_summaries_induce_zero_aggregate_gap`
- `localUncertaintyPotential_nonneg`
- `equal_beliefs_induce_zero_projection_loss`

This is still intentionally small. It is not:

- a full routing-quality theory
- a strong information-theoretic comparison framework
- a proof of information-theoretic optimality for the reduced summary

It is the first quantitative surface that later strengthening can build on.

## What Is Proved In The Local Model

The local model and its theorem packs currently establish:

- boundedness of the local state
- harmony between posterior, mean-field, controller, regime, posture, scores, and projection
- honesty of public projection relative to local knowledge
- small temporal theorems over repeated rounds
- refinement lemmas over the composed round step
- reduction-preservation and compression-discipline theorems for the
  controller-facing summary
- explicit blindness / erasure statements for the reduction itself and for the
  public corridor macrostate
- first quantitative ranking / distance style results over the reduced local state

The model should therefore be read as:

- structurally mature as a reduced deterministic local semantic object
- moderately mature as a finite information object
- still early as a quantitative decision-theoretic or information-theoretic routing theory

## Relationship To System-Level Stabilization

The new fixed-point and no-spontaneous-promotion theorems do not live in the local model itself. They live in `Field/System/EndToEnd.lean` and `Field/System/Convergence.lean`, where the local projection is composed with:

- the reduced async transport layer
- router lifecycle installation / maintenance
- a reduced end-to-end step relation

Those results are intentionally narrow. They currently rely on:

- `reliableImmediateAssumptions`
- an empty initial in-flight queue
- unchanged local/network state across the reduced end-to-end step, captured by `system_step_preserves_network`

So the current story is not "the local model proves general convergence." The honest claim is narrower: under a stable-input, reliable-immediate regime, the installed candidate view reaches a reduced fixed point and does not spontaneously promote to explicit-path.

## Relationship To The Quality Layer

The new routing-quality work also lives above the local model.

`Field/Quality/*` compares exported lifecycle/system route views. It does not change the local observer-controller semantics, and it does not promote the local model into a global route-selection or optimality object.

That separation matters:

- the local model still owns local knowledge, support, and public projection
- the system layer still owns transport/lifecycle composition and stable-step results
- the quality layer only ranks, compares, and support-refines the reduced route views that those higher layers already expose

The new support-only refinement result is still above the local model:

- `Field/Quality/Reference.lean` defines the reference support-best semantics over exported route views
- `Field/Quality/Refinement.lean` proves `supportDominance` agrees with that reference semantics
- `Field/Router/Canonical.lean` owns the current canonical support selector
- `Field/System/Canonical.lean` proves the current system-facing support winner agrees with that router-owned selector
- `Field/Adequacy/Canonical.lean` proves the low-level runtime artifact bridge under an explicit reduced lifecycle-alignment boundary
- `Field/Adequacy/Projection.lean` proves a reduced runtime execution projected from `systemStep` refines to that same canonical selector without any extra alignment parameter
- none of that moves route optimality into `Field/Model`

## What Is Not Yet Proved

The current field local model does not yet prove:

- global routing optimality
- end-to-end convergence outside the reduced reliable-immediate stable-input regime
- strong routing-quality or optimality claims over installed candidates
- large-network mean-field limits
- KL-style update inequalities
- stronger divergence bounds over the reduction itself
- stronger mutual-information bounds for the public projection
- theorem-backed information-theoretic optimality claims for the reduced summary
- production-controller correctness

Those belong to later strengthening phases.

## Where To Extend Next

The most natural next extensions of this document’s model are:

- a richer normalized belief object with stronger probabilistic structure
- sharper divergence and entropy laws
- stronger blindness theorems for the corridor projection
- stronger multi-round stabilization laws beyond the current reliable-immediate fixed-point regime
- richer reference objectives beyond the current support-only refinement layer
- stronger router-canonical objectives beyond the current support-only canonical selector
- tighter connection between local scoring and public projection conservativity

Until then, this document should be read as the specification of the current reduced local model and information boundary, not of the full Rust field engine.
