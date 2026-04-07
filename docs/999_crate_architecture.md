# Crate Architecture

This page describes the crate layout, the boundary rules, and the implementation policies that keep the workspace consistent.

## Boundary Rule

`core` defines what exists. `traits` defines what components are allowed to do.

`core` owns shared identifiers, data types, constants, error types, and the full model pipeline from world objects through observations, family-neutral estimates, policy, and action. Derives, trivial constructors, and simple validation are allowed. Cross-crate behavioral interfaces belong in `traits`.

`traits` owns the cross-crate behavioral interfaces, grouped below by purpose. The layering subset is forward-looking. The shared shape is part of the stable design, but in-tree coverage is still contract-oriented rather than a mature production layering stack.

| Category | Traits |
|---|---|
| Routing contract | `RoutingEnginePlanner`, `RoutingEngine`, `Router`, `RoutingControlPlane`, `RoutingDataPlane`, `PolicyEngine` |
| Local coordination | `CommitteeSelector`, `CommitteeCoordinatedEngine` |
| Layering | `SubstratePlanner`, `SubstrateRuntime`, `LayeredRoutingEnginePlanner`, `LayeredRoutingEngine`, `LayeringPolicyEngine` |
| Runtime effects | `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, `TransportEffects` |
| Hashing and content | `Hashing`, `ContentAddressable`, `TemplateAddressable` |
| Mesh specialization | `MeshTopologyModel`, `MeshTransport`, `RetentionStore`, `MeshRoutingEngine` |
| Simulator | `RoutingScenario`, `RoutingEnvironmentModel`, `RoutingSimulator`, `RoutingReplayView` |

## Dependency Graph

The workspace today contains five crates: `jacquard-core`, `jacquard-traits`, `jacquard-macros`, `jacquard-mesh`, and `jacquard-xtask`. Three more are planned: `jacquard-router`, `jacquard-transport`, and `jacquard-simulator`.

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

Every crate depends on `jacquard-core`. Every crate except `jacquard-core` depends on `jacquard-traits`. The future router will depend on mesh only through the `RoutingEngine` trait, not through mesh internals. `jacquard-core` and `jacquard-traits` must remain runtime-free and must not depend on `telltale-runtime`.

## Crate Layout

Inside `core`, files are grouped into three areas. `base/` holds cross-cutting primitives: identity, time, qualifiers, constants, and errors. `model/` holds the world-to-action pipeline: world objects, observations, estimation, policy, and action. `routing/` holds route lifecycle and runtime coordination objects.

`core` defines result shapes, not policies. It exposes coordination objects like `CommitteeSelection`, layering objects like `SubstrateLease`, and route lifecycle objects like `RouteHandle`, but it does not encode engine-local scoring, committee algorithms, leader requirements, layering decisions, or a parallel authority system above those route objects. Authority flows through the route contracts themselves: admitted routes, witnesses, proofs, leases, and explicit lifecycle transitions.

## Purity And Side Effects

Jacquard treats purity and side effects as part of the trait contract.

- `Pure` traits must be deterministic with respect to their inputs. They should not perform I/O, read ambient time, allocate order stamps, or mutate hidden state that changes outputs.
- `Read-only` traits may inspect owned state or snapshots, but they must not mutate canonical routing truth or perform runtime effects.
- `Effectful` traits may perform I/O or mutate owned runtime state, but only through an explicit boundary with a narrow purpose.

Signature design follows the same split. Use `&self` for pure and read-only methods. Use `&mut self` only when the method has explicit state mutation or side effects. Do not mix pure planning and effectful runtime mutation in one trait unless the split is impossible and documented.

That is why Jacquard separates `RoutingEnginePlanner` from `RoutingEngine`, `SubstratePlanner` from `SubstrateRuntime`, `LayeredRoutingEnginePlanner` from `LayeredRoutingEngine`, and `MeshTopologyModel` from the frame-shaped `MeshTransport` carrier boundary. The shared tick lifecycle follows the same rule: router-owned cadence and `RoutingTickContext` / `RoutingTickOutcome` live at the contract layer, while engine-specific control loops and control-state contents stay inside the owning engine crate.

## Enforcement

Trait purity and routing invariants are enforced by the lint suite. `cargo xtask` provides the stable-toolchain check lane, and the nightly Dylint libraries under `lints/trait_purity`, `lints/model_policy`, and `lints/routing_invariants` provide compiler-backed coverage. The live rule set lives in `crates/xtask/src/checks/` and the three lint crates. Public trait definitions in `jacquard-traits` also carry `#[purity(...)]` or `#[effect_trait]` annotations that the `#[purity(...)]` proc macro validates at compile time.

## Runtime Boundary

The routing core does not call platform APIs directly. Hashing, storage, route-event logging, transport observations, time, and ordering all cross explicit shared boundaries in `traits`. That is how native execution, tests, and simulation share one semantic model.

The effect traits are narrower than the higher-level component traits. They model runtime capabilities, not whole subsystems. `RoutingEngine`, `Router`, and `RetentionStore` are larger behavioral contracts and should not be forced through the effect layer.

## Invariants

- No crate may use floating-point types in routing logic, routing state, routing policy, or simulator verdicts.
- No crate may treat wall-clock time as distributed semantic truth.
- `Tick` is time and `RouteEpoch` is configuration versioning. Crates must not convert between them by rewrapping the inner integer.
- Canonical ordering must flow through shared ordering types. Crates must not invent crate-local tie-break schemes.
- Canonical hashing and content IDs must flow through the shared hash and content-addressing boundaries.
- Transport may observe links and carry bytes, but it must not invent route truth, publish canonical route health, or mutate materialized-route ownership.
- GPS, absolute location, clique grids, and singleton leaders are not shared routing truth. Spatial hints stay engine-private above the shared observation boundary.
- Multiple routing engines may coexist in one host runtime. Generic mixed-engine canonical route ownership is not a base-layer assumption.

## Ownership

Each crate owns a narrow slice of runtime state.

| Crate | Owns |
|---|---|
| `jacquard-core` | Shared vocabulary. No live state. |
| `jacquard-traits` | Compile-time boundaries. No runtime state. |
| `jacquard-mesh` | Mesh-private forwarding state, topology caches, repair state, retention state, and engine-local committee scoring. |
| `jacquard-router` (future) | Canonical route identity, materialization inputs, leases, handle issuance, top-level route-health publication. |
| `jacquard-transport` (future) | Local transport observations and device-facing adapter state. |
| `jacquard-simulator` (future) | Replay artifacts, scenario traces, post-run analysis. No canonical route truth during a live run. |

A host-owned policy engine above the router may own cross-engine migration policy and substrate selection.

## Extensibility

`core::Configuration` is the shared graph-shaped world object. Engine-specific structure such as topology exports, peer novelty, bridge estimates, planning caches, and forwarding tables belongs in the engine crate behind its trait boundary rather than in `core`.

The extension surface is split across [World Extensions](107_world_extensions.md), [Routing Engines](108_routing_engines.md), [Runtime Effects](104_runtime_effects.md), and [Mesh Routing](109_mesh_routing.md).
