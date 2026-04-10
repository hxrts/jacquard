# Field Adequacy

## Purpose

The adequacy layer is the first formal bridge between the Rust private field runtime and the reduced Lean protocol object. Its job is narrow and explicit: relate a small Rust-facing artifact shape to a reduced Lean machine snapshot and a reduced Lean protocol trace, then show that the host-visible controller evidence extracted from those artifacts agrees with the controller evidence extracted from the Lean trace.

This layer does not prove full Rust runtime correctness. It does not prove scheduler correctness, checkpoint correctness, transport correctness, or router correctness. It proves a reduced artifact-to-trace story that is honest about what information is preserved and what information is erased.

## Runtime Artifact Boundary

The adequacy boundary is defined in `Field/Adequacy/API.lean`.

The Rust-facing artifact is:

```text
RuntimeRoundArtifact :=
  blockedReceive : Option SummaryLabel
  disposition : HostDisposition
  emittedCount : Nat
  stepBudgetRemaining : Nat
```

This is intentionally much smaller than the real Rust choreography runtime. It mirrors only the controller-relevant fields of the private protocol round result:

- blocked receive frontier
- host disposition
- emitted summary count
- remaining step budget

It intentionally erases:

- session maps
- retention internals
- outbound queue internals
- checkpoint payloads
- transport-local state

## Reduced Adequacy Envelope

The adequacy API defines two envelope predicates:

```text
RuntimeArtifactAdmitted : RuntimeRoundArtifact → Prop
RuntimeExecutionAdmitted : List RuntimeRoundArtifact → Prop
```

The runtime artifact admission condition currently requires:

- `stepBudgetRemaining ≤ 8`
- `emittedCount ≤ 8`
- complete or failed-closed states must not claim a blocked receive
- blocked states must carry a blocked receive marker

The reduced trace envelope is:

```text
ProtocolTraceAdmitted : ProtocolTrace → Prop
```

and means that every replay-visible semantic object in the extracted trace remains observational-only.

## Concrete Extraction

The concrete extraction lives in `Field/Adequacy/Instance.lean`.

It defines:

- `extractSnapshotImpl : RuntimeRoundArtifact → MachineSnapshot`
- `extractTraceImpl : List RuntimeRoundArtifact → ProtocolTrace`
- `runtimeEvidenceImpl : List RuntimeRoundArtifact → List EvidenceInput`

The trace extraction is intentionally simple:

- each runtime artifact contributes one machine-input event
- if the artifact emits observational output, it contributes one semantic object carrying that summary count and disposition
- the full trace is the list-level concatenation of those chunks

This means the current adequacy story is trace-oriented rather than scheduler-oriented.

## What Is Proved

The current adequacy layer proves the following results.

### Snapshot-Level Admission

From `Field/Adequacy/API.lean` and `Field/Adequacy/Instance.lean`:

- admitted runtime artifacts extract to bounded snapshots
- admitted runtime artifacts extract to coherent snapshots

Concretely, the laws establish:

```text
RuntimeAdmittedImpliesBoundedAndCoherent
```

and the instance theorem

```text
admitted_runtime_artifact_extracts_to_protocol_snapshot
```

re-exports that result for the concrete extraction.

### Evidence Agreement

The adequacy instance proves:

```text
runtime_trace_evidence_matches_protocol_trace
```

This states that:

```text
runtimeEvidence artifacts =
  controllerEvidenceFromTrace (extractTrace artifacts)
```

So the host-visible evidence batch obtained from the reduced Rust-facing artifact list is exactly the same batch obtained from the extracted Lean semantic trace.

### Observational Trace Admission

The adequacy instance proves:

```text
admitted_runtime_execution_extracts_to_observational_trace
```

This shows that an admitted runtime execution extracts to a Lean trace whose semantic objects all remain observational-only.

### Reduced Simulation Witness

The adequacy API defines:

```text
RuntimeTraceSimulation (artifacts : List RuntimeRoundArtifact)
```

with fields:

- `trace`
- `trace_eq_extract`
- `trace_admitted`

The adequacy instance constructs the witness:

```text
admitted_runtime_execution_simulates_reduced_protocol
```

This is the current reduced simulation statement. It is a genuine witness object, not only a prose claim that “the extraction looks reasonable.”

The instance also proves:

```text
runtime_simulation_preserves_controller_evidence_batch
```

which ties that witness back to the same controller-visible evidence batch seen by the Rust-facing artifact list.

### Fragment-Trace Refinement

The adequacy layer now also connects extracted runtime traces to the reduced Telltale-shaped fragment trace used by `Field/Protocol/Bridge.lean`.

Concretely, the instance proves:

```text
runtime_execution_refines_fragment_trace
runtime_execution_refinement_preserves_fragment_observer_projection
```

These theorems say that the semantic objects extracted from the runtime artifact list are exactly the semantic objects seen after erasing the corresponding extracted snapshots into the protocol-machine fragment trace, and therefore induce the same controller-visible evidence.

## Assumptions Packaging

`Field/Assumptions.lean` packages the growing assumption boundary into:

- `SemanticAssumptions`
- `ProtocolEnvelopeAssumptions`
- `RuntimeEnvelopeAssumptions`
- `OptionalStrengtheningAssumptions`
- `ProofContract`

The important part is the runtime envelope:

```text
RuntimeEnvelopeAssumptions.admitted
RuntimeEnvelopeAssumptions.respectsReducedEnvelope
```

This prevents the assumptions package from silently admitting executions that the adequacy theorems cannot actually consume. The current packaged simulation witness is:

```text
contract_yields_runtime_trace_simulation
```

and it is deliberately a `def`, not a theorem returning only `Prop`, because it produces an actual reduced simulation witness object.

## What The Adequacy Layer Does Not Prove

The current adequacy layer still does not prove:

- full Rust choreography adherence to the reduced Lean machine on every execution
- fairness or scheduler properties
- checkpoint or recovery correctness
- replay exactness for the full Rust runtime
- correspondence with router-owned canonical route truth
- transport correctness

So the correct reading is:

- the adequacy layer proves a reduced artifact-to-trace simulation story
- it does not yet prove full implementation refinement

## Parity-Sensitive Surfaces

The most important Rust/Lean compatibility surfaces are:

| Lean surface | Rust-side analogue | Compatibility expectation |
|---|---|---|
| `FieldModelAPI.EvidenceInput` | field observer and summary shaping | semantic drift must be reviewed explicitly |
| `FieldProtocolAPI.MachineSnapshot` | private choreography round state projection | additions must preserve the observational boundary |
| `FieldProtocolAPI.ProtocolOutput` | host-visible private summary result | must remain observational-only |
| `FieldProtocolAPI.ProtocolSemanticObject` | replay-visible private protocol export | must not gain stronger authority |
| `FieldAdequacyAPI.RuntimeRoundArtifact` | reduced projection of `FieldChoreographyRoundResult`-like data | must stay aligned with the actual extraction used in Rust |

When any of these change, the adequacy layer must be reviewed first. The key questions are:

- is the changed field actually proof-relevant
- does the reduced artifact still capture the right host-visible boundary
- does evidence extraction still agree between the Rust-facing artifact list and the Lean trace
- is the layer still observational-only

## Where To Extend Next

The most likely next adequacy improvements are:

- a stronger simulation relation over richer runtime artifacts
- tighter replay correspondence
- a more explicit connection to Telltale runtime adequacy families
- a less reduced bridge from Rust choreography states to Lean machine states

Until then, this document should be read as the specification of the current reduced adequacy boundary, not as a claim of whole-runtime correctness.
