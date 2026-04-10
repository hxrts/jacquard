# Field Private Protocol

## Purpose

The field private protocol models the cooperative summary-exchange layer that may later inherit deeper Telltale proofs. It is not a second routing algorithm. The deterministic local controller in `Field/Model` remains the semantic owner of posterior state, regime inference, posture choice, continuation scoring, and public corridor projection.

The private protocol may contribute only observational summary facts. It may not publish canonical route truth.

## Reduced Protocol Surface

The current reduced protocol lives in `Field/Protocol/API.lean` and `Field/Protocol/Instance.lean`.

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

This is the protocol-machine-adjacent layer that later Telltale reuse should grow through.
It is also the protocol-side object consumed by the runtime adequacy layer's fragment-trace refinement theorems.

### `Protocol/Conservation.lean`

Packages field-side conservation statements:

- evidence conservation
- snapshot authority conservation
- fragment-trace authority conservation
- replay-equivalent fragment traces preserve controller-visible evidence

The current file is a mixture of direct-family style statements and small field-local glue, and it now says so honestly.

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

The important current result is still intentionally small:

```text
subtype_replacement_style_receive_refinement
```

This is Telltale-shaped, but it is not yet a full direct instantiation of the generic subtype-replacement kernel.

### `Protocol/Reconfiguration.lean`

Makes the current limitation explicit:

- the reduced protocol is fixed-participant
- reconfiguration is not part of the current semantics

## What Is Proved Today

The current protocol stack proves:

- global-to-local harmony for the reduced choreography
- bounded machine stepping
- fail-closed cancellation
- observational-only export
- field-side conservation over exports and replay-visible semantic objects
- reduced coherence cases
- a narrow receive-refinement theorem
- absence of reconfiguration semantics in the current reduced protocol

## Current Integration Points

The current implementation uses the protocol layer in two main downstream places:

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
- it has a receive-refinement surface deliberately shaped toward subtype-replacement style reasoning

## What Is Not Yet Fully Telltale-Derived

The current protocol is not yet fully Telltale-native.

It does not yet provide:

- a direct import of the full generic projection proof surface
- a full generic subtype-replacement instantiation
- a full protocol-machine adequacy theorem
- a full reconfiguration or delegation story

So the right characterization is:

- Telltale-shaped and partially family-aligned
- not yet a full direct inheritance of the deeper Telltale proof stack

## Rust Mapping

| Lean concept | Rust-side analogue | Notes |
|---|---|---|
| `MachineSnapshot` | reduced choreography/runtime state | Lean keeps only proof-relevant controller-facing fields |
| `MachineInput` | polling, summary receipt, ack receipt, cancellation | reduced to four bounded cases |
| `ProtocolOutput` | host-facing private summary batch | observational-only |
| `ProtocolSemanticObject` | replay-visible private export | authority remains observational-only |
| `HostDisposition` | private protocol round disposition | running, blocked, complete, failed-closed |

## What The Protocol Does Not Prove

The current protocol layer does not prove:

- canonical route publication
- router lifecycle correctness
- end-to-end stabilization
- planner correctness
- transport correctness
- full Rust choreography correctness

Those claims must remain outside this module family until the proof objects actually justify them.

## Where To Extend Next

The most useful next protocol extensions are:

- deeper direct instantiation of Telltale projection families
- a stronger receive-refinement story
- richer replay-object and observer-projection proofs
- tighter connection to runtime adequacy families

Until then, this document should be read as the specification of the current reduced private protocol boundary and its present Telltale alignment.
