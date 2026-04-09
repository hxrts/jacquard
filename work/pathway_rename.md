# Pathway Rename — Second Pass

Comprehensive work plan for the second layer of "mesh" → "pathway" renaming after
the initial `work/pathway.md` migration. The first pass covered crate names, imports,
and major type identifiers. This pass covers wire-protocol strings, remaining type and
method names, inline comments, test function names, and tooling internals.

## Rename Rules

**Rename** when `mesh` means the pathway engine:
- engine domain tags, storage key prefixes, protocol checkpoint keys
- type/method names that are pathway-engine-specific
- inline comments that describe the pathway engine as "mesh"
- test function names that encode the engine as "mesh"
- xtask/lint internals that use "mesh" as a string key for the pathway crate

**Keep** when `mesh` is used properly:
- "adaptive mesh routing system" — product-level framing
- "mesh network", "mesh topology", "multi-device mesh" — generic networking
- `*b"foreign-mesh-sup"` in batman tests — a generic foreign mesh engine fixture
- `"jacquard.mesh.v1"` engine contract ID — mesh is the product/system brand here

**Rename to a more generic term** when `mesh` is used as general infrastructure:
- `send_mesh_frame` → `send_frame` (frame-send method on a pathway-private trait)
- `interpret mesh policy` → `interpret routing policy` (lib.rs doc comment)
- `importing any mesh planner` → `importing any engine planner` (doc comment)
- `used by router and mesh` → `used by router and engine` (doc comment)
- `TransportKind::Custom("mesh-{byte}")` → `"transport-{byte}"` (test fixture label)

---

## Group A — Remaining Type and Method Names

- [x] `MeshAntiEntropyState` → `PathwayAntiEntropyState`
  - `crates/pathway/src/engine/types.rs` (definition)
  - `crates/pathway/src/engine/runtime/health.rs` (usage)

- [x] `MeshServiceRequirements` → `PathwayServiceRequirements`
  - `crates/pathway/src/topology.rs` (type alias + all usages in file)

- [x] `send_mesh_frame` → `send_frame`
  - `crates/pathway/src/choreography/effects.rs` (trait definition + impl)
  - `crates/pathway/src/choreography/forwarding.rs` (call site + FakeEffects impl)
  - `crates/pathway/src/choreography/hold_replay.rs` (call site)
  - `crates/pathway/src/choreography/runtime.rs` (FakeEffects impl)

---

## Group B — Wire-Protocol String Literals

- [x] Domain tags in `crates/pathway/src/engine/support.rs`:
  - `b"mesh-route-id"` → `b"pathway-route-id"`
  - `b"mesh-commitment"` → `b"pathway-commitment"`
  - `b"mesh-handoff-receipt"` → `b"pathway-handoff-receipt"`
  - `b"mesh-retention"` → `b"pathway-retention"`
  - `b"mesh-committee-id"` → `b"pathway-committee-id"`
  - `b"mesh-order-key"` → `b"pathway-order-key"`

- [x] Storage key prefixes in `crates/pathway/src/engine/support.rs`:
  - `b"mesh/"` → `b"pathway/"` (in `route_storage_key` and `topology_epoch_storage_key`)

- [x] Protocol checkpoint key prefix in `crates/pathway/src/choreography/runtime.rs`:
  - `"mesh/protocol/..."` → `"pathway/protocol/..."`

- [x] Test fixture checkpoint key in `crates/pathway/src/choreography/effects.rs`:
  - `b"mesh/choreo/activation"` → `b"pathway/choreo/activation"`

- [x] Comment in `crates/pathway/src/engine/mod.rs`:
  - Domain tag names in comment updated from `"mesh-route-id"` etc. to `"pathway-route-id"` etc.

---

## Group C — Test Assertions on Wire Keys

- [x] `crates/pathway/tests/choreography_runtime.rs` — all `"mesh/protocol/..."` → `"pathway/protocol/..."`
- [x] `crates/pathway/tests/checkpoint.rs` — all `"mesh/protocol/..."` → `"pathway/protocol/..."`
- [x] `crates/pathway/tests/engine_tick.rs` — `b"mesh/"` → `b"pathway/"` in storage key construction
- [x] `crates/pathway/tests/materialization.rs` — all `"mesh/..."` → `"pathway/..."` in key assertions
- [x] `crates/pathway/src/engine/support.rs` — inline test hashing using `b"mesh-retention"` → `b"pathway-retention"`
- [x] `crates/pathway/src/engine/support.rs` — updated snapshot hash value in `canonical_bytes_snapshot_values` (changed because domain tag changed)
- [x] `crates/xtask/fixtures/routing_invariants/crates/pathway/src/engine/runtime.rs` — fixture updated to `b"pathway/topology-epoch"`
- [x] `crates/xtask/fixtures/routing_invariants/crates/pathway/src/engine/runtime/mod.rs` — same

---

## Group D — Test Function Names

