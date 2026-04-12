# Field Adequacy

## Purpose

The adequacy layer is the first formal bridge between the Rust private field runtime and the reduced Lean protocol object. Its job is narrow and explicit: relate a small Rust-facing artifact shape to a reduced Lean machine snapshot and a reduced Lean protocol trace, then show that the host-visible controller evidence extracted from those artifacts agrees with the controller evidence extracted from the Lean trace.

That ownership is intentionally separate from `Field/Model/Boundary.lean`:
the model boundary owns protocol/controller extraction from protocol exports and
semantic traces, while adequacy owns runtime-artifact/runtime-state reduction
before those controller-boundary theorems are applied.

This layer does not prove full Rust runtime correctness. It does not prove scheduler correctness, checkpoint correctness, transport correctness, or router correctness. It proves a reduced artifact-to-trace story that is honest about what information is preserved and what information is erased.

It now also proves a small runtime/system safety story on top of the stuttering refinement layer, and it includes proof-facing fixture cases so the canonical theorems are pinned to concrete reduced runtime examples rather than only prose descriptions.

## Refinement Ladder

The adequacy layer now sits inside one explicit refinement ladder:

- local/private semantics
- public/system semantics
- router-owned truth
- runtime artifacts and reduced runtime states

This does not make adequacy the owner of the lower layers. The adequacy files
only bridge into them.

In particular:

- `Field/Protocol/*` still owns reduced protocol semantics
- `Field/System/*` still owns reduced end-to-end system semantics
- `Field/Router/*` still owns canonical route truth
- `Field/Adequacy/*` only packages reduced runtime projection, refinement, and
  preservation theorems over those semantic layers

## Semantic Objects Versus Proof Artifacts

The adequacy stack now keeps a sharper split between semantic objects and
proof-facing artifacts.

Semantic reduced execution objects:

- `RuntimeRoundArtifact`
- `RuntimeState`
- `RuntimeStep`
- runtime/system refinement relations and projected runtime views

Proof-packaging or fixture objects:

- `RuntimeTraceSimulation`
- theorem-pack wrappers in `Field/Adequacy/Refinement.lean`
- concrete fixture families in `Field/Adequacy/Fixtures.lean`
- concrete probabilistic boundary cases in
  `Field/Adequacy/ProbabilisticFixtures.lean`

The fixture files are intentionally proof-facing synthetic witnesses. They are
not additional runtime semantics.

## Runtime Artifact Boundary

The adequacy boundary is defined in `Field/Adequacy/API.lean`.

The Rust-facing artifact is:

```text
RuntimeRoundArtifact :=
  blockedReceive : Option SummaryLabel
  disposition : HostDisposition
  emittedCount : Nat
  stepBudgetRemaining : Nat
  routerArtifact : Option RuntimeRouterArtifact
```

This is intentionally much smaller than the real Rust choreography runtime. It mirrors only the controller-relevant fields of the private protocol round result:

- blocked receive frontier
- host disposition
- emitted summary count
- remaining step budget

The new `routerArtifact` field is still reduced. It carries at most one reduced lifecycle-route projection for the round artifact. It does not make the adequacy layer the owner of canonical route truth.

The concrete Rust analogue is `jacquard_field::FieldRuntimeRoundArtifact`,
recorded through `FieldEngine::runtime_round_artifacts()`. The actual Rust
artifact currently carries more fields than the reduced Lean object, including
protocol kind, optional destination, host-wait status, execution-policy class,
and observation tick. The adequacy boundary intentionally erases those fields
for now.

It intentionally erases:

- session maps
- retention internals
- outbound queue internals
- checkpoint payloads
- transport-local state
- protocol kind, destination, host-wait status, execution-policy class, and
  observation tick from the Rust round artifact

## Reduced Runtime State Layer

The adequacy layer now also has a proof-facing runtime-state module:

```text
Field/Adequacy/Runtime.lean
```

It introduces:

```text
RuntimeState
initialRuntimeState
RuntimeStep
runtimeArtifactsOfState
runtimeArtifactOfStep
RuntimeStateAdmitted
```

This is still intentionally reduced. The state records only:

- pending runtime artifacts
- completed runtime artifacts

and the step relation consumes exactly one pending artifact and appends it to the completed prefix.

This is not yet a faithful host/runtime operational semantics. It is the first proof-facing execution object above flat artifact lists, and it exists so later phases can state runtime-to-system refinement as an execution theorem rather than only as an artifact-alignment theorem.

On top of that state layer, `Field/Adequacy/Safety.lean` now packages the first runtime/system safety consequences:

