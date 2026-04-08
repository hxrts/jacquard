# Crate Architecture

This page describes the crate layout, the boundary rules, and the implementation policies that keep the workspace consistent.

## Boundary Rule

`core` defines what exists. `traits` defines what components are allowed to do.

`core` owns shared identifiers, data types, constants, error types, and the full model pipeline from world objects through observations, engine-neutral estimates, policy, and action. Derives, trivial constructors, and simple validation are allowed. Cross-crate behavioral interfaces belong in `traits`.

`traits` owns the cross-crate behavioral interfaces, grouped below by purpose. The layering subset is forward-looking. The shared shape is part of the stable design, but in-tree coverage is still contract-oriented rather than a mature production layering stack.

Shared transport vocabulary follows the same rule. `core` keeps a small,
observed-world transport schema in `TransportProtocol`, `EndpointAddress`, and
`LinkEndpoint` because those types appear in shared `Link`,
`ServiceDescriptor`, and `TransportObservation` facts. Jacquard intentionally
does not force those types fully opaque today; the opaque endpoint variant
remains available, and a broader opacity refactor should only happen if a
second transport proves the current shared model too specific.

| Category | Traits |
|---|---|
| Routing contract | `RoutingEnginePlanner`, `RoutingEngine`, `Router`, `RoutingControlPlane`, `RoutingDataPlane`, `PolicyEngine` |
| Local coordination | `CommitteeSelector`, `CommitteeCoordinatedEngine` |
| Layering | `SubstratePlanner`, `SubstrateRuntime`, `LayeredRoutingEnginePlanner`, `LayeredRoutingEngine`, `LayeringPolicyEngine` |
| Runtime effects | `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, `TransportEffects` |
| Hashing and content | `Hashing`, `ContentAddressable`, `TemplateAddressable` |
| Simulator | `RoutingScenario`, `RoutingEnvironmentModel`, `RoutingSimulator`, `RoutingReplayView` |

## Dependency Graph

The workspace today contains nine crates: `jacquard-core`, `jacquard-traits`, `jacquard-macros`, `jacquard-mesh`, `jacquard-router`, `jacquard-mem-node-profile`, `jacquard-mem-link-profile`, `jacquard-reference-client`, and `jacquard-xtask`. `jacquard-simulator` remains a planned crate above the shared router and engine boundaries.

```
jacquard-core
    ↑
jacquard-traits
    ↑
jacquard-mem-node-profile
      │
jacquard-mem-link-profile
      │
jacquard-mesh ──→ jacquard-router ←── jacquard-reference-client
      │               │                ↑
      └──────→ jacquard-simulator      └── composes mem-* + router + mesh

