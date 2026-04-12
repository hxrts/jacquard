# Field Private Protocol

## Purpose

The field private protocol models the cooperative summary-exchange layer as a
deliberately reduced proof object. It is not a second routing algorithm. The
deterministic local controller in `Field/Model` remains the semantic owner of
posterior state, regime inference, posture choice, continuation scoring, and
public corridor projection.

The private protocol may contribute only observational summary facts. It may not publish canonical route truth.

The current design decision is explicit: this protocol remains deliberately
reduced. The field search substrate now has its own proof-facing object in
`Field/Search/API.lean`, so the protocol is not being expanded into a second
owner of search semantics or route truth. Reconfiguration is now modeled
explicitly, but only as an observational reduced object over owner transfer,
checkpoint/restore, and continuation shift with a fixed participant set.

## Reduced Protocol Surface

The reduced protocol lives in:

- `Field/Protocol/API.lean`
- `Field/Protocol/Instance.lean`
- `Field/Protocol/Fixtures.lean`
- `Field/Protocol/Closure.lean`

It has:

- two roles
  - `controller`
  - `neighbor`
- two summary labels
  - `summaryDelta`
  - `antiEntropyAck`
- four machine inputs
  - `poll`
  - `receiveSummary`
  - `receiveAck`
  - `cancel`

The protocol state is reduced to:

```text
MachineSnapshot :=
  stepBudgetRemaining : Nat
  blockedOn : Option SummaryLabel
  disposition : HostDisposition
  emittedCount : Nat
```

This is deliberately smaller than the Rust choreography runtime. It erases session maps, queue internals, checkpoint payloads, and transport-local state.

That reduction is now more visibly deliberate because the Rust runtime carries
more protocol surface than this proof object: multiple protocol kinds, bounded
artifact retention, host-wait status, and per-round route-adjacent observational
artifacts. The reduced Lean protocol still keeps only the proof-relevant
summary/ack machine boundary.

## What The Protocol Exports

The protocol exports two host-facing observational objects:

- `ProtocolOutput`
  - the small controller-facing batch object
- `ProtocolSemanticObject`
  - the replay-visible semantic object used by trace-level proofs

Both remain observational-only. They may carry:

- summary batch counts
- host disposition
- observational-only authority

They may not carry canonical route authority.

## Current Law Surface

The protocol API exposes laws for:

- projection harmony
- projection from the global choreography
- bounded stepping
- coherence preservation
- fail-closed cancellation
- observational-only export

The concrete instance proves those laws for the reduced two-role summary-exchange choreography.

## Module Breakdown

### `Protocol/API.lean`

Defines:

- roles
- labels
- machine inputs
- host disposition
- protocol outputs
- semantic objects
- global choreography
- machine snapshots
- abstract projection, step, and export operations
- law interfaces

Downstream proofs should depend on this surface unless they need the first concrete realization.

### `Protocol/Instance.lean`

Defines the first reduced summary-exchange protocol instance:

- the concrete action list
- local projections
- bounded machine stepping
- concrete export policy

It is intentionally proof-relevant rather than implementation-complete.

### `Protocol/Bridge.lean`

Provides the first reduced Telltale-shaped bridge:

- fragment traces
- fragment semantic objects
- replay-equivalence and observer-style bridge lemmas
- snapshot-trace erasure lemmas connecting reduced machine snapshots to fragment traces

This is the protocol-machine-adjacent layer consumed by the runtime adequacy
layer's fragment-trace refinement theorems.

### `Protocol/Conservation.lean`

Packages field-side conservation statements:

- evidence conservation
- snapshot authority conservation
- fragment-trace authority conservation
- replay-equivalent fragment traces preserve controller-visible evidence

This file is the direct-family aligned conservation pack for the reduced field
boundary, plus the small field-local glue needed to connect it back to the
controller-boundary theorems.

### `Protocol/Coherence.lean`

Carries the reduced coherence story for the two-role summary-exchange machine:

- updated-edge style behavior
- incident-edge style behavior
- unrelated-edge style behavior

This is a reduced analogue of the operational coherence kernel, not the full generic Telltale proof.

### `Protocol/ReceiveRefinement.lean`

Introduces a narrow receive-refinement hook:

- `RefinedReceive`
- refined labels and refined inputs
- a subtype-replacement shaped theorem surface over summary and ack receives

The important theorem surface is:

