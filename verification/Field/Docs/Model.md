# Field Local Model Specification

This note states the first formal object for the field engine in more
mathematical terms and records how it relates to the Rust implementation.

The purpose of this document is not to restate the Lean files line by line. The
purpose is to make the proof object explicit:

- what the state space is
- what one local round means
- which invariants are part of the model
- which theorems have actually been established

## Scope

The current Lean model is a destination-local state machine for:

- one node
- one destination class
- one deterministic round step

It is deliberately:

- transport-agnostic
- bounded
- discrete
- smaller than the production Rust field engine

It does not yet model:

- a network of nodes
- asynchronous transport behavior
- router publication semantics
- route admission semantics
- end-to-end convergence or optimality

## State Space

Let the bounded scalar domain be

```text
B := { n ∈ Nat | 0 ≤ n ≤ 1000 }.
```

The current concrete instance represents all scalar quantities with clamped
natural numbers in `B`.

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
  B × B × ObservationFreshness × ReachabilityKnowledge

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

The current model uses `Nat` for hop bounds, but the concrete instance fixes
them to a tiny finite family of bands:

- explicit path: `(2, 2)`
- corridor envelope: `(1, 3)`
- opaque: `(0, 4)`

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

This matters because the model is not intended to be a loose federation of
submodels. The Lean theorem surface is organized around the composed transition,
and the subfunctions are introduced only to name the internal semantics of that
single round.

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

The `Harmony` predicate captures the intended subordination relations between
the layers of the local observer-controller.

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

The first model treats the following distinctions as proof-relevant, not merely
descriptive:

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

This is a basic sanity theorem: the local round is a pure function with no
ambient nondeterminism.

### 2. Boundedness Preservation

Via `round_preserves_bounded`, the concrete instance proves

```text
∀ e s, StateBounded (roundStep e s).
```

So every stored state component remains inside the declared bounded domain after
each round.

### 3. Harmony Preservation

Via `round_preserves_harmony`, the concrete instance proves

```text
∀ e s, Harmony (roundStep e s).
```

This is the main local “unification” theorem for the current model: the
subordination relations between posterior, mean field, controller state,
continuation scores, and corridor projection are preserved by the full step.

### 4. Freshness Honesty

Via `fresh_requires_refresh` and the concrete theorem `stale_without_refresh`,
the model proves:

```text
e.refresh = unchanged
  ⇒ (roundStep e s).posterior.freshness = stale.
```

So freshness cannot be manufactured without an explicit refresh signal.

### 5. Unknown / Unreachable Non-Collapse

Via `unknown_signal_stays_unknown` and the concrete theorem
`unknown_signal_not_collapsed`, the model proves:

```text
e.reachability = unknown
  ⇒ (roundStep e s).posterior.knowledge = unknown.
```

So the model does not silently collapse unknown reachability into unreachable
reachability.

### 6. Explicit-Path Honesty

Via `explicit_projection_requires_explicit_knowledge` and the concrete theorem
`corridor_projection_never_invents_explicit_path`, the model proves:

```text
(roundStep e s).projection.shape = explicitPath
  ⇒ (roundStep e s).posterior.knowledge = explicitPath.
```

This is the core honesty theorem for the current corridor model.

### 7. Multi-Layer Subordination

Via `multi_layer_projection_subordinate` and the concrete theorem
`unified_round_subordinate`, the model proves:

```text
let s' := roundStep e s in
  s'.meanField.fieldStrength = s'.posterior.support
  ∧ s'.controller.stabilityMargin = s'.meanField.fieldStrength
  ∧ s'.projection.support ≤ s'.posterior.support.
```

This is the clearest current theorem that the field model is one observer-
controller pipeline rather than several unrelated update routines.

### 8. Concrete Regime / Posture Examples

The instance also proves specific executable lemmas such as:

- `explicit_path_signal_yields_explicit_projection`
- `adversarial_corridor_signal_suppresses_posture`

These are not universal theorems over all inputs. They are witness lemmas
showing that the composed transition behaves as intended on representative
evidence configurations.

## Rust Mapping

The first Lean model intentionally captures only the proof-relevant semantic
shape of the Rust field engine.

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

The first Lean model does not encode:

- per-neighbor frontier entries
- route backend tokens
- route admission or route publication semantics
- checkpointing, retries, or transport-driver queues
- full summary provenance classes
- destination cache eviction or active-route maintenance

These omissions are intentional. None of them is required to state the first
boundedness, honesty, and harmony theorems for the local model.

## API / Instance Trust Boundary

The trust boundary follows the same `API` / `Instance` pattern used elsewhere
in Telltale.

`verification/Field/Model/API.lean` defines:

- the state-space vocabulary
- the abstract operations
- the abstract law bundles
- the stable wrappers used by downstream proofs

`verification/Field/Model/Instance.lean` defines:

- one concrete bounded realization
- its executable examples
- its proofs of the declared laws

Downstream proofs should depend on the API surface unless they explicitly need
the first concrete bounded realization. This keeps the first bucketized model
from becoming an accidental long-term semantic commitment.

## Assumption Envelope

The current theorem envelope is intentionally narrow.

Universal theorems are limited to:

- determinism
- boundedness preservation
- harmony preservation
- freshness honesty
- unknown/unreachable non-collapse
- explicit-path honesty
- multi-layer subordination

The more concrete example lemmas are witness theorems for specific evidence
cases, not universal behavioral claims.

The current model does not yet use `Distributed` family packaging for
assumptions. The assumption structure is still small enough to keep explicit in
individual theorem statements and design notes.
