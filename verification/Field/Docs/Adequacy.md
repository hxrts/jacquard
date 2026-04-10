# Field Adequacy and Parity

## What The Adequacy Layer Is

The adequacy layer is the first formal bridge between the Rust private protocol runtime in `crates/field/src/choreography.rs` and the reduced Lean protocol object. Its purpose is narrow: to prove that if a runtime round artifact stays inside the declared reduced envelope, the extracted Lean snapshot and trace satisfy the observational boundary. It is a structural bridge, not a full runtime correctness theorem.

## Runtime Artifact Shape

The narrowest Rust-facing artifact worth relating honestly today mirrors only the controller-relevant fields of `FieldChoreographyRoundResult`: the blocked receive marker, host disposition, emitted summary count, and remaining step budget. The adequacy layer intentionally erases the full Rust session map, artifact retention internals, outbound queue internals, and checkpoint payloads.

## Current Adequacy Claims

The current adequacy layer proves three things. First, if a runtime round artifact stays inside the declared envelope, the extracted Lean `MachineSnapshot` is bounded and coherent. Second, for the reduced artifact list, `runtimeEvidence artifacts = controllerEvidenceFromTrace (extractTrace artifacts)`, meaning the host-visible evidence batch derived from the runtime artifact list matches the batch derived from the corresponding Lean semantic-object trace. Third, for an admitted list of runtime round artifacts, the extracted Lean trace satisfies the reduced observational authority boundary: every semantic object remains observational-only, and the theorem is stated over the whole extracted trace rather than only one artifact.

## What This Does Not Prove

The current adequacy layer does not prove that the full Rust choreography runtime adheres to the reduced Lean machine on every execution. It does not prove scheduler correctness, checkpoint or recovery correctness, full replay exactness, or any claim about canonical route publication.

## Module Organization

`Adequacy/API.lean` declares the runtime-facing artifact boundary and the abstract adequacy obligations. `Adequacy/Instance.lean` gives the first concrete extraction, the execution-level observational trace theorem, and the reduced adequacy theorems. Downstream proofs should depend on the API surface unless they explicitly need the first concrete extraction. `Field/Assumptions.lean` packages the semantic and runtime-envelope assumptions into a `ProofContract` that upstream proofs can depend on without importing the full adequacy instance.

## Rust/Lean Parity

The following artifact shapes must not drift silently between the Rust and Lean representations.

| Artifact | Rust surface | Lean surface | Compatibility policy |
|---|---|---|---|
| Local field evidence shape | `observer.rs`, `summary.rs` | `FieldModelAPI.EvidenceInput` in `Model/API.lean` | Semantic drift requires explicit review and doc update |
| Corridor-envelope projection shape | `observer.rs`, `planner.rs`, `route.rs` | `FieldModelAPI.CorridorEnvelopeProjection` in `Model/API.lean` | Projection honesty must remain conservative across both sides |
| Protocol machine snapshot | `choreography.rs` | `FieldProtocolAPI.MachineSnapshot` in `Protocol/API.lean` | Additions must preserve the observational boundary |
| Protocol output batch | `summary.rs`, `runtime.rs` | `FieldProtocolAPI.ProtocolOutput` in `Protocol/API.lean` | Must never gain canonical route authority |
| Protocol-to-observer adapter | field-private adapter logic | `FieldBoundary.protocolOutputToEvidence` in `Model/Boundary.lean` | Must remain corridor-only and observational |

## When Rust Changes

Any change to a parity-sensitive artifact above requires verifying whether the Lean surface must change or can remain unchanged with a documented reason. Run `just lean-build` and `cargo test -p jacquard-field`, then update the parity table if field names, ownership, or compatibility policy changed. Check the adequacy layer specifically: does the narrow runtime artifact still match the fields of `FieldChoreographyRoundResult` being extracted? Does the Lean extraction still produce the same controller-visible evidence batch? Did a new runtime field become proof-relevant, or is it still intentionally erased? Is the layer still exporting only observational protocol facts rather than canonical route truth?

## Classical Scoping Note

Later classical work should target mean-field compression assumptions, stability envelopes for regime adaptation, and bounded backpressure and congestion response. Before serious classical theorems are realistic, the Rust field controller will likely need cleaner abstraction points for explicit residual models, destination-class aggregation assumptions, pressure and reward functions, and explicit separation between observational evidence and control priors. That work is out of scope for the first bounded field model.