jacquard-xtask
```

Every crate depends on `jacquard-core`. Every crate except `jacquard-core` depends on `jacquard-traits` only when they need behavioral boundaries. `jacquard-router` depends on registered engines only through shared traits, not through mesh internals. `jacquard-mem-node-profile` depends only on `jacquard-core` plus serialization support. `jacquard-mem-link-profile` depends on `jacquard-core` and `jacquard-traits` because it implements shared transport, retention, and effect traits. `jacquard-core` and `jacquard-traits` remain runtime-free.

## Crate Layout

Inside `core`, files are grouped into three areas. `base/` holds cross-cutting primitives: identity, time, qualifiers, constants, and errors. `model/` holds the world-to-action pipeline: world objects, observations, estimation, policy, and action. `routing/` holds route lifecycle and runtime coordination objects.

`core` defines result shapes, not policies. It exposes coordination objects like `CommitteeSelection`, layering objects like `SubstrateLease`, and route lifecycle objects like `RouteHandle`, but it does not encode engine-local scoring, committee algorithms, leader requirements, layering decisions, or a parallel authority system above those route objects. Authority flows through the route contracts themselves: admitted routes, witnesses, proofs, leases, and explicit lifecycle transitions.

## Purity And Side Effects

Jacquard treats purity and side effects as part of the trait contract.

- `Pure` traits must be deterministic with respect to their inputs. They should not perform I/O, read ambient time, allocate order stamps, or mutate hidden state that changes outputs.
- `Read-only` traits may inspect owned state or snapshots, but they must not mutate canonical routing truth or perform runtime effects.
- `Effectful` traits may perform I/O or mutate owned runtime state, but only through an explicit boundary with a narrow purpose.

Signature design follows the same split. Use `&self` for pure and read-only methods. Use `&mut self` only when the method has explicit state mutation or side effects. Do not mix pure planning and effectful runtime mutation in one trait unless the split is impossible and documented.

That is why Jacquard separates `RoutingEnginePlanner` from `RoutingEngine`, `SubstratePlanner` from `SubstrateRuntime`, and `LayeredRoutingEnginePlanner` from `LayeredRoutingEngine`. Engine-specific read-only seams such as mesh topology access stay in the owning engine crate rather than leaking into `jacquard-traits`. The shared tick lifecycle follows the same rule: router-owned cadence and `RoutingTickContext` / `RoutingTickOutcome` live at the contract layer, while engine-specific control loops and control-state contents stay inside the owning engine crate.

## Enforcement

Trait purity and routing invariants are enforced by the lint suite. `cargo xtask` provides the stable-toolchain check lane, and the nightly Dylint libraries under `lints/trait_purity`, `lints/model_policy`, and `lints/routing_invariants` provide compiler-backed coverage. The live rule set lives in `crates/xtask/src/checks/` and the three lint crates. Public trait definitions in `jacquard-traits` also carry `#[purity(...)]` or `#[effect_trait]` annotations that the `#[purity(...)]` proc macro validates at compile time.

## Runtime Boundary

The routing core does not call platform APIs directly. Hashing, storage, route-event logging, transport observations, time, and ordering all cross explicit shared boundaries in `traits`. That is how native execution, tests, and simulation share one semantic model.

The effect traits are narrower than the higher-level component traits. They model runtime capabilities, not whole subsystems. `RoutingEngine`, `Router`, and `RetentionStore` are larger behavioral contracts and should not be forced through the effect layer.

First-party mesh keeps one additional internal layer above those shared effects: mesh-private choreography effect interfaces generated from Telltale protocols. Those generated interfaces are not promoted into `jacquard-traits`. Concrete host/runtime adapters implement the shared effect traits, and `jacquard-mesh` interprets its private choreography requests in terms of those stable shared boundaries.

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
| `jacquard-mesh` | Mesh-private forwarding state, topology caches, repair state, retention state, engine-local committee scoring, and the private choreography guest runtime plus its protocol checkpoints. |
| `jacquard-router` | Canonical route identity, materialization inputs, leases, handle issuance, top-level route-health publication, and multi-engine orchestration state. |
| `jacquard-mem-node-profile` | In-memory node capability and node-state modeling only. No routing semantics. |
| `jacquard-mem-link-profile` | In-memory link capability, carrier, retention, and runtime-effect adapter state only. No canonical routing truth. |
| `jacquard-reference-client` | Narrow host-side composition of profile implementations, router, and engine instances for tests and examples. Observational with respect to canonical route truth. |
| `jacquard-simulator` (future) | Replay artifacts, scenario traces, post-run analysis. No canonical route truth during a live run. |

A host-owned policy engine above the router may own cross-engine migration policy and substrate selection.

## Extensibility

`core::Configuration` is the shared graph-shaped world object. Engine-specific structure such as topology exports, peer novelty, bridge estimates, planning caches, and forwarding tables belongs in the engine crate behind its trait boundary rather than in `core`.

The extension surface is split across [World Extensions](302_world_extensions.md), [Routing Engines](303_routing_engines.md), [Runtime Effects](301_runtime_effects.md), and [Mesh Routing](401_mesh_routing.md).

For first-party mesh specifically, Telltale stays an internal implementation substrate. Shared crates remain runtime-free. The future router may drive mesh through shared planning, tick, maintenance, and checkpoint orchestration, but it must not depend on mesh-private choreography payloads, protocol session keys, or guest-runtime internals.
