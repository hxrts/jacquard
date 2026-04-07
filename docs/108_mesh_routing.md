# Mesh Routing

`jacquard-mesh` is Jacquard's first-party routing-engine implementation. It consumes the shared world model from `jacquard-core` and implements the stable routing boundaries from `jacquard-traits`. Mesh-only heuristics, runtime caches, and repair state remain inside the mesh crate.

## Shared Inputs

Mesh planning consumes the shared world picture from `jacquard-core`. The engine reads `Observation<Configuration>`, `Node`, `Link`, `Environment`, `ServiceDescriptor`, and `NodeRelayBudget` without wrapping or reshaping them.

The mesh engine treats `ServiceDescriptor` as the shared capability-advertisement plane. Route-capable mesh nodes expose the default Jacquard routing surface, which includes the `Discover`, `Move`, and `Hold` services along with relay headroom, hold capacity, link-quality observations, and coarse environment posture. Mesh does not add a second advertisement protocol or a mesh-global algorithm handshake on top of that surface.

The static `MESH_CAPABILITIES` envelope is exercised by contract tests. The in-tree mesh crate proves its `Repair`, `Hold`, partition-tolerance, decidable-admission, and explicit-route-shape claims against live planner and runtime behavior rather than leaving them as unchecked constants.

## Deterministic Topology Model

`DeterministicMeshTopologyModel` is the mesh-owned read-only query surface. It queries shared `Configuration` objects and then derives four mesh-private estimate types: `MeshPeerEstimate`, `MeshNeighborhoodEstimate`, `MeshMediumState`, and `MeshNodeIntrinsicState`. These estimates are encapsulated in `jacquard-mesh` so engine-specific scoring doesn't leak into the shared cross-engine schema.

The mesh estimate vocabulary is intentionally narrow. Peer and neighborhood estimates expose optional score components, so unknown and zero remain distinct without turning those mesh-private components into shared observed facts. Where service validity matters, the topology model receives `observed_at_tick` explicitly rather than reinterpreting `RouteEpoch` as time. Mesh now uses the peer and neighborhood estimates directly in candidate ordering and committee selection, so swapping the topology model can change mesh-private route preference and coordination behavior without changing the shared world schema.

## Planning and Admission

The mesh engine implements the shared `RoutingEnginePlanner` contract, which produces a candidate in five deterministic steps:

1. read the current topology snapshot
2. build shortest local node paths in a stable order
3. filter to route-capable destinations that match the routing objective
4. derive a deterministic backend reference, route id, cost, and estimate
5. sort candidates by hop count, mesh-private topology-model preference, and deterministic route key

This algorithm produces a stable candidate ordering across replays.

Deferred-delivery classification is deliberately stricter than capability advertisement alone. A destination only qualifies for retention-biased routing when it both advertises `Hold` for mesh and currently reports positive available hold capacity in shared node state. A stale or capacity-free `Hold` advertisement is not enough.

Admission and witness generation operate on shared result objects. The mesh engine returns `RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, and `RouteWitness` values. This keeps the mesh engine interoperable with the common router and layering surfaces.

The rule is:

- if a planning judgment depends on observations, the current topology must be passed explicitly to the planner method that makes that judgment
- `BackendRouteRef` stays opaque at the shared boundary, but in mesh it is a self-contained plan token rather than a cache key
- mesh may memoize derived candidates internally, but cache hits and misses must produce the same result for the same topology
- admitted routes carry that opaque backend ref forward so `materialize_route` can decode the selected mesh plan without searching planner cache state

These rules are enforced in-repo. The routing-invariants lane checks the explicit-topology planner signatures, rejects synthetic default-topology fallback, guards the `Tick` / `RouteEpoch` split, and keeps mesh runtime sequencing fail-closed while the current debt is burned down.

## Engine Middleware

`RoutingEngine::engine_tick` is the engine-wide progress hook for mesh. Inside `jacquard-mesh`, this hook is the engine-internal middleware loop.

```text
topology observation
  -> refresh mesh-private estimates
  -> clear stale candidate cache
  -> summarize transport ingress
  -> checkpoint current mesh runtime state
