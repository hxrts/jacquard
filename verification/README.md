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
  - `FieldModelAPI.lean`
  - `FieldModelInstance.lean`
  - `FieldModelNotes.md`
- private choreography / protocol changes:
  - `FieldProtocolAPI.lean`
  - `FieldProtocolInstance.lean`
  - `FieldProtocolNotes.md`
- boundary or parity changes:
  - `FieldBoundary.lean`
  - `FieldParity.md`

## Read This First

- `FieldModelNotes.md`
- `FieldProtocolNotes.md`
- `FieldParity.md`
