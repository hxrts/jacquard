# Profile Implementations

`jacquard-mem-node-profile`, `jacquard-mem-link-profile`, and `jacquard-reference-client` are Jacquard's in-tree profile and composition crates. The two `mem-*` crates model node and link inputs without importing routing logic. The reference client composes those profile implementations with `jacquard-router` and `jacquard-mesh` to exercise the full shared routing path in tests.

## Ownership Boundary

Profile crates are `Observed`. They model capability advertisement, transport carriage, and link-level state. They do not plan routes, issue canonical handles, publish route truth, or interpret routing policy. Canonical route ownership remains on the router, and engine-private runtime state remains inside the routing engine. This keeps profile code reusable across routing engines and prevents observational fixtures from drifting into shadow control planes.

`jacquard-core` types flow through these crates unchanged. `Node`, `NodeProfile`, `NodeState`, `Link`, `LinkEndpoint`, `LinkState`, and `ServiceDescriptor` keep their shared-model shape end to end. The `mem-*` crates wrap builders around those shared objects instead of replacing or reshaping them, and the reference client hands the constructed world picture to the router as a plain `Observation<Configuration>`.

## Crate Responsibilities

| Crate | Provides | Shared boundary it implements |
| --- | --- | --- |
| `jacquard-mem-node-profile` | `SimulatedNodeProfile`, `NodeStateSnapshot`, `SimulatedServiceDescriptor` builders | none — it only emits `jacquard-core` model values |
| `jacquard-mem-link-profile` | `SimulatedLinkProfile`, `SharedInMemoryNetwork`, `InMemoryTransport`, `InMemoryRetentionStore`, `InMemoryRuntimeEffects`, BLE profile defaults | `TransportEffects`, `RetentionStore`, `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects` |
| `jacquard-reference-client` | `fixtures::{route_capable_node, active_link}`, `Client<Router>`, `MeshRouter`/`MeshClient` aliases, `build_mesh_client` | none — it is pure composition over the crates above |

The `mem-*` crates stay routing-engine-neutral: they carry frames, emit observations, and build shared model values, but they do not mint route truth or interpret routing policy. Reference-client fixtures are the single place where a service descriptor picks up the `MESH_ENGINE_ID` routing-engine tag, because that decision is composition, not profile.

## Composition

`build_mesh_client` and `build_mesh_client_with_profile` are the wiring entry points. They attach an `InMemoryTransport` to a `SharedInMemoryNetwork`, construct a `MeshEngine` over a `DeterministicMeshTopologyModel`, plug in an `InMemoryRetentionStore` and `InMemoryRuntimeEffects`, register that engine on a fresh `MultiEngineRouter`, and return a `MeshClient`. Multiple clients built against the same network share one deterministic carrier.

```mermaid
graph LR
  NodeProfile[jacquard-mem-node-profile<br/>SimulatedNodeProfile<br/>NodeStateSnapshot<br/>SimulatedServiceDescriptor]
  LinkProfile[jacquard-mem-link-profile<br/>SimulatedLinkProfile<br/>InMemoryTransport<br/>InMemoryRetentionStore<br/>InMemoryRuntimeEffects]
  Network((SharedInMemoryNetwork))
  Ref[jacquard-reference-client<br/>fixtures + build_mesh_client]
  Router[MultiEngineRouter]
  Mesh[MeshEngine]

  NodeProfile --> Ref
  LinkProfile --> Ref
  Network --> LinkProfile
  Ref --> Router
  Ref --> Mesh
  Router -- registers --> Mesh
```

The reference end-to-end example is [`e2e_multi_layer_routing.rs`](../crates/reference-client/tests/e2e_multi_layer_routing.rs). It shows how to add a new client runtime to the same in-memory network without bypassing the router-owned canonical path.

## Extension Guidance

Mirror the existing layering when adding a new device or transport profile. Build node-side world inputs as builders over the shared `NodeProfile`, `NodeState`, and `ServiceDescriptor` types. Build link-side and transport behavior as adapters that implement the shared effect boundaries listed above. Compose the new profile with the router and a routing engine through a host harness that looks like `jacquard-reference-client`. Do not introduce a parallel node schema or a mesh-specific transport trait along the way.

Keep the ownership boundary strict. Profile crates stay `Observed`. Routers stay the canonical `ActorOwned` route publisher. Routing engines own only route-private runtime state and typed evidence. The [Crate Architecture](999_crate_architecture.md) document has the full dependency graph and ownership rules these crates fit into.