- support conservativity for quiescent runtime-state winners
- no false explicit-path promotion for quiescent runtime-state winners
- no route creation from destination-local silence
- admissible ready-installed lifecycle origin for canonical winners
- observational equivalence on quiescent runtime states projecting the same system state

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
- any reduced router-facing runtime projection must stay lifecycle-honest

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

The runtime-state layer reuses that same extraction over completed runtime prefixes via:

```text
extractTraceOfState
runtimeEvidenceOfState
admitted_runtime_state_simulates_reduced_protocol
admitted_runtime_state_extracts_to_observational_trace
runtime_step_preserves_state_admitted
```

So the current adequacy bridge can be read in two compatible ways:

- as an artifact-list bridge
- as a reduced runtime-state execution-prefix bridge

That split is deliberate:

- flat artifact lists are the reduced runtime projection surface
- reduced runtime states are the semantic execution object used by the next
  refinement layer
- projected fixtures are only proof aids for pinning theorem surfaces to
  concrete examples

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

### Runtime-To-Canonical Refinement

The adequacy layer now also contains a dedicated runtime-to-canonical theorem file:

```text
Field/Adequacy/Canonical.lean
```

It defines an explicit reduced alignment predicate:

```text
RuntimeSystemCanonicalAligned
```

and proves:

```text
runtime_canonical_route_eq_canonicalSystemRoute_of_alignment
runtime_canonical_route_view_eq_bestSystemRouteView_supportDominance_of_alignment
```

This is the first explicit bridge from Rust-facing runtime artifacts to router-owned canonical truth. The bridge is still narrow: it depends on an explicit reduced lifecycle-alignment hypothesis rather than proving full runtime correctness.

### Projected Runtime-System Refinement

The adequacy layer now also contains a stronger projected-runtime theorem file:

```text
Field/Adequacy/Projection.lean
```

It defines:

```text
runtimeArtifactOfLifecycleRoute
projectedRuntimeArtifactsOfState
RuntimeExecutionProjectsSystemState
```

and now also proves simple metric-preservation facts such as projected artifact-count preservation across the runtime/system projection.

### Runtime-State Safety Preservation

The stronger runtime-state layer also proves:

```text
runtime_step_preserves_protocol_and_router_invariants
quiescent_runtime_state_support_conservative
quiescent_runtime_state_no_false_explicit_path_promotion
quiescent_runtime_state_no_route_creation_from_system_silence
quiescent_runtime_state_canonical_winner_has_admissible_system_origin
quiescent_runtime_states_projecting_same_system_have_equal_canonical_route
runtime_projection_observational_equivalence_preserves_canonical_route
```

These are still reduced theorems. They are not full Rust implementation theorems. But they move the current story beyond bare artifact alignment by showing that the runtime/system refinement relation preserves the first safety claims operators actually care about.

### Proof-Facing Fixtures

The adequacy layer now also contains:

```text
Field/Adequacy/Fixtures.lean
```

This file gives reduced runtime cases that exercise:

- support-dominance canonical selection
- the stronger support-then-hop router selector
- empty-runtime silence
- one known non-claim from the quality layer

The intended parity workflow is:

1. define or update a reduced runtime/system scenario
2. pin the expected canonical outcome with a small theorem in `Fixtures.lean`
3. keep at least one non-claim or boundary scenario alongside positive cases

This is still a reduced parity workflow, not a direct Rust extraction pipeline.

and proves:

```text
projectedRuntimeArtifactsOfState_admitted
runtimeExecutionProjectsSystemState_implies_alignment
projected_runtime_canonical_route_eq_canonicalSystemRoute
projected_runtime_canonical_route_view_eq_bestSystemRouteView_supportDominance
```

This removes the free alignment hypothesis from the stronger top-level story. The current theorem-driven path is:

- reduced system state
- projected reduced runtime artifacts
- reduced runtime canonical selector
- router-owned canonical truth

That is stronger than the earlier alignment-only bridge, but it is still not a full Rust/runtime correctness theorem. The projected artifacts are generated from the reduced Lean `systemStep`, not extracted from arbitrary production Rust executions.

### Runtime-State To System Refinement

The adequacy layer now also contains a runtime-state refinement file:

```text
Field/Adequacy/Refinement.lean
```

It defines:

```text
projectedRuntimeStateOfSystem
RuntimeStateProjectsSystemState
RuntimeStateQuiescent
```

The key idea is a stuttering refinement relation:

- `runtimeArtifactsOfState runtimeState`
  is the completed runtime prefix
- `runtimeState.pendingArtifacts`
  is the remaining runtime suffix
- their concatenation must match `projectedRuntimeArtifactsOfState state`

So the runtime-state story is no longer phrased only as:

- one synthetic runtime artifact list
- one canonical theorem over that list

It is now phrased as:

