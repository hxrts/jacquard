# Field Verification Notes

This directory contains the first formal model and proof notes for the field
engine.

## What Is Proved

Today the Lean work covers:

- a bounded deterministic local field model
- first boundedness and honesty theorems for one local round
- a reduced private summary-exchange protocol instance
- a narrow proof boundary showing protocol exports stay observational when
  turned into local evidence

## What Is Not Proved

This work does not prove:

- global routing optimality
- full Rust controller correctness
- canonical route publication
- router lifecycle semantics
- BLE-specific or transport-specific protocol behavior

## Where New Proof Work Should Land

- local observer-controller model changes:
  - `verification/Field/Model/API.lean`
  - `verification/Field/Model/Instance.lean`
  - `verification/Field/Docs/Model.md`
- private choreography / protocol changes:
  - `verification/Field/Protocol/API.lean`
  - `verification/Field/Protocol/Instance.lean`
  - `verification/Field/Docs/Protocol.md`
- boundary or parity changes:
  - `verification/Field/Model/Boundary.lean`
  - `verification/Field/Docs/Parity.md`

## Read This First

- `verification/Field/Docs/Model.md`
- `verification/Field/Docs/Protocol.md`
- `verification/Field/Docs/Parity.md`
