# Field Verification Guide

## What This Stack Covers

The field verification stack currently has six proof surfaces:

- the deterministic local observer-controller model in `Field/Model`
- the information layer built on top of the finite belief object in `Field/Information`
- the private cooperative summary-exchange protocol in `Field/Protocol`
- the reduced finite network semantics in `Field/Network`
- the reduced router-facing publication/admission/installation semantics in `Field/Router`
- the runtime and assumption boundary in `Field/Adequacy` and `Field/Assumptions`

These surfaces are intentionally separated. The local controller is not a choreography. The private protocol does not own canonical route truth. The adequacy layer does not get to claim more than the runtime artifact boundary actually supports.

## Current Module Map

- `Docs/Model.md`
  - local state space, unified round semantics, information interpretation, decision layer, and blindness story
- `Docs/Protocol.md`
  - reduced choreography, protocol-machine surface, Telltale alignment, conservation/coherence/refinement story
- `Field/Network/*`
  - finite node/destination state, synchronous round buffer, and first network safety theorems
- `Field/Router/*`
  - router-facing publication, admission, and installation boundary
- `Docs/Adequacy.md`
  - runtime artifact boundary, reduced simulation witness, packaged assumptions, and parity-sensitive surfaces
- `Docs/Guide.md`
  - contributor guide and current maturity summary

The code map for the whole feature tree lives in `verification/Field/CODE_MAP.md`.

## What Is Proved Today

### Local Model

The local model currently gives:

- boundedness theorems for the destination-local state
- harmony theorems connecting posterior, mean-field, controller, regime, posture, scores, and public projection
- honesty theorems preventing public explicit-path claims without the right local knowledge
- small temporal theorems over repeated rounds
- one finite decision procedure over a representative evidence alphabet

### Information Layer

The information layer currently gives:

- a finite hypothesis space `FieldHypothesis`
- a finite belief object `FiniteBelief`
- a probability-simplex style wrapper `ProbabilitySimplexBelief`
- a concrete weight-normalized distribution with zero-mass fallback behavior
- first mass and entropy theorems
- a first public-projection blindness / erasure theorem

This is no longer only a bounded surrogate story, but it is still an early information layer rather than a full probabilistic routing theory.

### Private Protocol

The protocol layer currently gives:

- a reduced global choreography
- controller and neighbor projections
- bounded machine stepping
- fail-closed cancellation
- observational-only export
- field-side conservation and coherence theorem packs
- a narrow receive-refinement hook aligned to Telltale subtype-replacement style
- an explicit statement that the current reduced protocol has no reconfiguration semantics

### Network And Router Layers

The network/router layers currently give:

- a finite node vocabulary and finite destination-class vocabulary
- a reduced synchronous round buffer that republishes one public corridor projection per sender/destination slot
- an explicit delivered-message view that can later be refined by async delivery semantics
- router-facing publication candidates that are still distinct from canonical route truth
- reduced observed/admitted/rejected admission semantics
- a minimal installed-route object that only exists above admission
- first safety theorems showing:
  - local projection honesty lifts to published candidates
  - explicit-path installation cannot appear without explicit local knowledge
  - installed support remains conservative with respect to the supporting node's local evidence

### Async-Readiness Split

The current network object is deliberately synchronous, but the future async split is already fixed:

- protocol layer
  - private summary exchange, blocked receives, replay-visible protocol traces, and protocol-machine side conditions
- network layer
  - message delivery, delay, loss, retry, and neighbor-indexed transport assumptions over public publications
- adequacy layer
  - correspondence between Rust-facing runtime artifacts and the richer async protocol/network semantics

### Adequacy And Assumptions

The adequacy and assumptions layers currently give:

- reduced runtime artifacts
- extraction to reduced machine snapshots and traces
- evidence agreement between Rust-facing artifacts and Lean traces
- an explicit reduced simulation witness
- a packaged `ProofContract` for semantic, protocol, runtime, and optional strengthening assumptions

## What Is Not Proved

The current stack does not prove:

- global routing optimality
- router-owned canonical route correctness
- full Rust controller correctness
- full Rust choreography runtime correctness
- transport correctness
- asynchronous transport semantics
- large asymptotic mean-field or fluid-limit theorems

The current system is best read as:

- a strong reduced local-model and protocol-boundary proof stack
- an early but real information-theoretic layer
- a reduced runtime simulation bridge

## Maturity Summary

| Area | Status | Notes |
|---|---|---|
| Local model boundedness, harmony, honesty | Stable | main reduced semantic object is in place |
| Private protocol projection and observational boundary | Stable | reduced but structurally coherent |
| Conservation and coherence packs | Moderate | partly direct-family style, partly field-local glue |
| Receive refinement | Moderate | narrow subtype-replacement shaped result exists |
| Information layer | Moderate | finite normalized belief object and first blindness theorem exist |
| One-step decision layer | Early | useful but intentionally small |
| Reduced network and router layers | Moderate | explicit publication/admission/installation boundary and first safety theorems exist |
| Runtime adequacy | Early | reduced simulation witness, not full refinement |
| Packaged assumptions | Early | structure is in place, but theorem dependence is still selective |

## Ownership Rules

When adding new proofs, keep these boundaries intact.

- If the statement is about posterior, regime, posture, scores, or corridor projection, it belongs in `Field/Model` or `Field/Information`.
- If the statement is about choreography, projection, blocked receives, semantic objects, or protocol traces, it belongs in `Field/Protocol`.
- If the statement is about node-indexed local states, reduced message delivery, or network-level safety, it belongs in `Field/Network`.
- If the statement is about router-facing publication, admission, installation, or canonical handling eligibility, it belongs in `Field/Router`.
- If the statement is about protocol exports becoming controller evidence, it belongs in `Field/Model/Boundary`.
- If the statement is about Rust-facing runtime artifacts, extracted traces, or runtime simulation, it belongs in `Field/Adequacy`.
- If the statement is about the global assumption contract used across theorem packs, it belongs in `Field/Assumptions`.

## How To Extend The Stack

Use the API/instance pattern consistently.

1. Decide whether the new concept is an abstract interface or a first concrete realization.
2. Put proof-facing vocabulary and laws in the API file first if downstream proofs should depend on an abstraction.
3. Put the first concrete realization and instance-level proofs in the companion instance file.
4. Only then update downstream theorem packs.
5. Update the relevant field doc if the new result changes the public mental model of the stack.

## Telltale Reuse Discipline

The current field stack is Telltale-aligned, not fully Telltale-derived.

That means:

- use Telltale theorem-family structure where it genuinely fits
- do not restate Telltale theorems under field names unless the field theorem is genuinely narrower
- keep field-local glue explicit when the full generic Telltale theorem is not yet being instantiated directly
- do not overclaim proof reuse

`Docs/Protocol.md` is the authoritative place for the current Telltale alignment story.

## Contributor Checklist

Before landing a meaningful field-proof change:

1. Build the field root:
   - `nix develop --command bash -lc 'cd verification && lake build Field.Field'`
2. Check that the docs still match the code.
3. Update `verification/Field/CODE_MAP.md` if module responsibilities moved.
4. If the change affects the overall roadmap, update `work/lean.md` or `work/lean2.md`.
5. If the change affects Rust/Lean parity, update `Docs/Adequacy.md`.

## What To Avoid

- Do not move router-owned canonical truth into the protocol proof object.
- Do not force the deterministic controller into a choreography encoding.
- Do not claim full runtime adequacy when the actual theorem is a reduced simulation witness.
- Do not introduce transport-specific details into the local controller model unless they are genuinely proof-relevant there.
- Do not bypass the API/instance split just to make one downstream theorem shorter.
