# Field Private Protocol

## Purpose and Scope

The field private protocol models the cooperative summary-exchange layer that may later connect to richer Telltale choreography proofs. It is not a second routing algorithm. The deterministic local controller (the observer-controller model in `Field/Model`) remains the semantic owner of corridor belief and posture choice. The protocol may supply only observational summary inputs and may not publish canonical route truth.

This split is intentional. The controller is not a choreography, and forcing it into one would conflate two genuinely different proof obligations. The protocol surface exists precisely to bound what cooperative evidence may cross into the controller and to prove that nothing stronger gets through.

## Reduced Protocol Specification

The first reduced protocol covers summary exchange, anti-entropy acknowledgement, bounded step budgets, and fail-closed cancellation. It has two roles, controller and neighbor, and two message classes, summaryDelta and antiEntropyAck. These are sufficient to exercise projection, bounded stepping, and observational export without importing every protocol kind from the Rust engine.

The protocol does not try to encode the whole Rust runtime from `crates/field/src/choreography.rs`. The Rust engine has session maps, artifact retention, outbound queues, and checkpoint payloads. The reduced Lean protocol intentionally erases all of those fields.

## Observational Boundary

The reduced protocol exports two related host-facing objects. `ProtocolOutput` is the small adapter object used by the current controller boundary. `ProtocolSemanticObject` is the replay-visible object used by stronger trace-level boundary theorems. Both carry only accepted summary batch counts, blocked receive markers, fail-closed or running disposition, and observational-only authority tags. The protocol may not export canonical route truth.

## Fail-Closed Policy

The protocol requires a bounded step budget, bounded emitted summary counts, explicit fail-closed cancellation, and no exports after fail-closed termination.

## Module Organization

`Protocol/API.lean` defines the vocabulary: roles, message labels, machine inputs, observational outputs, abstract projection and export operations, and law interfaces for harmony, bounded stepping, fail-closed cancellation, and observational-only export. Downstream proofs should depend on this surface unless they explicitly need the concrete choreography.

`Protocol/Instance.lean` gives the first reduced realization: the summary-exchange action list, local projections for each role, the bounded machine transition, and the concrete export policy. It is intentionally smaller than the Rust runtime. The goal is the proof-relevant boundary, not a full engine replica.

`Protocol/Bridge.lean` connects the reduced protocol object to a Telltale-shaped machine fragment, defining the snapshot-to-fragment relation, fragment semantic-object traces, and a simulation-style bridge from reduced machine steps to reduced fragment steps. `Protocol/Conservation.lean` packages the field theorems that are direct conservation-family instantiations and keeps the remaining field-local glue explicit. `Protocol/Coherence.lean` proves the reduced updated-edge, incident-edge, and unrelated-edge style coherence cases. `Protocol/ReceiveRefinement.lean` introduces the smallest typed receive-refinement hook over summary and ack receives, with theorem shape chosen to stay close to the local `Consume` / subtype-replacement proof vocabulary. `Protocol/Reconfiguration.lean` audits the fixed-participant property and explicitly rules out reconfiguration from the current protocol semantics.

## Rust Mapping

| Lean concept | Rust module | Notes |
|---|---|---|
| `MachineSnapshot` | `choreography.rs` | Lean keeps only step budget, blocked receive, disposition, and emitted count |
| `MachineInput` | `choreography.rs`, `runtime.rs` | Lean collapses polling, summary receipt, acknowledgement, and cancellation into four bounded inputs |
| `ObservedSummaryBatch` | `summary.rs`, `runtime.rs` | Lean exports only bounded observational summary batches |
| `HostDisposition` | `choreography.rs` | running, blocked, complete, or failed closed |

## What The Protocol Proves

The first reduced protocol proves projection harmony for the two local roles, bounded machine stepping, fail-closed cancellation, observational-only export, field-side conservation over replay-visible exports, reduced incident-edge and unrelated-edge coherence cases, and a first typed receive-refinement hook.

The narrowest field analogue of the operational coherence kernel is `MachineCoherent`. In the reduced protocol, `blockedOn` plays the role of the active receive frontier, `disposition` tracks whether the machine is live, blocked, complete, or failed closed, and `stepBudgetRemaining` and `emittedCount` provide the bounded operational side conditions. This is not the full `Coherent(G,D)` object from Telltale, but it is the appropriate reduced analogue for a two-role summary-exchange protocol with a single active frontier at a time.

The protocol does not prove canonical route publication, planner correctness, router lifecycle semantics, global field optimality, or any claim tying the reduced Lean machine to the full Rust choreography runtime.

## Telltale Alignment

