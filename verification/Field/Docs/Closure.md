# Field Closure Statement

This note defines what “full field system closure” means for Jacquard. It is
the maintained ownership and parity statement for the implemented Rust/Lean
boundary.

The purpose of this file is to remove ambiguity. The field stack already has a
real Rust implementation and a substantial Lean proof stack, and this note
records the final stable parity story for the reduced proof-facing surfaces.

## Final Ownership Split

The intended end-state ownership split is:

- Rust field engine
  - owns the continuously updated destination-local field state
  - owns field-private evidence fusion
  - owns field-private search execution
  - owns route-private runtime state for field-managed routes
  - does not own canonical router truth
- Rust router
  - owns candidate comparison across engines
  - owns admission/materialization pipeline above engine-local witnesses
  - owns canonical published/materialized route truth
  - owns route lifecycle truth above engine-private state
- Rust choreography/runtime
  - owns private summary-exchange operational state
  - owns protocol stepping, bounded retention, and replay-oriented artifacts
  - may contribute observational evidence into field
  - does not own canonical route truth
- Lean local model
  - owns the reduced destination-local observer/controller semantics
  - owns the reduced regime/posture/control interpretation
  - does not own full Rust runtime mechanics
- Lean protocol
  - owns the reduced private protocol object and its proof surface
  - owns protocol/controller observational boundary inputs
  - does not silently become implementation-complete unless the proof object is
    intentionally expanded
- Lean router/system/adequacy
  - owns reduced router/system semantic objects and their theorems
  - owns reduced adequacy extraction and refinement claims
  - does not become a second truth owner for route semantics

## Rust Field Surface Inventory

The following Rust field surfaces must each stay in one of four states:

- `semantic`
  - part of the intended final semantic object
- `reduced`
  - proof-facing reduction of a richer runtime object
- `observational`
  - diagnostic or replay-facing surface that must not claim truth ownership
- `out_of_scope`
  - implementation detail not intended to become a first-class proof object

### Planner / Search Surfaces

| Rust surface | Status target | Notes |
|---|---|---|
| `FieldEngine::candidate_routes` | semantic | engine-local candidate production under the shared planner contract |
| `FieldEngine::check_candidate` | semantic | shared admission boundary |
| `FieldEngine::admit_route` | semantic | shared admission boundary with field-owned witness content |
| `FieldPlannerSearchRecord` | observational | replay/inspection surface unless promoted into adequacy extraction |
| `FieldSearchRun` | observational | contains proof-relevant pieces, but the Rust object itself is not router truth |
| `FieldReplaySnapshot` | observational packaging over mixed classes | versioned replay carrier; does not change ownership of the wrapped surfaces |
| `FieldSearchConfig` | semantic | execution-policy boundary for the field search substrate |
| `FieldSearchEpoch` / `FieldSearchSnapshotId` | semantic | stable search snapshot identity and reconfiguration boundary |
| selected private search witness | semantic | engine-private continuation choice input, not public route truth |
| selected runtime realization plus bounded continuation envelope | semantic | one public corridor claim may keep richer private runtime continuation detail |

### Runtime / Evidence Surfaces

| Rust surface | Status target | Notes |
|---|---|---|
| `FieldEngine::ingest_forward_summary` | semantic | explicit evidence ingress surface |
| `FieldEngine::record_forward_summary` | semantic | explicit evidence ingress surface |
| `FieldEngine::record_reverse_feedback` | semantic | explicit evidence ingress surface |
| destination observer state / frontier state | semantic | field-private local state actually consumed by planning/runtime |
| `FieldEngine::protocol_artifacts` | observational | bounded private diagnostics unless promoted into adequacy extraction |
| `FieldEngine::runtime_round_artifacts` | reduced | intended reduced adequacy-facing runtime projection |
| `FieldRuntimeRoundArtifact` | reduced | proof-facing runtime projection surface |
| `FieldRuntimeRouteArtifact` | reduced | reduced route-facing runtime projection |
| `FieldEngine::route_commitments` | observational | router/runtime coordination surface, not protocol-trace adequacy by default |

### Materialization / Maintenance Surfaces

| Rust surface | Status target | Notes |
|---|---|---|
| `FieldEngine::materialize_route` | semantic | field-owned route-private installation behavior |
| `FieldEngine::maintain_route` | semantic | field-owned route-private maintenance behavior |
| backend route token / witness detail | semantic | field-owned route-private continuation/witness boundary |
| active-route map internals | out_of_scope | implementation detail unless a reduced runtime-state model promotes part of it |

