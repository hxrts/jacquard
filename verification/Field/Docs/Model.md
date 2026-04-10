# Field Local Model Specification

This note describes the current formal object for the field engine in mathematical and proof-structural terms. It is the authoritative description of the current reduced local model, not a sketch of the full production Rust system. The wider field verification stack now also has reduced network and router layers in `Field/Network/*` and `Field/Router/*`, but this document stays focused on the destination-local semantic object they build on.

## Scope

The current Lean model is a destination-local, deterministic, bounded state machine for one node and one local round. It is intentionally smaller than the full Rust field engine.

It does model:

- one destination-local observer-controller state
- one local round transition
- a finite belief object over reduced reachability hypotheses
- a reduced information layer built on top of that belief object
- a public corridor-envelope projection derived from local state
- a small finite decision layer over a representative evidence alphabet

The local model by itself does not yet model:

- asynchronous transport behavior
- end-to-end convergence or optimality
- full production router lifecycle semantics

Those wider concerns are now split as follows:

- reduced finite network semantics
  - `Field/Network/*`
- reduced publication/admission/installation semantics
  - `Field/Router/*`
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

## What Is Proved In The Local Model

The local model and its theorem packs currently establish:

- boundedness of the local state
- harmony between posterior, mean-field, controller, regime, posture, scores, and projection
- honesty of public projection relative to local knowledge
- small temporal theorems over repeated rounds
- first quantitative ranking-style results over the reduced local state

The model should therefore be read as:

- structurally mature as a reduced deterministic local semantic object
- moderately mature as a finite information object
- still early as a quantitative decision-theoretic or information-theoretic routing theory

## What Is Not Yet Proved

The current field local model does not yet prove:

- global routing optimality
- end-to-end convergence
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
- stronger multi-round stabilization laws
- tighter connection between local scoring and public projection conservativity

Until then, this document should be read as the specification of the current reduced local model and information boundary, not of the full Rust field engine.