```text
subtype_replacement_style_receive_refinement
refined_receive_has_subtype_replacement_witness
subtype_replacement_witness_preserves_observational_boundary
```

This is the reduced subtype-replacement boundary used by Field. It keeps the
receive refinement object explicit without promoting the protocol into a full
implementation-complete machine semantics.

### `Protocol/Fixtures.lean`

Provides proof-facing concrete protocol examples:

- one representative summary/ack exchange snapshot list
- fragment-trace / observer-projection agreement on that exchange
- concrete receive-refinement witnesses
- fixed-participant and supported-reconfiguration fixtures

### `Protocol/Closure.lean`

Packages the final reduced protocol-boundary statement:

- reduced Telltale-family alignment
- closed receive-refinement witness coverage
- fixed-participant choreography
- observational-only reconfiguration semantics

### `Protocol/Reconfiguration.lean`

Makes the final protocol boundary explicit:

- the reduced protocol is fixed-participant
- reconfiguration is explicit and observational-only
- supported reconfiguration does not own route truth

## What Is Proved Today

The current protocol stack proves:

- global-to-local harmony for the reduced choreography
- bounded machine stepping
- fail-closed cancellation
- observational-only export
- field-side conservation over exports and replay-visible semantic objects
- reduced coherence cases
- a narrow receive-refinement theorem
- explicit observational-only reconfiguration semantics in the reduced protocol
- closed receive-refinement witnesses for the two receive forms
- a final reduced protocol-boundary theorem pack in `Protocol/Closure.lean`

## Current Integration Points

The protocol layer is used in two main downstream places:

- `Field/Model/Boundary.lean`
  - turns protocol outputs and semantic objects into controller-visible evidence while proving the exports stay observational-only
- `Field/Adequacy/Instance.lean`
  - connects reduced runtime artifacts to the protocol layer through extracted traces and the fragment-trace bridge

So the protocol layer is no longer only a standalone reduced choreography note. It is now the shared proof object sitting between the controller-boundary story and the runtime adequacy story.

## Telltale Alignment

The current field protocol is Telltale-aligned in several concrete ways:

- it has an explicit global choreography object
- it projects that choreography into local roles
- it uses a bounded machine snapshot and machine-input step model
- it has replay-visible semantic objects
- it phrases conservation and observer-style results in Telltale-compatible vocabulary where possible
- it has a receive-refinement surface with explicit subtype-replacement
  witnesses for both receive forms

## Final Boundary Statement

The right characterization is:

- directly aligned to the reduced Telltale-family surfaces the repo actually
  imports today
- intentionally reduced relative to the richer Rust choreography runtime
- intentionally fixed-participant
- intentionally reconfiguring only through observational owner-transfer,
  checkpoint/restore, and continuation-shift surfaces

What is out of scope for this protocol object is therefore explicit rather than
transitional:

- full Rust choreography correctness
- richer multi-kind runtime retention semantics
- delegation or participant-set reconfiguration
- any attempt to turn the protocol layer into a second owner of route truth

## Rust Mapping

| Lean concept | Rust-side analogue | Notes |
|---|---|---|
| `MachineSnapshot` | reduced choreography/runtime state | Lean keeps only proof-relevant controller-facing fields |
| `MachineInput` | polling, summary receipt, ack receipt, cancellation | reduced to four bounded cases |
| `ProtocolOutput` | host-facing private summary batch | observational-only |
| `ProtocolSemanticObject` | replay-visible private export | authority remains observational-only |
| `HostDisposition` | private protocol round disposition | running, blocked, complete, failed-closed |

The Rust choreography runtime currently contains more than this reduced proof
surface:

- multiple protocol kinds, not only the reduced summary/ack vocabulary
- explicit `FieldProtocolArtifact` retention for replay-oriented diagnostics
- `FieldRuntimeRoundArtifact` projections recorded alongside protocol rounds

Those richer Rust surfaces are intentionally consumed through reduction layers
such as `Field/Adequacy/*`; they do not mean the reduced protocol proof object
has become implementation-complete.

## What The Protocol Does Not Prove

The current protocol layer does not prove:

- canonical route publication
- router lifecycle correctness
- end-to-end stabilization
- planner correctness
- transport correctness
- full Rust choreography correctness

Those claims must remain outside this module family until the proof objects actually justify them.

This document should be read as the maintained specification of the reduced
private protocol boundary used by the field stack.
