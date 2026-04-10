# Field Verification Guide

## What This Is

The field verification work covers three distinct proof surfaces: the deterministic local observer-controller model, the private cooperative summary-exchange protocol, and the boundary constraining what the protocol may contribute to the controller. These surfaces are deliberately kept separate. The controller is not a choreography, and the private protocol does not own canonical route truth.

## What Is Proved Today

The Lean stack covers a bounded deterministic local field model with first boundedness, honesty, and harmony theorems for one local round. A probability-simplex style finite-belief information API sits above that model with a first Shannon-style entropy theorem and a first field-side blindness / erasure result. A one-round finite decision procedure over a representative evidence alphabet is proved sound and complete.

On the protocol side, the stack has a reduced summary-exchange choreography with projection harmony, bounded machine stepping, fail-closed cancellation, and observational-only export. Field-side conservation, coherence, and receive-refinement packs aligned with Telltale's theorem-family structure are in place. A narrow adequacy bridge connects Rust-facing runtime round artifacts to the reduced Lean protocol object, including execution-level observational trace extraction, a reduced simulation witness, and host evidence agreement.

## What Is Not Proved

The current work does not prove global routing optimality, full Rust controller correctness, canonical route publication, router lifecycle semantics, or transport-specific protocol behavior. The adequacy bridge is still reduced rather than a full Rust runtime correctness theorem.

## Maturity

| Area | Status |
|---|---|
| Local model: boundedness, harmony, honesty | Stable |
| Private protocol: projection and coherence boundary | Stable |
| Observational-only protocol/controller boundary | Stable |
| Probability-simplex finite-belief information API | Early |
| Public-projection blindness bridge | Early |
| Bounded ranking candidate and first descent theorem | Early |
| One-round decision procedure | Early |
| Global choreography object and local role projection | Partial |
| Protocol-machine fragment and replay-visible semantic objects | Partial |
| Field-side conservation, coherence, and subtype-replacement hooks | Partial |
| Runtime artifact extraction and reduced simulation witness | Early |

## Ownership Split

The three proof surfaces must remain separate. Do not put router-owned canonical route truth into the private protocol proof object. Do not force the deterministic controller into a choreography encoding. Do not let runtime or transport details leak into the local controller model unless they are genuinely proof-relevant there.

A useful placement heuristic: proofs mentioning belief, regime, posture, continuation score, or corridor projection belong in `Field/Model` or `Field/Information`. Proofs mentioning choreography, projection, blocked receive, semantic objects, or replay traces belong in `Field/Protocol`. Proofs mentioning exported evidence batches or observational-only authority belong in `Field/Model/Boundary`. Proofs mentioning runtime artifacts from `crates/field/src` belong in `Field/Adequacy`.

## Where New Work Should Land

Local observer-controller model changes belong in `Field/Model/API.lean` and `Field/Model/Instance.lean`, with documentation updates in `Docs/Model.md`. Information-theoretic strengthening belongs in `Field/Information/API.lean`, `Field/Information/Instance.lean`, and `Field/Information/Blindness.lean`. Private choreography and protocol changes belong in the `Field/Protocol/` modules, with documentation in `Docs/Protocol.md`. Boundary and adequacy changes belong in `Field/Model/Boundary.lean`, the `Field/Adequacy/` modules, and `Field/Assumptions.lean`, with documentation in `Docs/Adequacy.md`.

## How To Add A New Proof

Decide which proof surface owns the statement. If downstream work should depend on an abstraction rather than a first concrete realization, refine or add to the API layer before adding the instance. Give the concrete proof in the companion instance or theorem module. Update the relevant doc in `Field/Docs/`, and if the theorem changes the public mental model update `work/lean.md`.

## What To Avoid

Do not add controller fields to the Lean model just because they exist in Rust. Do not restate a protocol theorem as a controller theorem when the controller only sees observational exports. Do not claim a full runtime adequacy theorem when what you have is a reduced simulation witness. Do not import concrete real-analysis or Iris machinery directly into downstream field proofs. Always go through an API/instance boundary. Do not add repeated theorem-local side conditions that are better expressed in `Field/Assumptions.lean`.
