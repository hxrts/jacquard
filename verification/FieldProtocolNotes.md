# Field Private Protocol Notes

This note specifies the reduced private cooperative protocol used by the first
Lean field proof boundary.

## Purpose

The choreography model exists to formalize bounded cooperative summary
exchange. It is not a second routing algorithm.

## Reduced Protocol Spec

The first reduced protocol covers:

- summary exchange
- anti-entropy acknowledgement
- bounded step budgets
- fail-closed cancellation

It does not try to encode the whole Rust runtime from
`crates/field/src/choreography.rs`.

## Roles And Message Classes

The first reduced choreography has two roles:

- `controller`
- `neighbor`

The first reduced message classes are:

- `summaryDelta`
- `antiEntropyAck`

These are enough to exercise projection, bounded stepping, and observational
export without importing every protocol kind from the Rust engine.

## Observational Outputs

The only host-visible protocol product in the first model is an
`ObservedSummaryBatch`.

The reduced protocol may export:

- accepted summary batch counts
- blocked receive markers
- fail-closed disposition

It may not export canonical route truth.

## Fail-Closed Policy

The first reduced protocol requires:

- a bounded step budget
- bounded emitted summary counts
- explicit fail-closed cancellation
- no exports after fail-closed termination

## API / Instance Split

`FieldProtocolAPI.lean` defines:

- protocol roles
- message labels
- machine inputs
- observational outputs
- abstract projection, machine advance, and export operations
- abstract laws for harmony, bounded stepping, fail-closed cancellation, and observational-only export

`FieldProtocolInstance.lean` defines:

- the first reduced summary-exchange action list
- the first concrete local projections
- the first bounded machine transition
- the first concrete export policy

Downstream proofs should depend on the API surface unless they explicitly need
the reduced concrete choreography.

## Rust Mapping Note

The reduced protocol maps to the current field engine like this:

| Lean concept | Rust module | Notes |
| --- | --- | --- |
| `MachineSnapshot` | `crates/field/src/choreography.rs` | Lean keeps only step budget, blocked receive, disposition, and emitted count. |
| `MachineInput` | `crates/field/src/choreography.rs`, `crates/field/src/runtime.rs` | Lean collapses polling, summary receipt, acknowledgement, and cancellation into four bounded machine inputs. |
| `ObservedSummaryBatch` | `crates/field/src/summary.rs`, `crates/field/src/runtime.rs` | Lean exports only bounded observational summary batches. |
| `HostDisposition` | `crates/field/src/choreography.rs` | Lean keeps the host-visible control state: running, blocked, complete, or failed closed. |

## Imported Telltale Proof Families

The field protocol work is intended to sit on these Telltale families:

- `Choreography/Projection/*`
  - gives the projection story for the global summary-exchange choreography
- `Choreography/Harmony/*`
  - gives harmony between the global choreography and local role projections
- `Protocol/Coherence/*`
  - gives coherence for buffered asynchronous local execution
- `Protocol/Typing/*`
  - gives local typing obligations for projected protocols
- `Protocol/Preservation.lean`
  - gives preservation across protocol-machine steps
- `Protocol/Determinism.lean`
  - gives deterministic step reasoning where applicable
- `Runtime/ProtocolMachine/Model/*`
  - gives the executable protocol-machine model
- `Runtime/ProtocolMachine/Semantics/*`
  - gives step semantics for replay-visible machine transitions
- `Runtime/Adequacy/*`
  - gives adequacy between model and execution
- replay / authority conservation families
  - give the authority and replay-honesty story needed for simulator use

The reduced instance is the smallest field-specific object that can later be
ported onto those families without changing the ownership boundary.

For the current reduced model, direct reuse of the deeper conservation /
authority families is not yet necessary beyond the explicit
`OutputAuthority.observationalOnly` boundary. Those richer families should be
pulled in once the field protocol exports replay-visible semantic objects that
need the full conservation story.

## What The Protocol Layer Proves

The first reduced protocol proves:

- projection harmony for the two local roles
- bounded machine stepping
- fail-closed cancellation
- observational-only export

It does not prove:

- canonical route publication
- planner correctness
- router lifecycle semantics
- global field optimality