The current field protocol is already Telltale-aligned in several concrete ways. It has an explicit global choreography object and projects that object into local roles, giving a projection-harmony theorem for the reduced controller/neighbor protocol with explicit proofs that both roles are projected from the global choreography. It carries a bounded machine state with blocked receives and fail-closed termination in a protocol-machine style rather than an ad hoc controller-side state machine. It exports replay-visible semantic objects with a clean separation between private protocol execution and host-visible observational export. The field-side conservation and authority theorems are now phrased as direct Telltale-family style instantiations where possible, and the reduced coherence and receive-refinement packs keep any remaining field-local glue explicit.

The current protocol is not yet fully Telltale-native. The choreography is still a reduced field-specific object, not yet phrased through Telltale's proof-carrying projection API. The machine state is still field-reduced rather than explicitly related to the full Telltale protocol-machine state. The replay-visible semantic objects remain field-specific even though the conservation wrappers are now aligned. The receive-refinement hook is intentionally narrow: it uses a Telltale-shaped theorem surface without trying to re-prove the full generic subtype-replacement kernel inside Jacquard.

## Telltale Concept Map

| Field definition | Telltale concept | Most relevant modules |
|---|---|---|
| `GlobalChoreography` | choreography | `Choreography/Projection/*`, `Choreography/Harmony/*` |
| `projectChoreography`, `projectImpl` | projection / local type | `Choreography/Projection/Project/*` |
| `MachineSnapshot` | reduced protocol-machine state | `Runtime/ProtocolMachine/Model/*`, `Protocol/Coherence/*` |
| `MachineCoherent` | operational coherence analogue | `Protocol/Coherence/Consume.lean`, `Protocol/Coherence/EdgeCoherence.lean` |
| `advanceMachineImpl` | protocol-machine step | `Protocol/Preservation.lean`, `Runtime/ProtocolMachine/Semantics/*` |
| `ProtocolSemanticObject` | semantic object | `Runtime/Proofs/ProtocolMachine/SemanticObjects/ReplayFailureExactness.lean` |
| `ProtocolTrace` | trace / replay artifact | `Proofs/ObserverProjection.lean` |
| `OutputAuthority.observationalOnly` | observational authority token | `Proofs/Conservation/Authority.lean`, `Proofs/Conservation/Evidence.lean` |
| `controllerEvidenceFromTrace` | observer projection | `Proofs/ObserverProjection.lean` |

## Paper Relevance

Paper 1 (coherence, protocol, session types) is the most directly relevant today. The reduced machine's updated-edge stepping and blocked-receive coherence are field-specific analogues of paper 1's preservation architecture. The incident-edge and unrelated-edge cases are present but thin, because the reduced protocol has only one active summary-exchange frontier at a time. Receive-side evidence refinement should eventually hook into paper 1's subtype-replacement pattern only if the protocol starts carrying typed receive refinements stronger than the current fixed summary/ack labels.

Paper 2 (quantitative, mean-field, stability) is primarily relevant to the controller model rather than the protocol. The likely bridge points are the finite belief support and uncertainty summaries, the `UncertaintyBurden` ranking candidate, and bounded repeated-evidence traces. These are setup for later descent or finite-exploration theorems and do not run through the protocol layer.

Paper 3 (choreography, adequacy, reconfiguration) is relevant through the harmony and replay-visible export story. The current protocol is already using paper 3's most immediately relevant part: harmony from a global choreography to local projections, replay-visible semantic export, and an observational-only authority boundary. The reconfiguration and delegation machinery of paper 3 is not yet engaged. The fixed-participant audit in `Reconfiguration.lean` explicitly rules it out for now.

## Proof-Family Targets

The next Telltale proof families to engage are listed below.

| Family | Purpose |
|---|---|
| `Choreography/Projection/*`, `Choreography/Harmony/*` | global-to-local harmony |
| `Protocol/Coherence/*` (Consume, EdgeCoherence, Preservation, Unified, SubtypeReplacement) | full coherence kernel |
| `Protocol/Preservation.lean`, `Runtime/ProtocolMachine/Semantics/*` | machine stepping |
| `Runtime/Proofs/ProtocolMachine/SemanticObjects/ReplayFailureExactness.lean` | semantic-object story |
| `Proofs/ObserverProjection.lean`, conservation families | authority and replay honesty |
| `Runtime/Adequacy/*` | runtime bridge |

## Iris Boundary

If later field proofs need ghost state, runtime adherence, or VM-style obligations, they should follow the same pattern as Telltale: define a field-specific API first and keep any concrete extraction machinery in a separate instance layer. The goal is the same one used by `IrisExtractionAPI.lean` and `IrisExtractionInstance.lean`: downstream proofs should depend on the abstract logical boundary rather than on one concrete extracted runtime representation.
