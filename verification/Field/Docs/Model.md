# Field Local Model Specification

This note states the first formal object for the field engine in mathematical terms and records how it relates to the Rust implementation.

The purpose of this document is to make the proof object explicit: what the state space is, what one local round means, which invariants are part of the model, and which theorems have actually been established.

## Scope

The current Lean model is a destination-local state machine for one node, one destination class, and one deterministic round step. It is deliberately transport-agnostic, bounded, discrete, and smaller than the production Rust field engine.

It does not yet model a network of nodes, asynchronous transport behavior, router publication semantics, route admission semantics, or end-to-end convergence or optimality.

## State Space

Let the bounded scalar domain be

```text
B := { n ∈ Nat | 0 ≤ n ≤ 1000 }.
```

The current concrete instance represents all scalar quantities with clamped natural numbers in `B`.

The finite enumerated domains are:

- `RefreshSignal = { unchanged, explicitRefresh }`
- `EvidenceFeedback = { none, weakReverse, strongReverse }`
- `ReachabilitySignal = { preserve, unknown, unreachable, corridorOnly, explicitPath }`
- `ObservationFreshness = { stale, fresh }`
- `ReachabilityKnowledge = { unknown, unreachable, corridor, explicitPath }`
- `OperatingRegime = { sparse, congested, retentionFavorable, unstable, adversarial }`
- `RoutingPosture = { opportunistic, structured, retentionBiased, riskSuppressed }`
- `CorridorShape = { opaque, corridorEnvelope, explicitPath }`

The formal input space is:

```text
EvidenceInput :=
  RefreshSignal
  × ReachabilitySignal
  × B                 -- support signal
  × B                 -- entropy signal
  × B                 -- controller pressure
  × EvidenceFeedback
```

The destination-local semantic state is the product

```text
LocalState :=
  PosteriorState
  × MeanFieldState
  × ControllerState
  × RegimeState
  × PostureState
  × ScoredContinuationSet
  × CorridorEnvelopeProjection.
```

With component structure:

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

The current model uses `Nat` for hop bounds, but the concrete instance fixes them to a tiny finite family of bands:

- explicit path: `(2, 2)`
- corridor envelope: `(1, 3)`
- opaque: `(0, 4)`

## Information-Theoretic Interpretation

The current Lean object is now split into two information layers:

- the local controller model in `Field/Model/*`
- the information-theoretic normalization layer in `Field/Information/*`

It is still not a full probabilistic routing model in the Shannon sense. It does not yet prove KL-style update inequalities, mutual-information bounds, or entropy-production statements about the whole controller pipeline. It now does, however, carry an explicit finite belief object and a first concrete weight-normalized distribution over that finite hypothesis space, so the information-theoretic story is no longer only metaphorical.

### Local Belief and Uncertainty

`FieldHypothesis` is the current finite hypothesis space:

```text
FieldHypothesis = { unknown, unreachable, corridor, explicitPath }.
```

`PosteriorState` is the first local information-bearing state. Formally it is

```text
PosteriorState = FiniteBelief × ObservationFreshness × ReachabilityKnowledge.
```

Semantically:

- `belief : FiniteBelief` assigns bounded nonnegative weight to each reduced hypothesis
- `freshness` indicates whether the uncertainty summary is backed by an explicit refresh event
- `knowledge` records the coarse reachability class currently justified by the local evidence

The instance currently derives two coarse observables from that belief object:

```text
supportMass(belief) := min(corridorWeight + explicitPathWeight, 1000)
uncertaintyMass(belief) := min(unknownWeight + unreachableWeight, 1000)
```

and then exposes them as:

```text
PosteriorState.support = supportMass(belief)
PosteriorState.entropy = uncertaintyMass(belief).
```

So the model should now be read as carrying a finite private belief object together with one bounded support observable and one bounded uncertainty observable derived from it.

### Mean Field as a Reduced Statistic

`MeanFieldState` is not a second independent source of truth. It is a reduced statistic derived from the posterior and control pressure:

```text
MeanFieldState = B × B × B.
```

Its intended role is analogous to a low-order moment summary or compressed field statistic. `fieldStrength` tracks the dominant local support level, `relayAlignment` approximates how well the neighborhood field aligns with continuation pressure, and `riskAlignment` approximates how much uncertainty and control pressure are jointly concentrated.