```

Each tick ingests the latest topology observation, refreshes the mesh-private estimate caches, evicts stale candidate entries, summarizes the latest bounded transport observations, and writes the scoped topology-epoch checkpoint. The hook does not perform engine layering. The host or router still provides the tick cadence.

Discovery enters mesh through the shared world picture: nodes, links, environment, and service advertisements are already merged into `Observation<Configuration>` before the engine plans. Richer route exports, neighbor-advertisement choreography, and mesh-private anti-entropy state remain future mesh work rather than hidden v1 runtime behavior.

## Runtime and Repair

Materialization stores a mesh-private active-route object under the router-owned canonical identity. In v1 that object contains the explicit `MeshPath`, optional `CommitteeSelection`, the current owner node, the owner-relative next-hop cursor, the repair budget, in-flight forwarding counters, last-ack tick, partition-mode flag, retained-object set, and a deterministic ordering key. Canonical route identity, admission, and lease ownership remain outside this mesh-private runtime object. Mesh decodes the admitted opaque backend ref during this step instead of recovering route shape from planner cache state.

Materialization now fails closed until the engine has observed topology through `engine_tick`. Mesh no longer synthesizes a pre-observation route health or an empty-world fallback to get past activation.

Lifecycle sequencing is explicit and fail-closed. Mesh validates first, builds the next active-route state off to the side, persists the checkpoint, records the route event, and only then publishes the in-memory runtime mutation. If checkpoint or route-event logging fails, the new state is not committed.

Maintenance is expressed through the shared `RouteMaintenanceResult` surface. In v1 mesh, repair means a typed local repair-budget model: `LinkDegraded` consumes one repair step and returns `Repaired`, `EpochAdvanced` can spend that same budget to keep the route current, and budget exhaustion escalates to typed replacement rather than to a richer graph-edit object. `CapacityExceeded` and `PartitionDetected` enter partition mode, `PolicyShift` performs handoff, `AntiEntropyRequired` is currently a typed progress refresh over the shared-world view, and lease-expiry remains a typed failure. A handoff advances the owner-relative cursor to the remaining suffix under the next owner. Forwarding then succeeds only for the current owner of that suffix; old owners fail closed with `StaleOwner`, exhausted owner-relative paths fail with `Invalidated`, and malformed admitted plan tokens fail the same way during materialization. Each case maps to a typed `RouteMaintenanceResult` value rather than a side-channel mutation.

## Optional Committee Coordination

Mesh can attach a swappable `CommitteeSelector`. `DeterministicCommitteeSelector` is the optional in-tree implementation.

The selector returns `Option<CommitteeSelection>` rather than assuming a committee always exists. In the in-tree selector, committee gating reads the mesh neighborhood estimate and committee ranking reads the mesh peer estimate, so route ordering and local coordination stay on the same topology-model interpretation. `None` means no committee applies. A selector error is not silently downgraded to `None`; mesh surfaces it as a typed inadmissible candidate using `BackendUnavailable`. The result is advisory coordination evidence only. It does not replace canonical route admission, route witness, or route lease ownership.

## Retention and Partition Handling

The mesh engine treats retention as part of the routing-engine contract. It uses the shared `RetentionStore` boundary for deferred-delivery payloads. It exposes typed partition fallback through `RouteMaintenanceOutcome::HoldFallback`.

Retained payload identity flows through the shared `Hashing` boundary. Route and runtime checkpoints flow through the shared storage and route-event-log effects.

Storage keys and runtime checkpoints are scoped by the local engine identity so multiple local mesh engines can share one backend without overwriting one another.

V1 mesh now supports a scoped checkpoint round-trip for mesh-private active-route state and the latest topology epoch. That recovery surface is intentionally narrow: it restores the mesh-owned runtime object keyed by `RouteId`, while canonical route identity and lease ownership still remain on the router side.

## Runtime Services

`jacquard-mesh` routes host capabilities through the shared trait surfaces from `jacquard-traits`. It does not define parallel mesh-only runtime traits.

The mesh engine consumes `MeshTransport` for frame send and transport observations, and `RetentionStore` for deferred-delivery payload storage. It also uses the `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, and `Hashing` runtime surfaces. This concentrates mesh-specific logic in the mesh crate while preserving the stable runtime boundary.
