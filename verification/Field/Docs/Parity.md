# Field Parity Ledger

This file is the maintained Rust/Lean/docs compatibility ledger for the field
stack. It exists so future changes move the right surfaces together instead of
quietly drifting.

## Search Boundary

| Surface | Rust owner | Lean/docs owner | Status |
|---|---|---|---|
| search config / execution policy | `FieldSearchConfig` | `Field/Search/API.lean`, `Field/Adequacy/Search.lean`, `Docs/Guide.md` | active parity surface |
| query family split | `SearchQuery::single_goal`, candidate-set resolution in `search/runner.rs` | `Field/Search/API.lean`, `Docs/Guide.md` | active parity surface |
| snapshot identity / reconfiguration | `FieldSearchEpoch`, `FieldSearchSnapshotId`, `FieldSearchReconfiguration` | `Field/Search/API.lean`, `Field/Adequacy/Search.lean`, `Docs/Closure.md` | active parity surface |
| selected private result | `FieldSelectedContinuation`, `FieldPlannerSearchRecord` | `Docs/Closure.md` | active parity surface |
| planner-visible publication boundary | `planner.rs` one-candidate corridor projection | router docs | active parity surface |

## Runtime Boundary

| Surface | Rust owner | Lean/docs owner | Status |
|---|---|---|---|
| evidence ingress | `ingest_forward_summary`, `record_forward_summary`, `record_reverse_feedback` | `Docs/Closure.md` | active parity surface |
| observer refresh / cache gating | `runtime.rs`, `observer.rs` | `Docs/Adequacy.md` | active parity surface |
| continuation-envelope route state | `route.rs`, `runtime.rs` | `Docs/Closure.md` | active parity surface |
| route maintenance / commitment invalidation | `runtime.rs` | `Docs/Adequacy.md` | active parity surface |
| runtime/search linkage metadata | `FieldRuntimeRoundArtifact` | `Field/Adequacy/API.lean`, `Docs/Adequacy.md` | active parity surface |

## Replay / Inspection Boundary

| Surface | Rust owner | Lean/docs owner | Status |
|---|---|---|---|
| versioned replay snapshot | `FieldReplaySnapshot` | `Docs/Adequacy.md` | active parity surface |
| search replay view | `FieldSearchReplaySurface` | `Docs/Closure.md` | observational |
| protocol replay packaging | `FieldProtocolReplaySurface` | `Docs/Protocol.md`, `Docs/Adequacy.md` | observational |
| reduced protocol replay extraction | `FieldReplaySnapshot::reduced_protocol_replay()` | `Field/Protocol/Reconfiguration.lean`, `Field/Adequacy/Search.lean`, `Docs/Protocol.md`, `Docs/Adequacy.md` | reduced |
| runtime replay view | `FieldRuntimeReplaySurface` | `Docs/Adequacy.md` | reduced |
| exported replay bundle | `FieldExportedReplayBundle` | `Docs/Adequacy.md` | reduced tooling surface |
| replay-derived fixture vocabulary | `FieldExportedReplayBundle::lean_replay_fixture`, fixture JSON under `crates/field/tests/fixtures/replay/` | `Field/Adequacy/ReplayFixtures.lean`, `Docs/Adequacy.md`, `Docs/Protocol.md` | active parity surface |
| commitment replay view | `FieldCommitmentReplaySurface` | `Docs/Adequacy.md` | observational |

## Ownership Discipline

These statements must remain true unless a later proof/documented design change
explicitly replaces them:

- field is a single private-selector engine
- one routing objective yields one planner-visible corridor candidate
- field-internal plurality stays private
- protocol artifacts and protocol reconfiguration are observational-only
- runtime round artifacts are reduced
- exported replay and replay-derived fixtures are reduced and non-authoritative
- participant-set change stays outside the supported field reconfiguration boundary
- canonical route truth remains router-owned

## Drift Checks

The repo-local drift checks currently enforce:

- no stale field route vocabulary such as `primary_neighbor`, `alternates`, or
  `MAX_ALTERNATE_COUNT` in the field implementation/docs surfaces
- crate docs point only at live field proof/doc pages
- field docs keep the reduced proof boundary explicit instead of overclaiming a
  richer runtime/search proof surface

If a future change intentionally invalidates one of those checks, update this
ledger and the associated tests in the same patch.