The harmony law

```text
s.meanField.fieldStrength = s.posterior.support
```

is therefore an information-flow statement: the coarse field summary may compress the local posterior, but it is not allowed to invent stronger support than the posterior carries.

### Controller and Regime as Decision-Theoretic Compression

`ControllerState`, `RegimeState`, and `PostureState` do not add new evidence. They are decision-theoretic compressions of the observational state: the controller turns reduced field statistics into slow control variables, the regime classifies the present explanatory situation, and the posture chooses the routing stance compatible with that regime.

So the current local round has the information-processing shape

```text
evidence
  → posterior surrogate
  → reduced field statistic
  → control statistic
  → regime classification
  → posture choice.
```

This is exactly why the model is written as one unified observer-controller pipeline instead of a set of disconnected submodels.

### Continuation Scores and Public Projection

`ScoredContinuationSet` and `CorridorEnvelopeProjection` are the first places where the model distinguishes between:

- private local scoring
- public shared claim

`ScoredContinuationSet` is still private and can be read as a bounded ranking over the continuation hypotheses that survive the current local belief state.

`CorridorEnvelopeProjection` is the conservative public image of that private state. It is intentionally lossy. In information-theoretic language, it is a rate-limited projection of the private belief state into a smaller public alphabet:

```text
CorridorEnvelopeProjection := CorridorShape × B × Nat × Nat.
```

The current honesty and harmony laws formalize this lossy-public boundary:

- explicit-path publication requires explicit-path knowledge
- public support is bounded above by local support
- hop-band disclosure is chosen from a fixed coarse family, not from arbitrary path detail

So the projection is not a posterior announcement. It is a constrained summary channel from private local belief into shared observable state.

### Uncertainty Functional

The current uncertainty functional is still a bounded surrogate, not a true Shannon entropy:

```text
U(belief) := uncertaintyMass(belief)
          := min(unknownWeight + unreachableWeight, 1000).
```

This quantity is intentionally simple. It measures how much belief mass remains in the two noncommittal or failure-like classes. It is useful for conservativity and stabilization proofs, but it is not yet a theorem-backed Shannon quantity.

The right future refinement is:

- refine `U` further with stronger entropy or divergence statements over the now weight-normalized finite distribution
- compare public projection to private belief using information loss or erasure arguments
- connect the finite field model to Telltale's `InformationCost.lean` and `ClassicalAnalysisAPI.lean` boundaries

The first concrete information instance now uses a true weight-normalized finite belief distribution with a zero-mass fallback to `unknown`. It defines:

```text
normalizeBelief : FiniteBelief → Distribution FieldHypothesis
shannonUncertainty : FiniteBelief → ℝ
```

and proves at least one genuine Shannon-style theorem:

```text
belief_shannon_entropy_nonnegative :
  0 ≤ shannonUncertainty(belief).
```

That theorem is modest, but it is no longer only a bounded-surrogate statement. It uses the real information-theoretic API boundary instead of only the coarse `uncertaintyMass` proxy. The next refinement should use the normalized belief object to derive sharper entropy, divergence, and blindness statements rather than only first nonnegativity and mass-ratio facts.

### What Is Proved Now vs Later

The current model proves structural properties of this compressed information pipeline:

- the state remains bounded
- information does not collapse across proof-relevant categories such as `unknown` and `unreachable`
- the public projection remains subordinate to the local belief state
- the reduced statistics remain harmonized with the posterior

These are pre-information-theoretic theorems. They establish that the current finite surrogate has the right shape before stronger claims are attempted.

Later refinements can legitimately strengthen the model by introducing, for example:

- explicit finite distributions over continuation hypotheses
- a normalized uncertainty functional with a direct Shannon interpretation
- divergence-style update measures between prior and posterior summaries
- theorem links to the `ClassicalAnalysisAPI` / `ClassicalAnalysisInstance` pattern already used in Telltale
- public/private information-flow bounds for the shared corridor envelope

That later work should refine the current model, not replace its basic observer-controller harmony structure.

## Deterministic Round Transition

The model is organized around one total deterministic transition

```text
roundStep : EvidenceInput × LocalState → LocalState.
```

This transition is defined as the composition

