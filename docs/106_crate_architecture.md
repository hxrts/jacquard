# Crate Architecture

This page describes the crate layout, the boundary rules, and the implementation policies that keep the workspace consistent.

## Boundary Rule

`core` defines what exists. `traits` defines what components are allowed to do.

`core` owns shared identifiers, data types, constants, error types, and the full model pipeline from world objects through observations, family-neutral estimates, policy, and action. It must not grow behavioral traits for subcomponents. Derives, trivial constructors, and simple validation are allowed. Cross-crate behavioral interfaces are not.

`traits` owns all cross-crate behavioral interfaces. This includes the routing contract (`PolicyEngine`, `CommitteeSelector`, `CommitteeCoordinatedEngine`, `SubstratePlanner`, `SubstrateRuntime`, `LayeredRoutingEnginePlanner`, `LayeredRoutingEngine`, `LayeringPolicyEngine`, `RoutingEnginePlanner`, `RoutingEngine`, `Router`, `RoutingControlPlane`, `RoutingDataPlane`), the runtime effect traits (`TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, `TransportEffects`), the pure hashing and content-addressing traits (`Hashing`, `ContentAddressable`, `TemplateAddressable`), the mesh-specialized traits (`MeshTopologyModel`, `MeshTransport`, `RetentionStore`, `MeshRoutingEngine`), and the simulator traits (`RoutingScenario`, `RoutingEnvironmentModel`, `RoutingSimulator`, `RoutingReplayView`). The layering subset of that routing contract is intentionally forward-looking: the shared shape is part of the stable design, but the current in-tree coverage is still contract-oriented rather than a mature production layering stack.

## Dependency Graph

The workspace today contains five crates: `jacquard-core`, `jacquard-traits`, `jacquard-macros`, `jacquard-mesh`, and `jacquard-xtask`. The graph below also shows `jacquard-router`, `jacquard-transport`, and `jacquard-simulator` as future crates that land once the router control plane, transport adapters, and simulator harness come online.

```
jacquard-core
    ↑
jacquard-traits
    ↑
jacquard-mesh ──→ jacquard-router (future) ←── jacquard-transport (future)
      │               │
      └──────→ jacquard-simulator (future)

jacquard-xtask
```

Every crate depends on `jacquard-core`. Every crate except `jacquard-core` depends on `jacquard-traits`. The future router will depend on mesh only through the `RoutingEngine` trait, not through mesh internals. The future simulator will depend on core, traits, mesh, and router, and will use `telltale-simulator` as the execution base.

`jacquard-core` and `jacquard-traits` must remain runtime-free. They may not depend on `telltale-runtime`.

`jacquard-core` compiles first with no workspace dependencies. `jacquard-traits` compiles against core. Mesh compiles against core and traits. Future transport adapters will compile against core and traits in parallel with mesh. The future router will compile against core, traits, and mesh. The future simulator will compile last against all of the above plus the Telltale simulator crates.

The future `jacquard-transport` crate is transitional. It will start as structural stubs that satisfy trait signatures without real radio integration. If transport implementations grow substantial platform logic, they should move into dedicated crates such as `jacquard-transport-ble` and `jacquard-transport-wifi`.

## Crate Layout

Inside `core`, files are grouped into three areas. `base/` holds cross-cutting primitives: identity, time, qualifiers, constants, and errors. `model/` holds the world-to-action pipeline: world objects, observations, estimation, policy, and action. `routing/` holds route lifecycle and runtime coordination objects: admission, committees, layering, runtime state, and stamped route events. Small transport and content files stay at the crate root.

This is also the main abstraction boundary for how opinionated Jacquard should be. `core` may define shared coordination result objects such as `CommitteeSelection`, identity-assurance qualifiers, and evidence classes. It must not define engine-local committee scoring policy, require a leader, or turn one routing engine's grouping heuristic into a workspace-wide law.

The same minimality rule applies to routing-engine layering. `core` may define substrate requirements, substrate leases, and layer parameters. It must not make one routing engine natively aware of another routing engine or force one composition policy on every host.

The same rule applies to authority. `core` expresses authority through the shared route contracts themselves: admitted routes, witnesses, proofs, leases, and explicit lifecycle transitions. It does not define a second parallel authority system above those route objects.

## Purity And Side Effects

Jacquard treats purity and side effects as part of the trait contract.

- `Pure` traits must be deterministic with respect to their inputs. They should not perform I/O, read ambient time, allocate order stamps, or mutate hidden state that changes outputs.
- `Read-only` traits may inspect owned state or snapshots, but they must not mutate canonical routing truth or perform runtime effects.
- `Effectful` traits may perform I/O or mutate owned runtime state, but only through an explicit boundary with a narrow purpose.

Signature design follows the same split. Use `&self` for pure and read-only methods. Use `&mut self` only when the method has explicit state mutation or side effects. Do not mix pure planning and effectful runtime mutation in one trait unless the split is impossible and documented.

That is why Jacquard separates `RoutingEnginePlanner` from `RoutingEngine`, `SubstratePlanner` from `SubstrateRuntime`, `LayeredRoutingEnginePlanner` from `LayeredRoutingEngine`, `MeshTopologyModel` from `MeshTransport`, and `RoutingScenario` / `RoutingEnvironmentModel` from `RoutingSimulator`.

This rule is enforced in three layers. Public trait definitions in `jacquard-traits` carry `#[purity(...)]` or `#[effect_trait]` annotations. The `#[purity(...)]` proc macro rejects obvious receiver-shape violations such as `&mut self` on pure traits. The repository also ships `cargo xtask` as the stable workspace check surface and three nightly Dylint libraries: `lints/trait_purity`, `lints/model_policy`, and `lints/routing_invariants`. The fast path and the compiler-backed path are intended to stay paired: `cargo xtask check routing-invariants --validate` proves the stable fixtures hit every routing rule, and the nightly lane reruns the same policy family from `nix develop ./nix/nightly` with `cargo dylint --path ...`.

Routing invariants use the same enforcement pattern. The repository-level routing-invariants lane runs through `cargo xtask check routing-invariants` and checks explicit-topology planner signatures, `Tick`/`RouteEpoch` separation, world-extension error purity, and several fail-closed mesh runtime rules. Those checks start in `jacquard-mesh`, but they are written to extend to the router, simulator adapters, and future external engines.

The routing core does not call platform APIs directly. Hashing, storage, route-event logging, transport observations, time, and ordering all cross explicit shared boundaries in `traits`. That is how native execution, tests, and simulation share one semantic model. The effect traits are narrower than the higher-level component traits. They model runtime capabilities, not whole subsystems. `RoutingEngine`, `Router`, and `RetentionStore` are larger behavioral contracts and should not be forced through the effect layer.

## Invariants And Ownership

Cross-crate invariants:

- No crate may use floating-point types in routing logic, routing state, routing policy, or simulator verdicts.
- No crate may treat wall-clock time as distributed semantic truth.
- `Tick` is time and `RouteEpoch` is configuration/versioning. Crates must not convert between them by rewrapping the inner integer.
- Canonical ordering must flow through shared ordering types. Crates must not invent crate-local tie-break schemes.
- Canonical hashing and content IDs must flow through the shared hash and content-addressing boundaries.
- Transport may observe links and carry bytes, but it may not invent route truth, publish canonical route health, or mutate materialized-route ownership.
- GPS, absolute location, clique grids, and singleton leaders are not shared routing truth. If a routing engine uses spatial hints or local coordination structures, those remain engine-private interpretations above the shared observation boundary.
- Multiple routing engines may coexist in one host runtime. Gradual migration between engines is allowed, and limited layering is allowed through the shared substrate boundary. Generic mixed-engine canonical route ownership is not a base-layer assumption.

Ownership by crate:

- `jacquard-core` owns the shared vocabulary. It does not own live state.
- `jacquard-traits` owns compile-time boundaries. It does not own runtime state.
- `jacquard-mesh` owns mesh-private forwarding state, route realization artifacts, topology caches, route repair state, route exports, engine-side route commitments, deferred-delivery retention state, and any engine-local committee scoring or misbehavior tracking.
- `jacquard-router` (future) will own canonical route identity, route materialization inputs, lease transfer, route replacement, canonical handle issuance, and top-level route-health publication.
- a host-owned policy engine above the router may own cross-engine migration policy and substrate selection policy.
- `jacquard-transport` (future) will own local transport observations and device-facing adapter state only.
- `jacquard-simulator` (future) will own replay artifacts, scenario traces, and post-run analysis outputs. It will not own canonical route truth during a live run.

## Extensibility

`core::Configuration` is the shared graph-shaped world object. If mesh needs geometry, richer topology exports, peer novelty scores, bridge estimates, flow-direction estimates, or other engine-specific structure, those should live in mesh-owned types behind the mesh trait boundary rather than being pushed into the base `Environment` or shared estimate model. The same rule applies to any routing-engine-specific state. Engine-private planning caches, forwarding tables, retention stores, and derived peer or neighborhood heuristics belong in the engine crate, not in `core`.

See [Extensibility](107_extensibility.md) for the full extension surface, dependency rules, and stable contract types.