- a reduced runtime state
- a reduced runtime step relation
- a runtime/system refinement relation
- quiescent runtime-state consequences for canonical truth

The main theorems are:

```text
runtime_step_preserves_runtime_system_refinement
runtime_step_preserves_runtime_system_refinement_admitted
quiescent_runtime_state_canonical_route_eq_canonicalSystemRoute
contract_yields_runtime_execution_canonical_refinement
quiescent_runtime_state_route_view_eq_bestSystemRouteView_supportDominance
quiescent_runtime_state_support_conservative
quiescent_runtime_state_explicit_path_requires_explicit_sender_knowledge
```

This is still a reduced theorem story. The relation is defined against the projected-runtime view induced by the current Lean `systemStep`, so it is not yet a full extracted-Rust forward simulation. But it is now an execution-state refinement layer, not only an artifact-list bridge.

## Assumptions Packaging

After the cleanup refactor, the assumptions layer is split into:

- `Field/AssumptionCore.lean`
- `Field/AssumptionTheorems.lean`
- `Field/Assumptions.lean` as the thin umbrella import

The shared vocabulary still packages the growing assumption boundary into:

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

The current packaged contract also exports:

```text
contract_yields_runtime_evidence_agreement
contract_yields_observational_controller_boundary
contract_yields_protocol_trace_admitted
contract_yields_reduced_quality_stability
contract_yields_reduced_quality_support_conservativity
contract_yields_explicit_path_quality_observer
contract_yields_support_optimality_refinement
contract_yields_canonical_router_refinement
contract_yields_runtime_canonical_refinement
contract_yields_runtime_system_canonical_refinement
contract_yields_runtime_state_system_canonical_refinement
```

So the assumptions layer is no longer only a container for future assumptions. It already exposes a small usable bridge from the default contract to the current adequacy and controller-boundary results, while keeping contract vocabulary and theorem packaging in separate files.
It also now distinguishes:

- reduced quality-comparison readiness
- support-only optimality-refinement readiness
- canonical-router refinement readiness
- runtime-canonical refinement readiness
- runtime-system refinement readiness
- still-false global optimality readiness

## What The Adequacy Layer Does Not Prove

The current adequacy layer still does not prove:

- full Rust choreography adherence to the reduced Lean machine on every execution
- fairness or scheduler properties
- checkpoint or recovery correctness
- replay exactness for the full Rust runtime
- extracted-Rust correspondence with router-owned canonical route truth without going through the reduced projected runtime execution
- transport correctness

So the correct reading is:

- the adequacy layer proves a reduced artifact-to-trace simulation story
- it does not yet prove full implementation refinement

## Relationship To End-To-End Results

The newer end-to-end and convergence files in `Field/System/*` sit above this adequacy boundary. They compose reduced local state, async transport, and router lifecycle objects and then prove safety/stability facts about that reduced system.

Those results do not upgrade the adequacy claim. This module family still does not prove:

- full Rust transport refinement
- full Rust router/runtime refinement
- routing-quality or optimality properties

The stronger support-only, canonical-router, runtime-canonical, and runtime-system refinement contracts change only the theorem-pack boundary above `Field/System`, `Field/Router`, `Field/Quality`, and `Field/Adequacy`. They do not strengthen the Rust adequacy statement into full implementation-optimality or full implementation-correctness theorems.

So adequacy remains an artifact-to-trace bridge, not an end-to-end implementation-correctness theorem.

## Parity-Sensitive Surfaces

The most important Rust/Lean compatibility surfaces are:

| Lean surface | Rust-side analogue | Compatibility expectation |
|---|---|---|
| `FieldModelAPI.EvidenceInput` | field observer and summary shaping | semantic drift must be reviewed explicitly |
| `FieldProtocolAPI.MachineSnapshot` | private choreography round state projection | additions must preserve the observational boundary |
| `FieldProtocolAPI.ProtocolOutput` | host-visible private summary result | must remain observational-only |
| `FieldProtocolAPI.ProtocolSemanticObject` | replay-visible private protocol export | must not gain stronger authority |
| `FieldAdequacyAPI.RuntimeRoundArtifact` | reduced projection of `FieldChoreographyRoundResult`-like data | must stay aligned with the actual extraction used in Rust |

Concretely, the Rust surfaces that currently need this review discipline are:

- `FieldEngine::runtime_round_artifacts()`
- `FieldEngine::protocol_artifacts()`
- `FieldRuntimeRoundArtifact`
- `FieldRuntimeRouteArtifact`

Route commitments and planner search records are intentionally outside this
adequacy object today. They are runtime diagnostics and router-facing runtime
surfaces, not part of the current artifact-to-trace reduction theorem.

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
