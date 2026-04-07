# Mesh Routing

`jacquard-mesh` is Jacquard's first-party routing-engine implementation. It consumes the shared world model from `jacquard-core` and implements the stable routing boundaries from `jacquard-traits`. Mesh-only heuristics, runtime caches, and repair state remain inside the mesh crate.

## Shared Inputs

Mesh planning consumes the shared world picture from `jacquard-core`. The relevant types are `Observation<Configuration>`, `Node`, `Link`, `Environment`, `ServiceDescriptor`, and `NodeRelayBudget`. The engine reads these directly without forking the schema.

The mesh engine treats `ServiceDescriptor` as the shared capability-advertisement plane. Route-capable mesh nodes expose the default Jacquard routing surface, which includes the `Discover`, `Move`, and `Hold` services along with relay headroom, hold capacity, link-quality observations, and coarse environment posture. Mesh does not add a second advertisement protocol or a mesh-global algorithm handshake on top of that surface.

## Deterministic Topology Model

`DeterministicMeshTopologyModel` is the mesh-owned read-only query surface. It uses shared `Configuration` objects directly. It derives mesh-private estimates above the shared boundary.

The mesh-private estimate types are `MeshPeerEstimate`, `MeshNeighborhoodEstimate`, `MeshMediumState`, and `MeshNodeIntrinsicState`. These types remain in `jacquard-mesh`. They are not promoted into the shared cross-engine schema.

## Planning and Admission

The mesh engine implements the shared `RoutingEnginePlanner` contract directly. Candidate production proceeds in five deterministic steps:

1. read the current topology snapshot
2. build shortest local node paths in a stable order
3. filter to route-capable destinations that match the routing objective
4. derive a deterministic backend reference, route id, cost, and estimate
5. sort candidates by hop count and deterministic route key

This algorithm produces a stable candidate ordering across replays.

Admission and witness generation operate on shared result objects. The mesh engine returns `RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, and `RouteWitness` values. This keeps the mesh engine interoperable with the common router and layering surfaces.

The clean rule is:

- if a planning judgment depends on observations, the current topology must be passed explicitly to the planner method that makes that judgment
- `BackendRouteRef` stays opaque at the shared boundary, but in mesh it is a self-contained plan token rather than a cache key
- mesh may memoize derived candidates internally, but cache hits and misses must produce the same result for the same topology

## Engine Middleware

`RoutingEngine::engine_tick` is the engine-wide progress hook for mesh. Inside `jacquard-mesh`, this hook is the engine-internal middleware loop.

```text
topology observation
  -> refresh mesh-private estimates
  -> clear stale candidate cache
  -> poll transport ingress
  -> checkpoint current mesh runtime state
```

Each tick ingests the latest topology observation, refreshes the mesh-private estimate caches, evicts stale candidate entries, polls transport ingress, and writes a runtime checkpoint. The hook does not perform engine layering. The host or router still provides the tick cadence.

## Runtime and Repair

Materialization stores a mesh-private active-route object under the router-owned canonical identity. The active-route record contains an explicit `MeshPath`, an optional `CommitteeSelection`, the repair budget, in-flight forwarding state, retention bookkeeping, and a deterministic ordering key. Canonical route identity, admission, and lease ownership remain outside this mesh-private runtime object.

Maintenance is expressed through the shared `RouteMaintenanceResult` surface. The current implementation handles repair on `LinkDegraded`, replacement when the repair budget is exhausted, partition-mode entry on `CapacityExceeded` and `PartitionDetected`, handoff on `PolicyShift`, and lease-expiry failure. Each case maps to a typed `RouteMaintenanceResult` value rather than a side-channel mutation.

## Optional Committee Coordination

Mesh can attach a swappable `CommitteeSelector`. `DeterministicCommitteeSelector` is the optional in-tree implementation.

The selector returns `Option<CommitteeSelection>` rather than assuming a committee always exists. The result is advisory coordination evidence only. It does not replace canonical route admission, route witness, or route lease ownership.

## Retention and Partition Handling

The mesh engine treats retention as part of the routing-engine contract. It uses the shared `RetentionStore` boundary for deferred-delivery payloads. It exposes typed partition fallback through `RouteMaintenanceOutcome::HoldFallback`.

Retained payload identity flows through the shared `Hashing` boundary. Route and runtime checkpoints flow through the shared storage and route-event-log effects.

## Runtime Services

`jacquard-mesh` routes host capabilities through the shared trait surfaces from `jacquard-traits`. It does not define parallel mesh-only runtime traits.

The mesh engine consumes `MeshTransport` for frame send and transport observations, and `RetentionStore` for deferred-delivery payload storage. It also uses the `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, and `Hashing` runtime surfaces. This concentrates mesh-specific logic in the mesh crate while preserving the stable runtime boundary.
