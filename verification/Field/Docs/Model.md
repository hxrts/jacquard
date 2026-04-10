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
  → mean-field summary
  → controller update
  → regime classification
  → posture selection
  → continuation scoring
  → public corridor projection
```

This is why `LocalState` carries all of these components together and why the harmony laws matter: they prevent later stages from inventing stronger support or stronger public claims than earlier stages justify.

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

## Blindness And Erasure

`Field/Information/Blindness.lean` now treats the public corridor projection as a lossy observer over the normalized private belief object.

The key current result is the first genuine erasure theorem:

```text
opaque_projection_erases_unknown_unreachable_split
```

Informally, once corridor-capable mass is zero on both sides, the public projection forgets how the remaining mass is split between `unknown` and `unreachable`.

This is intentionally narrow. It does not yet give a full mutual-information or divergence theory for the public corridor projection. But it is now a mathematically meaningful blindness statement over the normalized belief layer.

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

- `round_projection_support_conservative`
- `round_mean_field_tracks_posterior_support`
- `explicit_projection_requires_explicit_round_knowledge`
- `repeated_explicit_path_rounds_stabilize`

These are still reduced local theorems, not end-to-end system theorems. Their role is to show that the fully composed local round step preserves the same honesty/conservativity story promised by the API-level harmony laws.

## Quantitative Layer

`Field/Information/Quantitative.lean` adds the first small quantitative objects and lemmas above the normalized belief boundary.

The current file defines:

- `beliefL1Distance`
- `localUncertaintyPotential`

and proves:

- `beliefL1Distance_nonneg`
- `beliefL1Distance_eq_zero_of_equal`
- `localUncertaintyPotential_nonneg`
- `equal_beliefs_induce_zero_projection_loss`

This is still intentionally small. It is not a full routing-quality theory or a strong information-theoretic comparison framework. It is the first quantitative surface that later strengthening can build on.

## What Is Proved In The Local Model

The local model and its theorem packs currently establish:

- boundedness of the local state
- harmony between posterior, mean-field, controller, regime, posture, scores, and projection
- honesty of public projection relative to local knowledge
- small temporal theorems over repeated rounds
- refinement lemmas over the composed round step
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
- stronger mutual-information bounds for the public projection
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
