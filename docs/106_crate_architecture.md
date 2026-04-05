# Crate Architecture

This page describes the crate layout, the boundary rules, and the implementation policies that keep the workspace consistent.

## Boundary Rule

`core` defines what exists. `traits` defines what components are allowed to do.

`core` owns shared identifiers, data types, constants, error types, and the full model pipeline from world objects through observations, estimates, policy, and action. It must not grow behavioral traits for subcomponents. Derives, trivial constructors, and simple validation are allowed. Cross-crate behavioral interfaces are not.

`traits` owns all cross-crate behavioral interfaces. This includes the routing contract (`RoutingController`, `CommitteeSelector`, `SubstratePlanner`, `SubstrateRuntime`, `LayeredRoutePlanner`, `LayeredRouteFamily`, `LayerCoordinator`, `RoutePlanner`, `RouteFamily`, `Router`, `RoutingControlPlane`, `RoutingDataPlane`), the runtime effect traits (`TimeEffects`, `OrderEffects`, `HashEffects`, `StorageEffects`, `AuditEffects`, `TransportEffects`), the mesh-specialized traits (`MeshTopologyModel`, `MeshTransport`, `CustodyStore`, `MeshRouteFamily`), and the simulator traits (`RoutingScenario`, `RoutingEnvironmentModel`, `RoutingSimulator`, `RoutingReplayView`).

## Dependency Graph

```
jacquard-core
    ↑
jacquard-traits
    ↑
jacquard-mesh ──→ jacquard-router ←── jacquard-transport
      │               │
      └──────→ jacquard-simulator
```

Every crate depends on `jacquard-core`. Every crate except `jacquard-core` depends on `jacquard-traits`. The router depends on mesh only through the `RouteFamily` trait, not through mesh internals. The simulator depends on core, traits, mesh, and router, and uses `telltale-simulator` as the execution base.

`jacquard-core` and `jacquard-traits` must remain runtime-free. They may not depend on `telltale-runtime`.

`jacquard-core` compiles first with no workspace dependencies. `jacquard-traits` compiles against core. Mesh and transport compile against core and traits in parallel. The router compiles against core, traits, and mesh. The simulator compiles last against all of the above plus the Telltale simulator crates.

`jacquard-transport` is transitional. In this phase it provides structural stubs that satisfy trait signatures without real radio integration. If transport implementations grow substantial platform logic, split them into dedicated crates such as `jacquard-transport-ble` and `jacquard-transport-wifi`.

## Crate Layout

Inside `core`, files are grouped into three areas. `base/` holds cross-cutting primitives: identity, time, qualifiers, constants, and errors. `model/` holds the world-to-action pipeline: world objects, observations, estimation, policy, and action. `routing/` holds route lifecycle and runtime coordination objects: admission, runtime state, capabilities, and audit. Small transport and content files stay at the crate root.

This is also the main abstraction boundary for how opinionated Jacquard should be. `core` may define shared coordination result objects such as `CommitteeSelection`, identity-assurance qualifiers, and evidence classes. It must not define family-local committee scoring policy, require a leader, or turn one routing family's grouping heuristic into a workspace-wide law.

The same minimality rule applies to family layering. `core` may define substrate requirements, substrate leases, and layer parameters. It must not make one family natively aware of another family or force one composition policy on every host.

## Purity And Side Effects

Jacquard treats purity and side effects as part of the trait contract.

- `Pure` traits must be deterministic with respect to their inputs. They should not perform I/O, read ambient time, allocate order stamps, or mutate hidden state that changes outputs.
- `Read-only` traits may inspect owned state or snapshots, but they must not mutate canonical routing truth or perform runtime effects.
- `Effectful` traits may perform I/O or mutate owned runtime state, but only through an explicit boundary with a narrow purpose.

Signature design follows the same split. Use `&self` for pure and read-only methods. Use `&mut self` only when the method has explicit state mutation or side effects. Do not mix pure planning and effectful runtime mutation in one trait unless the split is impossible and documented.

