# Field Verification Code Map

This map describes the current organization of `verification/Field`.

## Top-Level Theorem Packs

- `Field/LocalModel.lean`
  - imports the local observer-controller model, the finite-belief information layer, and the first decision procedure
- `Field/PrivateProtocol.lean`
  - imports the reduced private choreography/runtime layer and the Telltale-shaped protocol bridge
- `Field/Boundary.lean`
  - imports the observational controller-boundary theorems
- `Field/Adequacy.lean`
  - imports the first Rust-runtime adequacy bridge
- `Field/Field.lean`
  - umbrella import for the whole current field verification stack

## Local Model

- `Field/Model/API.lean`
  - semantic state vocabulary, abstract round-step operations, boundedness/harmony laws
- `Field/Model/Instance.lean`
  - first bounded concrete realization, structural theorems, temporal theorems, and first quantitative ranking law
- `Field/Model/Decision.lean`
  - one-step finite exploration / decision procedure over a small evidence alphabet

## Information Layer

- `Field/Information/API.lean`
  - abstract normalization and information-theoretic operations over `FiniteBelief`
- `Field/Information/Instance.lean`
  - first concrete weight-normalized distribution and entropy/mass theorems
- `Field/Information/Blindness.lean`
  - field-side information-cost / blindness bridge over the normalized public projection

## Private Protocol

- `Field/Protocol/API.lean`
  - reduced protocol roles, labels, machine state, global choreography, abstract projection/step/export laws
- `Field/Protocol/Instance.lean`
  - first reduced summary-exchange instance
- `Field/Protocol/Bridge.lean`
  - Telltale-shaped reduced protocol-machine fragment and replay/observer bridge
- `Field/Protocol/Conservation.lean`
  - field-side conservation pack for evidence, authority, and replay-equivalent fragment traces
- `Field/Protocol/Coherence.lean`
  - reduced updated-edge / incident-edge / unrelated-edge coherence lemmas
- `Field/Protocol/ReceiveRefinement.lean`
  - first typed receive-refinement hook aligned to subtype-replacement shape
- `Field/Protocol/Reconfiguration.lean`
  - fixed-participant audit note proving the current reduced protocol has no reconfiguration semantics

## Boundary And Adequacy

- `Field/Model/Boundary.lean`
  - controller-evidence boundary from protocol exports and traces
- `Field/Adequacy/API.lean`
  - abstract Rust-runtime artifact boundary
- `Field/Adequacy/Instance.lean`
  - first concrete runtime extraction, execution-level observational trace theorem, and evidence-agreement theorems
- `Field/Assumptions.lean`
  - packaged proof-contract assumptions for semantic and runtime-envelope theorems

## Notes

- `Field/Docs/Model.md`
  - mathematical description of the local field model
- `Field/Docs/Protocol.md`
  - protocol, Telltale mapping, and replay/authority notes
- `Field/Docs/Adequacy.md`
  - runtime artifact bridge and adequacy note
- `Field/Docs/TelltaleGap.md`
  - precise gap between current field objects and deeper Telltale proof reuse
- `Field/Docs/Parity.md`
  - Rust/Lean proof-relevant artifact boundary
- `Field/Docs/Extending.md`
  - contributor guidance

## Maturity Snapshot

- most mature:
  - local boundedness/harmony/honesty theorems
  - reduced private protocol and observational boundary
- moderate:
  - finite-belief information layer
  - normalized public-projection blindness bridge
  - one-step decision layer
  - reduced protocol-machine fragment
- earliest:
  - stronger runtime correctness theorem beyond the current execution-level extraction bridge
  - deeper Telltale-native reuse of conservation and subtype-replacement families