```text
roundStep
  = projectCorridor
    ∘ scoreContinuations
    ∘ choosePosture
    ∘ inferRegime
    ∘ updateController
    ∘ compressMeanField
    ∘ updatePosterior.
```

More precisely, if

```text
s  : LocalState
e  : EvidenceInput
p' := updatePosterior e s
m' := compressMeanField e p'
c' := updateController e m' s.controller
r' := inferRegime p' m' c'
t' := choosePosture r' c'
q' := scoreContinuations p' m' c' t'
x' := projectCorridor p' m' c' q'
```

then

```text
roundStep(e, s) = (p', m', c', r', t', q', x').
```

This matters because the model is not intended to be a loose federation of submodels. The Lean theorem surface is organized around the composed transition, and the subfunctions are introduced only to name the internal semantics of that single round.

## Representation Invariants

The current API isolates the following predicates.

### Boundedness

```text
PosteriorBounded
MeanFieldBounded
ControllerBounded
RegimeBounded
ContinuationScoresBounded
ProjectionBounded
StateBounded
```

Informally:

- all scalar state lies in `B`
- alternate continuation score is no greater than the primary score
- projected hop bounds satisfy `hopLower ≤ hopUpper`

### Harmony

The `Harmony` predicate captures the intended subordination relations between the layers of the local observer-controller.

For a state `s`, `Harmony(s)` requires:

```text
s.meanField.fieldStrength = s.posterior.support
s.controller.stabilityMargin = s.meanField.fieldStrength
s.projection.shape = explicitPath  ↔  s.posterior.knowledge = explicitPath
s.projection.support ≤ s.posterior.support
s.scored.alternateScore ≤ s.scored.primaryScore
```

This is the formal statement that:

- mean-field state is subordinate to the posterior
- controller state is subordinate to mean-field state
- explicit-path projection is subordinate to explicit knowledge
- shared support is subordinate to local support

### Semantic Non-Collapse

The first model treats the following distinctions as proof-relevant, not merely descriptive:

- stale vs fresh
- unknown vs unreachable
- corridor-only vs explicit-path

These distinctions are encoded in finite enums and then protected by theorems.

## Theorems Established So Far

The first concrete instance proves the following.

### 1. Determinism

```text
local_round_deterministic :
  ∀ e s, roundStep e s = roundStep e s
```

This is a basic sanity theorem: the local round is a pure function with no ambient nondeterminism.

### 2. Boundedness Preservation

Via `round_preserves_bounded`, the concrete instance proves

```text
∀ e s, StateBounded (roundStep e s).
```

So every stored state component remains inside the declared bounded domain after each round.

### 3. Harmony Preservation

Via `round_preserves_harmony`, the concrete instance proves

```text
∀ e s, Harmony (roundStep e s).
```

This is the main local “unification” theorem for the current model: the subordination relations between posterior, mean field, controller state, continuation scores, and corridor projection are preserved by the full step.

### 4. Freshness Honesty

Via `fresh_requires_refresh` and the concrete theorem `stale_without_refresh`, the model proves:

```text
e.refresh = unchanged
  ⇒ (roundStep e s).posterior.freshness = stale.
```

So freshness cannot be manufactured without an explicit refresh signal.

### 5. Unknown / Unreachable Non-Collapse

Via `unknown_signal_stays_unknown` and the concrete theorem `unknown_signal_not_collapsed`, the model proves:

```text
e.reachability = unknown
  ⇒ (roundStep e s).posterior.knowledge = unknown.
```

So the model does not silently collapse unknown reachability into unreachable reachability.

### 6. Explicit-Path Honesty

Via `explicit_projection_requires_explicit_knowledge` and the concrete theorem `corridor_projection_never_invents_explicit_path`, the model proves:

```text
(roundStep e s).projection.shape = explicitPath
  ⇒ (roundStep e s).posterior.knowledge = explicitPath.
```

This is the core honesty theorem for the current corridor model.

### 7. Multi-Layer Subordination

Via `multi_layer_projection_subordinate` and the concrete theorem `unified_round_subordinate`, the model proves:

```text
let s' := roundStep e s in
  s'.meanField.fieldStrength = s'.posterior.support
  ∧ s'.controller.stabilityMargin = s'.meanField.fieldStrength
  ∧ s'.projection.support ≤ s'.posterior.support.
```

This is the clearest current theorem that the field model is one observer-controller pipeline rather than several unrelated update routines.