### Choreography Internals

| Rust surface | Status target | Notes |
|---|---|---|
| private session maps and queue internals | out_of_scope | not required as first-class proof objects unless the protocol boundary expands |
| retained `FieldProtocolArtifact` objects | observational | replay/inspection surfaces |
| per-round host wait status, disposition, blocked receive markers | reduced | already part of the reduced runtime/protocol boundary |

## Parity Inventory

These are the surfaces that must remain intentionally aligned across Rust docs,
Rust code, and Lean proof objects.

### Search Boundary

- search config and execution policy
  - Rust: `FieldSearchConfig`
  - Lean target: proof-facing field search policy object
- search query kinds
  - Rust: `single_goal` and candidate-set query resolution
  - Lean target: explicit query-family model
- snapshot identity and reconfiguration
  - Rust: `FieldSearchEpoch`, `FieldSearchSnapshotId`, `FieldSearchReconfiguration`
  - Lean target: proof-facing snapshot/reconfiguration boundary
- selected-result semantics
  - Rust: selected private witness and selected-result report surface
  - Lean target: selector/search split and replay-stability theorems
- single-candidate publication boundary
  - Rust: one selected private result, one selected runtime realization, one
    planner-visible corridor claim
  - Lean target: explicit plurality-private / publication-singular lineage

### Protocol Boundary

- reduced machine snapshot
  - Rust analogue: choreography round projection
  - Lean: `MachineSnapshot`
- protocol output and semantic object
  - Rust analogue: host-visible protocol export / replay-visible export
  - Lean: `ProtocolOutput`, `ProtocolSemanticObject`
- blocked receive / disposition / bounded stepping
  - Rust analogue: choreography round result
  - Lean: reduced protocol state and stepping laws

### Adequacy Boundary

- runtime round artifacts
  - Rust: `FieldRuntimeRoundArtifact`
  - Lean: `RuntimeRoundArtifact`
- runtime route artifact projection
  - Rust: `FieldRuntimeRouteArtifact`
  - Lean: reduced router-facing runtime projection
- runtime-state execution prefix
  - Rust analogue: bounded retained artifacts and their execution order
  - Lean: reduced runtime-state object

### Router / Route Lineage Boundary

- field candidate summary and witness
  - Rust: planner output and backend token
  - Lean target: explicit bridge from field-private search/result to router-facing route objects
- route maintenance and invalidation classes
  - Rust: maintenance outcomes and commitment invalidation causes
  - Lean target: reduced lifecycle/runtime alignment where intended

## Permanent Non-Goals

The closure target still excludes the following unless a later plan changes
them explicitly:

- proving every Rust choreography/session-map internal as a first-class Lean
  object
- making protocol artifacts or runtime round artifacts a second owner of
  canonical route truth
- treating field-private search replay artifacts as public route truth
- proving transport-specific implementation details that do not affect the
  reduced field/protocol/router boundary
- promoting every diagnostic surface into the adequacy theorem surface
- collapsing router-owned truth into field-owned truth

## Completion Rule

The field stack is “fully closed” only when:

- every semantic/reduced/observational surface above has a stable declared
  status
- the search boundary has a direct Lean model rather than prose-only treatment
- the protocol and adequacy docs no longer describe the final system as only
  partially aligned unless that reduction is deliberate and permanent
- the route lineage from field evidence to router-owned truth is explicit and
  stable

Field is therefore a closed Rust/Lean system at the intended reduced boundary.

## Final Review

The final closure answers are:

- what the Rust field system can do
  - maintain a continuously updated destination-local field model
  - run private Telltale-backed search over frozen field snapshots
  - publish one planner-visible corridor candidate per objective
  - materialize, maintain, and replay bounded field runtime surfaces
- what the Lean field system proves
  - the reduced local observer-controller semantics
  - the reduced private protocol boundary and its fixed-participant closure
  - the reduced field search boundary
  - the reduced runtime-search adequacy bridge into router/system truth
- what remains intentionally reduced or observational
  - richer choreography/session-map/runtime internals
  - replay packaging surfaces beyond the reduced extraction objects
  - canonical route truth, which remains router-owned
