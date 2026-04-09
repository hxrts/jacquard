# Field Local Model Notes

This note defines the first formal object for the field engine and records how
it maps onto the Rust implementation.

## Minimum Semantic State

The Lean local model covers one node and one destination-local round. The first
formal state is:

- `EvidenceInput`
- `PosteriorState`
- `MeanFieldState`
- `RegimeState`
- `PostureState`
- `ControllerState`
- `ScoredContinuationSet`
- `CorridorEnvelopeProjection`
- `LocalState`

The model is deliberately transport-agnostic. It does not encode BLE packets,
radio channels, transport mailbox state, or simulator-specific packet formats.

## Bounded Representation

The first concrete instance uses bounded `Nat` values clamped to the shared
permille budget `0..1000`.

- Posterior support and entropy are bounded.
- Mean-field strength and alignments are bounded.
- Controller price and stability margin are bounded.
- Regime residual is bounded.
- Continuation scores are bounded and preserve primary ≥ alternate.
- Corridor projection support is bounded and hop bounds preserve `lower ≤ upper`.

The first instance keeps the state finite and proof-friendly without trying to
mirror every Rust field.

## Representation Invariants

The first API and instance explicitly preserve:

- stale vs fresh evidence
- unknown vs unreachable knowledge
- corridor-only vs explicit-path knowledge
- projection support subordinate to posterior support
- explicit-path projection only when explicit-path knowledge is present

## Unified Model Note

The field model is treated as one local observer-controller pipeline, not a
bundle of unrelated mini-models.

One deterministic round is:

1. update posterior state from bounded evidence
2. compress posterior state into mean-field state
3. update controller state from mean-field pressure
4. infer operating regime
5. choose routing posture
6. score continuation options
7. project the strongest honest shared corridor envelope

This is why the Lean surface exposes both named subfunctions and one composed
`roundStep`.

## Rust Mapping Note

The first Lean model intentionally maps only to the proof-relevant semantic
shape of the Rust field engine.

| Lean concept | Rust module | Notes |
| --- | --- | --- |
| `EvidenceInput` | `crates/field/src/observer.rs`, `crates/field/src/summary.rs` | Lean collapses direct, forward, and reverse evidence into one bounded input object. |
| `PosteriorState` | `crates/field/src/observer.rs`, `crates/field/src/state.rs` | Lean keeps only bounded support, entropy, freshness, and knowledge. |
| `MeanFieldState` | `crates/field/src/control.rs`, `crates/field/src/state.rs` | Lean keeps the low-order summary fields used by control. |
| `ControllerState` | `crates/field/src/control.rs`, `crates/field/src/state.rs` | Lean keeps only congestion price and stability margin. |
| `RegimeState` | `crates/field/src/control.rs`, `crates/field/src/state.rs` | Lean keeps the inferred regime plus one bounded residual. |
| `PostureState` | `crates/field/src/control.rs`, `crates/field/src/state.rs` | Lean keeps the chosen posture as a finite enum. |
| `ScoredContinuationSet` | `crates/field/src/planner.rs`, `crates/field/src/attractor.rs` | Lean keeps only primary and alternate scores. |
| `CorridorEnvelopeProjection` | `crates/field/src/observer.rs`, `crates/field/src/planner.rs`, `crates/field/src/route.rs` | Lean models the conservative shared claim, not the full route witness. |

## Intentionally Omitted Rust Concepts

The first Lean model does not encode:

- per-neighbor frontier entries
- route backend tokens
- router admission or publication semantics
- checkpointing, retries, or transport-driver queues
- detailed summary provenance classes
- destination caches, eviction policy, or active-route maintenance

These stay out of the first model because they are not required for the first
honesty and boundedness theorems.

## API / Instance Trust Boundary

`FieldModelAPI.lean` defines the stable proof surface:

- abstract operations
- abstract law bundles
- stable wrappers consumed by downstream proofs

`FieldModelInstance.lean` defines only the first concrete bounded realization.

Downstream proofs should depend on the API surface unless they are explicitly
about the first bounded instance. That mirrors the Telltale `API` / `Instance`
pattern and prevents the first bucket encoding from becoming a hidden long-term
commitment.

## Theorem Summary

The first instance proves:

- `local_round_deterministic`
  - one local round is a deterministic pure function
- `unknown_signal_not_collapsed`
  - unknown reachability is not silently collapsed to unreachable
- `stale_without_refresh`
  - stale evidence cannot become fresh without explicit refresh
- `corridor_projection_never_invents_explicit_path`
  - the shared corridor projection cannot manufacture explicit-path truth
- `unified_round_subordinate`
  - mean-field, controller state, and shared projection remain subordinate to the posterior

These are first-order honesty and boundedness results, not global routing
optimality claims.

## Assumption Envelope

The stronger concrete theorems in the first instance are intentionally explicit
about their assumptions:

- `explicit_path_signal_yields_explicit_projection`
  - assumes the concrete `explicitPathEvidence` case
- `adversarial_corridor_signal_suppresses_posture`
  - assumes the concrete `adversarialEvidence` case

`Distributed` family packaging is not yet useful for the first bounded local
model. The current assumption envelope is still small enough to keep inline in
the theorem statements and notes.