- [x] `crates/pathway/tests/capabilities.rs` — `mesh_capability_surface_matches_the_advertised_constant` → `pathway_capability_surface_matches_the_advertised_constant`
- [x] `crates/pathway/tests/contracts.rs` — `mesh_routing_engine_exposes_explicit_mesh_owned_subcomponents` → `pathway_routing_engine_exposes_explicit_pathway_owned_subcomponents`
- [x] `crates/pathway/tests/determinism.rs` — `mesh_engine_accepts_non_blake3_hashing_for_route_identity` → `pathway_engine_accepts_non_blake3_hashing_for_route_identity`
- [x] `crates/pathway/tests/maintenance.rs` — `anti_entropy_required_is_a_progress_refresh_in_v1_mesh` → `...in_v1_pathway`
- [x] `crates/pathway/tests/maintenance.rs` — `link_degraded_consumes_one_repair_budget_step_in_v1_mesh` → `...in_v1_pathway`
- [x] `crates/pathway/tests/commitments.rs` — `v1_mesh_exposes_one_commitment_per_route_across_runtime_postures` → `v1_pathway_exposes...`
- [x] `crates/pathway/tests/common/engine.rs` — `mesh_connectivity` → `pathway_connectivity` (definition + all call sites)
- [x] `crates/pathway/src/engine/support.rs` — `mesh_domain_tags_are_unique` → `pathway_domain_tags_are_unique`
- [x] `crates/pathway/src/choreography/effects.rs` — `fake_mesh_choreo_adapter_maps_runtime_actions` → `fake_pathway_choreo_adapter_maps_runtime_actions`
- [x] `crates/router/tests/router_registry.rs` — `multi_engine_router_rejects_duplicate_mesh_registration` → `...duplicate_pathway_registration`
- [x] `crates/router/tests/router_registry.rs` — `multi_engine_router_registers_multiple_engines_and_selects_mesh_candidate` → `...selects_pathway_candidate`
- [x] `crates/router/tests/router_fail_closed.rs` — `recovery_restores_router_and_mesh_state_from_router_owned_registry` → `...pathway_state...`

---

## Group E — Inline Comments and File-Header Docs

- [x] `crates/pathway/src/engine/mod.rs` — domain tag names in comment
- (All other file-header doc comments were already updated in the first pass)

---

## Group F — Cross-Crate Comments and Error Strings

- [x] `crates/router/tests/router_registry.rs` — `.expect_err("duplicate mesh engine should be rejected")` → `"...pathway engine..."`
- [x] `crates/router/tests/router_middleware.rs` — `.expect("mesh capabilities")` → `"pathway capabilities"`
- [x] `crates/router/tests/common/router_builder.rs` — `.expect("register committee mesh engine")` → `"...pathway engine"`
- [x] `crates/router/tests/common/router_builder.rs` — `.expect("register mesh engine")` → `"register pathway engine"`
- [x] `crates/traits/src/routing.rs` — doc comment `(eg. mesh)` → `(eg. pathway)`
- [x] `crates/mem-link-profile/src/effect.rs` — `"used by router and mesh"` → `"used by router and engine"`
- [x] `crates/mem-link-profile/src/lib.rs` — `"interpret mesh policy"` → `"interpret routing policy"`
- [x] `crates/mem-node-profile/src/lib.rs` — `"importing any mesh planner"` → `"importing any engine planner"`

---

## Group G — Xtask and Lint Check Internals

- [x] `crates/xtask/src/checks/checkpoint_namespacing.rs` — crate-type string `"mesh"` → `"pathway"`; all `engine/mesh/` → `engine/pathway/`; all `mesh/` → `pathway/`
- [x] `lints/model_policy/src/shared_boundary.rs` — added `starts_with("Pathway")` alongside existing checks
- [x] `crates/xtask/src/checks/routing_invariants.rs` — regex `b"mesh/(topology-epoch|route/)"` → `b"pathway/(topology-epoch|route/)"`
- [x] `lints/routing_invariants/src/lint.rs` — same regex update
- [x] `crates/xtask/src/checks/engine_service_boundary.rs` — variable `mesh_lib` → `pathway_lib`; error message `"no mesh/src/lib.rs found"` → `"no pathway/src/lib.rs found"`
- [x] `crates/xtask/src/checks/pathway_choreography.rs` — stale `"mesh-choreography"` error message → `"pathway-choreography"`
- [x] `crates/xtask/fixtures/mesh_choreography/` — removed orphaned directory

---

## Group H — Test Data and Enum Variants

- [x] `crates/pathway/tests/lifecycle.rs` — `b"mesh-payload"` → `b"test-payload"` (generic test data)
- [x] `crates/pathway/src/engine/support.rs` — `TransportKind::Custom(format!("mesh-{byte}"))` → `"transport-{byte}"` (generic test label)
- [x] `crates/macros/tests/integration/annotation_contract.rs` — `RouteMode::Mesh` → `RouteMode::Pathway`

---

## Intentionally Kept

- `"jacquard.mesh.v1"` — 16-byte wire protocol ID; "mesh" refers to Jacquard as a mesh routing system (product brand)
- `*b"foreign-mesh-sup"` in `crates/batman/src/planner.rs` — a fixture for a generic foreign mesh-routing engine, not the pathway engine

---

## Verification Grep Searches

Run these after implementation to confirm completion:

```bash
rg -n "\bmesh\b" crates/pathway/src/ --type rust
rg -n "\bmesh\b" crates/pathway/tests/ --type rust
rg -n '"mesh' crates/pathway/ --type rust
rg -n "b\"mesh" crates/pathway/ --type rust
rg -n "\bmesh\b" crates/router/tests/ --type rust
rg -n "\bmesh\b" crates/traits/src/ --type rust
rg -n "\bmesh\b" crates/mem-link-profile/src/ --type rust
rg -n "\bmesh\b" crates/mem-node-profile/src/ --type rust
rg -n "\bmesh\b" crates/batman/src/ --type rust
rg -n "mesh" crates/xtask/src/checks/ --type rust
rg -n "mesh" lints/model_policy/src/ --type rust
rg -n "mesh" lints/routing_invariants/src/ --type rust
rg -n "\bmesh\b" crates/macros/tests/ --type rust
```

---

## Verification Steps

- [x] `cargo check --workspace` — compiles clean
- [x] `cargo test --workspace` — all tests pass
- [x] `just ci-dry-run` — all 36 checks green
- [x] Grep audit confirms zero unintended renames