That is why Jacquard separates `RoutePlanner` from `RouteFamily`, `SubstratePlanner` from `SubstrateRuntime`, `LayeredRoutePlanner` from `LayeredRouteFamily`, `MeshTopologyModel` from `MeshTransport`, and `RoutingScenario` / `RoutingEnvironmentModel` from `RoutingSimulator`.

This rule is enforced in three layers. Public trait definitions in `jacquard-traits` carry `#[purity(...)]` or `#[effect_trait]` annotations. The `#[purity(...)]` proc macro rejects obvious receiver-shape violations such as `&mut self` on pure traits. The repository also ships `scripts/check/trait-purity.sh` and a companion Dylint library for workspace-wide annotation checks.

The routing core does not call platform APIs directly. Hashing, storage, audit emission, transport ingress, time, and ordering all cross explicit runtime-effect traits in `traits`. That is how native execution, tests, and simulation share one semantic model. The effect traits are narrower than the higher-level component traits. They model runtime capabilities, not whole subsystems. `RouteFamily`, `Router`, and `CustodyStore` are larger behavioral contracts and should not be forced through the effect layer.

## Invariants And Ownership

Cross-crate invariants:

- No crate may use floating-point types in routing logic, routing state, routing policy, or simulator verdicts.
- No crate may treat wall-clock time as distributed semantic truth.
- Canonical ordering must flow through shared ordering types. Crates must not invent crate-local tie-break schemes.
- Canonical hashing and content IDs must flow through the shared hash and content-addressing boundaries.
- Transport may observe links and carry bytes, but it may not invent route truth, publish canonical route health, or mutate materialized-route ownership.
- GPS, absolute location, clique grids, and singleton leaders are not shared routing truth. If a family uses spatial hints or local coordination structures, those remain family-private interpretations above the shared observation boundary.
- Multiple families may coexist in one host runtime. Gradual migration between families is allowed, and limited layering is allowed through the shared substrate boundary. Generic mixed-family canonical route ownership is not a base-layer assumption.

Ownership by crate:

- `jacquard-router` owns canonical route materialization, lease transfer, route replacement, canonical handle issuance, and top-level route-health publication.
- a host-owned layer coordinator above the router may own cross-family migration policy and substrate selection policy
- `jacquard-mesh` owns mesh-private forwarding state, topology caches, route repair state, route exports, family-side route commitments, deferred-delivery custody state, and any family-local committee scoring or misbehavior tracking.
- `jacquard-transport` owns local transport observations and device-facing adapter state only.
- `jacquard-simulator` owns replay artifacts, scenario traces, and post-run analysis outputs. It does not own canonical route truth during a live run.
- `jacquard-core` owns the shared vocabulary. It does not own live state.
- `jacquard-traits` owns compile-time boundaries. It does not own runtime state.

## Extensibility

`core::Configuration` is the shared graph-shaped world object. If mesh needs geometry, richer topology exports, or other spatial structure, those should live in `MeshConfiguration` or other mesh-owned types rather than being pushed into the base `Environment`. The same rule applies to any family-specific state. Family-private planning caches, forwarding tables, and custody stores belong in the family crate, not in `core`.

The same minimality rule applies to coordination and layering. The shared boundary may expose the shape of a committee or witness-set result, substrate requirements, substrate leases, and layer parameters, but it should not force one committee algorithm, one leader model, one adversary heuristic, or one layering policy onto every family. That policy belongs in the family crate or embedding host.

External route families should depend on `jacquard-core` and `jacquard-traits`. They should not depend on mesh internals, router internals, or simulator-private helpers. An external family must implement `RouteFamily` and treat `RouteSummary`, `Estimate<RouteEstimate>`, `RouteAdmissionCheck`, `RouteWitness`, `RouteHandle`, `RouteLease`, `RouteCommitment`, `RouteMaintenanceResult`, `CommitteeSelection`, `SubstrateRequirements`, `SubstrateLease`, `LayerParameters`, `Observation<T>`, and `Fact<T>` as the stable cross-crate contract. It must not assume mesh route shape, mesh topology structure, mesh-specific maintenance semantics, or that the shared committee or substrate abstraction implies direct family awareness.