### 8. Concrete Regime / Posture Examples

The instance also proves specific executable lemmas such as:

- `explicit_path_signal_yields_explicit_projection`
- `adversarial_corridor_signal_suppresses_posture`

These are not universal theorems over all inputs. They are witness lemmas showing that the composed transition behaves as intended on representative evidence configurations.

### 9. Stronger Local Conservativity And Monotonicity

The reduced instance now also proves a first batch of decision-relevant local theorems:

- `stronger_feedback_cannot_decrease_support`
  - stronger reverse feedback bonuses cannot reduce posterior support when the other evidence coordinates are fixed
- `explicit_refresh_does_not_increase_entropy`
  - explicit refresh does not increase the reduced uncertainty score relative to unchanged refresh
- `projection_support_le_primary_score`
  - the shared corridor support remains subordinate to the primary continuation score, not only to posterior support
- `adversarial_regime_implies_risk_suppressed`
- `unstable_regime_implies_risk_suppressed`
  - the simplified controller map is fail-safe under the two most defensive regimes
- `no_spontaneous_explicit_path_promotion`
  - one round cannot promote the shared projection to explicit-path unless explicit-path truth is already present in the incoming signal or preserved prior knowledge

The finite-belief refinement sharpens two of these statements:

- `projection_is_conservative_quotient_of_belief`
  - the public corridor projection cannot advertise more support than the private belief assigns to corridor-capable hypotheses
- `explicit_path_projection_requires_explicit_path_belief_mass`
  - explicit-path publication is grounded in the explicit-path component of the belief object, not only in the coarse `knowledge` enum

These are still reduced-model theorems, not global routing guarantees. They do, however, move the proof surface beyond pure shape invariants and into first-order controller conservativity.

### 10. Short Temporal And Stabilization Theorems

The current model now has three temporal layers:

- one-step theorems over `roundStep`
- short-horizon witness theorems over `roundTwice`
- repeated-evidence stabilization theorems over `runRepeatedEvidence`

The short-horizon layer includes:

- `roundTwice`
  - a small helper used to state explicit two-round scenarios over the deterministic local controller
- `repeated_unknown_evidence_stays_stale_and_opaque`
  - two consecutive `unknownEvidence` rounds keep the local state stale, preserve `unknown` knowledge, and force an opaque shared projection
- `repeated_unknown_evidence_never_promotes_explicit_path`
  - the same two-round scenario cannot drift into explicit-path publication
- `explicit_path_evidence_recovers_after_unknown_round`
  - strong explicit-path evidence can restore explicit-path projection after one unknown/stale round in the reduced model

The repeated-evidence stabilization layer includes:

- `runRepeatedEvidence`
  - iterates one fixed evidence object through the local round function
- `repeated_unknown_rounds_stabilize_opaque`
  - after the first unknown round, all later unknown rounds remain stale, unknown, and opaque
- `repeated_unknown_rounds_never_oscillate`
  - repeated unknown evidence cannot oscillate between incompatible public projection classes
- `repeated_explicit_path_rounds_preserve_projection`
  - repeated strong explicit-path evidence preserves explicit-path publication once it is reached
- `repeated_corridor_risk_rounds_stay_defensive`
  - repeated stale corridor-only high-risk evidence remains non-explicit and keeps the simplified controller risk-suppressed

These are still reduced-model theorems. They do not yet prove convergence, optimality, or asymptotic rates. They do show that the proof surface now contains genuine stabilization and no-oscillation statements rather than only one-step algebra.

### 11. Quantitative Ranking Candidate

The current model now names one paper-2-style ranking candidate:

```text
UncertaintyBurden(s)
  := s.posterior.entropy
   + s.controller.congestionPrice
   + s.regime.residual.
```

This quantity is intentionally modest. It is not yet called a Lyapunov function, because there is not yet a proved strict-descent theorem. It is an honest bounded proxy for residual uncertainty in the local belief state, control pressure still carried by the node, and unexplained residual misfit in the inferred regime.

The proved theorem `uncertainty_burden_bounded` shows only that this quantity stays finite on bounded local states. The intended next use is as a candidate ranking for recovery, stabilization, or regime-boundary arguments in the style of paper 2.

The model now also proves one first quantitative descent fact over this candidate:

