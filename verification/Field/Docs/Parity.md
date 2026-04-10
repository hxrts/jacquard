# Field Rust / Lean Parity Table

This note records the proof-relevant artifacts whose Rust and Lean shapes must
not drift silently.

## Parity Table

| Artifact | Owner | Rust surface | Lean surface | Serialization / representation | Compatibility policy |
| --- | --- | --- | --- | --- | --- |
| Local field evidence shape | `crates/field` | `observer.rs`, `summary.rs` | `FieldModelAPI.EvidenceInput` in `verification/Field/Model/API.lean` | bounded discrete fields, no transport payloads | semantic drift requires explicit review and note update |
| Corridor-envelope projection shape | `crates/field` | `observer.rs`, `planner.rs`, `route.rs` | `FieldModelAPI.CorridorEnvelopeProjection` in `verification/Field/Model/API.lean` | bounded support plus hop band | projection honesty must stay conservative across both sides |
| Protocol machine snapshot | `crates/field` | `choreography.rs` | `FieldProtocolAPI.MachineSnapshot` in `verification/Field/Protocol/API.lean` | bounded budget / blocked / disposition / emitted count | additions must preserve observational boundary |
| Protocol output batch | `crates/field` | `summary.rs`, `runtime.rs` | `FieldProtocolAPI.ProtocolOutput` in `verification/Field/Protocol/API.lean` | observational-only batch count | must never gain canonical route authority |
| Protocol-to-observer adapter | `crates/field` | field-private adapter logic | `FieldBoundary.protocolOutputToEvidence` in `verification/Field/Model/Boundary.lean` | bounded evidence projection | must remain corridor-only / observational |

## Checklist Gate

Any change to a parity-sensitive field above requires:

1. update the Rust code
2. update the Lean surface or document why it remains unchanged
3. run `just lean-build`
4. run `cargo test -p jacquard-field`
5. update this table if field names, ownership, or compatibility policy changed

This is the current gate for parity-sensitive field artifacts.

## Classical / Mean-Field Scoping Note

Later classical work should target:

- mean-field compression assumptions
- stability envelopes for regime adaptation
- bounded backpressure / congestion response

Before serious classical theorems are realistic, the Rust field controller will
likely need cleaner abstraction points for:

- explicit residual models
- explicit destination-class aggregation assumptions
- explicit pressure / reward functions
- explicit separation between observational evidence and control priors

That work is deliberately out of scope for the first bounded field model.