```text
explicit_path_round_strictly_reduces_uncertainty_burden_from_initial.
```

This is intentionally narrow. It says that one explicit-path refresh step from the default bounded local state strictly lowers the ranking candidate. It is not yet a global convergence theorem or a general Lyapunov law, but it is stronger than mere boundedness.

### 12. Future Classical And Decision Hooks

The next natural mathematical hooks are `FiniteBelief`, which should eventually be compared against Telltale's `InformationCost.lean` and `ClassicalAnalysisAPI.lean` interfaces. `uncertaintyMass` is the current bounded surrogate that could later be replaced by a true Shannon-style quantity on a normalized finite distribution. `runRepeatedEvidence` gives the first concrete place to ask paper-2-style questions about descent, stabilization, and regime boundaries.

Any future use of `ClassicalAnalysisAPI` should follow the same trust-boundary pattern Telltale uses elsewhere: downstream field proofs should depend on the API surface, and any concrete real-analysis instantiation should live in a separate instance layer rather than leaking into the controller model.

One small decision-style property that now looks plausible is:

```text
given a bounded initial local state and a fixed bounded evidence alphabet,
does explicit-path publication occur within N steps?
```

That is still only a design note, not a completed theorem. It is useful because it can be phrased either as a direct bounded-step theorem or as a tiny quotient-exploration decision problem in the style of paper 2.

The current stack now includes the first such tiny decision layer in `Field/Model/Decision.lean`. It asks:

```text
can explicit-path publication occur in one round under a finite evidence alphabet?
```

and proves soundness and completeness for that one-step finite exploration procedure.

## Rust Mapping

The first Lean model intentionally captures only the proof-relevant semantic shape of the Rust field engine.

| Lean concept | Rust module | Mathematical role |
| --- | --- | --- |
| `EvidenceInput` | `crates/field/src/observer.rs`, `crates/field/src/summary.rs` | bounded observational input object |
| `PosteriorState` | `crates/field/src/observer.rs`, `crates/field/src/state.rs` | destination-local belief state |
| `MeanFieldState` | `crates/field/src/control.rs`, `crates/field/src/state.rs` | low-order compression of local field conditions |
| `ControllerState` | `crates/field/src/control.rs`, `crates/field/src/state.rs` | slow control variables |
| `RegimeState` | `crates/field/src/control.rs`, `crates/field/src/state.rs` | inferred explanatory regime |
| `PostureState` | `crates/field/src/control.rs`, `crates/field/src/state.rs` | control stance induced by the regime |
| `ScoredContinuationSet` | `crates/field/src/planner.rs`, `crates/field/src/attractor.rs` | bounded ranking of continuation options |
| `CorridorEnvelopeProjection` | `crates/field/src/observer.rs`, `crates/field/src/planner.rs`, `crates/field/src/route.rs` | conservative shared claim derived from local state |

## Deliberate Omissions

The first Lean model does not encode per-neighbor frontier entries, route backend tokens, route admission or publication semantics, checkpointing, retries, transport-driver queues, full summary provenance classes, or destination cache eviction and active-route maintenance. None of these is required to state the first boundedness, honesty, and harmony theorems for the local model.

## API / Instance Trust Boundary

The trust boundary follows the same `API` / `Instance` pattern used elsewhere in Telltale. `Model/API.lean` defines the state-space vocabulary, abstract operations, abstract law bundles, and stable wrappers used by downstream proofs. `Model/Instance.lean` defines one concrete bounded realization with its executable examples and proofs of the declared laws. Downstream proofs should depend on the API surface unless they explicitly need the first concrete bounded realization. This keeps the first bucketized model from becoming an accidental long-term semantic commitment.

## Assumption Envelope

The current theorem envelope is intentionally narrow. Universal theorems cover determinism, boundedness preservation, harmony preservation, freshness honesty, unknown/unreachable non-collapse, explicit-path honesty, and multi-layer subordination. The more concrete example lemmas are witness theorems for specific evidence cases, not universal behavioral claims.

The current theorem set should be read as structural groundwork for a later information-theoretic strengthening. At this stage the surface contains one-step structural invariants, stronger one-step conservativity and monotonicity lemmas, and a very small two-round temporal envelope. The assumption structure is still small enough to keep explicit in individual theorem statements and design notes rather than using `Distributed` family packaging.
