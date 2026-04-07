# Mesh Routing

`jacquard-mesh` is Jacquard's first-party routing-engine implementation. It consumes the shared world model from `jacquard-core` and implements the stable routing boundaries from `jacquard-traits`. Mesh-only heuristics, runtime caches, and repair state remain inside the mesh crate.

## Shared Inputs

Mesh planning consumes the shared world picture from `jacquard-core`. The engine reads `Observation<Configuration>`, `Node`, `Link`, `Environment`, `ServiceDescriptor`, and `NodeRelayBudget` without wrapping or reshaping them.

The mesh engine treats `ServiceDescriptor` as the shared capability-advertisement plane. Route-capable mesh nodes expose the default Jacquard routing surface, which includes the `Discover`, `Move`, and `Hold` services along with relay headroom, hold capacity, link-quality observations, and coarse environment posture. Mesh does not add a second advertisement protocol or a mesh-global algorithm handshake on top of that surface.

The static `MESH_CAPABILITIES` envelope is exercised by contract tests. The in-tree mesh crate proves its `Repair`, `Hold`, partition-tolerance, decidable-admission, and explicit-route-shape claims against live planner and runtime behavior rather than leaving them as unchecked constants.

## Deterministic Topology Model

`DeterministicMeshTopologyModel` is the mesh-owned read-only query surface. It queries shared `Configuration` objects and then derives four mesh-private estimate types: `MeshPeerEstimate`, `MeshNeighborhoodEstimate`, `MeshMediumState`, and `MeshNodeIntrinsicState`. These estimates are encapsulated in `jacquard-mesh` so engine-specific scoring doesn't leak into the shared cross-engine schema.

The mesh estimate vocabulary is intentionally narrow. Peer and neighborhood estimates expose optional score components, so unknown and zero remain distinct without turning those mesh-private components into shared observed facts. Those mesh-private scores are also clamped to the crate's `HealthScore` range so composition stays bounded. Where service validity matters, the topology model receives `observed_at_tick` explicitly rather than reinterpreting `RouteEpoch` as time. Mesh now uses the peer and neighborhood estimates directly in candidate ordering and committee selection, so swapping the topology model can change mesh-private route preference and coordination behavior without changing the shared world schema.

## Planning and Admission

The mesh engine implements the shared `RoutingEnginePlanner` contract, which produces a candidate in five deterministic steps:

1. read the current topology snapshot
2. run bounded deterministic weighted path search from the local node
3. filter to route-capable destinations that match the routing objective
4. derive a deterministic backend reference, route id, cost, and estimate
5. sort candidates by path metric, mesh-private topology-model preference, and deterministic route key

This algorithm produces a stable candidate ordering across replays. In v1, the search metric is integer-only and combines hop count, delivery confidence, loss-derived congestion, symmetry, mesh-private peer and neighborhood estimates, protocol-repeat penalties, protocol-diversity bonuses, and a deferred-delivery bonus when the destination is honestly hold-capable. The shared `RouteCost` surface then reflects the chosen path's hop count, confidence, symmetry, congestion, protocol diversity, and deferred-delivery hold reservation without exposing the mesh-private estimate internals that shaped the search.

Deferred-delivery classification is deliberately stricter than capability advertisement alone. A destination only qualifies for retention-biased routing when its `Hold` service advertisement is currently valid for mesh, the advertised capacity hint reports positive `hold_capacity_bytes`, and the node state separately reports positive `hold_capacity_available_bytes`. A stale advertisement, an empty capacity hint, or unknown live capacity is not enough.

### Admission Contract

Admission and witness generation operate on shared result objects. The mesh engine returns `RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, and `RouteWitness` values. This keeps the mesh engine interoperable with the common router and layering surfaces.

The rule is:

- if a planning judgment depends on observations, the current topology must be passed explicitly to the planner method that makes that judgment
- `BackendRouteRef` stays opaque at the shared boundary, but in mesh it is a self-contained plan token rather than a cache key
- mesh may memoize derived candidates internally, but cache hits and misses must produce the same result for the same topology
- admitted routes carry that opaque backend ref forward so `materialize_route` can decode the selected mesh plan without searching planner cache state
- materialization still revalidates that decoded plan against the latest observed topology, the shared topology epoch, and the plan validity window before issuing a proof

These rules are enforced in-repo. See `crates/xtask/src/checks/routing_invariants.rs` for the live rule set.

## Engine Middleware

`RoutingEngine::engine_tick` is the engine-wide progress hook for mesh. Inside `jacquard-mesh`, this hook is the engine-internal middleware loop.

```text
topology observation
  -> refresh mesh-private estimates
  -> summarize transport ingress
  -> update bounded control state
  -> clear stale candidate cache
  -> checkpoint current mesh runtime state
```

Each tick ingests the latest topology observation, refreshes the mesh-private estimate caches, summarizes the latest bounded transport observations, and folds that evidence into a bounded control state. In v1 that control state carries transport stability, repair pressure, and anti-entropy pressure with deterministic decay. Mesh uses it to tighten route health, escalate repair posture under sustained pressure, and make `AntiEntropyRequired` consume real anti-entropy debt rather than acting as pure bookkeeping. The hook then evicts stale candidate entries and writes the scoped topology-epoch checkpoint. The host or router still provides the tick cadence.

Discovery enters mesh through the shared world picture: nodes, links, environment, and service advertisements are already merged into `Observation<Configuration>` before the engine plans. Richer route exports, neighbor-advertisement choreography, and mesh-private anti-entropy state remain future mesh work rather than hidden v1 runtime behavior.

## Runtime and Repair

Materialization stores a mesh-private active-route object under the router-owned canonical identity. In v1 that object contains the explicit `MeshPath`, optional `CommitteeSelection`, and a deterministic ordering key plus four route-private substates:

- `MeshForwardingState` for current owner, owner-relative next-hop cursor, in-flight frames, and last ack
- `MeshRepairState` for bounded repair budget and last repair tick
- `MeshHandoffState` for the last handoff receipt and handoff tick
- `MeshRouteAntiEntropyState` for partition mode, retained objects, and last anti-entropy refresh

Canonical route identity, admission, and lease ownership remain outside this mesh-private runtime object.

Mesh decodes the admitted opaque backend ref during materialization instead of recovering route shape from planner cache state. Token decode alone is not enough. The runtime re-derives the candidate against the latest observed topology and fails closed if the plan epoch, handle epoch, witness epoch, latest topology epoch, or plan validity window do not still agree.

Materialization fails closed until the engine has observed topology through `engine_tick`. Mesh no longer synthesizes a pre-observation route health or an empty-world fallback to get past activation.

### Route Health

Route health is derived from the active route's remaining suffix rather than from engine-global topology presence. Mesh validates the current owner-relative suffix against the latest observed topology and folds first-hop transport observations into that route-local view when available. It publishes `ReachabilityState::Unknown` when it lacks route-local validation data rather than pretending the route is generically reachable or unreachable.

### Lifecycle and Maintenance

Lifecycle sequencing is explicit and fail-closed. Mesh validates first, builds the next active-route state off to the side, persists the checkpoint, records the route event, and only then publishes the in-memory runtime mutation. If checkpoint or route-event logging fails, the new state is not committed.

Maintenance is expressed through the shared `RouteMaintenanceResult` surface. In v1 mesh, repair means a bounded local suffix-repair algorithm over the latest observed topology. `LinkDegraded` and `EpochAdvanced` attempt to recompute the remaining suffix from the current owner to the final destination, consume one repair step on success, and escalate to typed replacement when no bounded patch is available or the repair budget is exhausted.

`CapacityExceeded` and `PartitionDetected` enter partition mode and report the current retained-object count through `HoldFallback`. `PolicyShift` performs handoff. `AntiEntropyRequired` can flush retained payloads and recover from partition mode. Lease-expiry remains a typed failure.

### Forwarding

A handoff advances the owner-relative cursor to the remaining suffix under the next owner. Forwarding then succeeds only for the current owner of that suffix.

Old owners fail closed with `StaleOwner`, exhausted owner-relative paths fail with `Invalidated`, and malformed admitted plan tokens fail the same way during materialization. Each case maps to a typed `RouteMaintenanceResult` value rather than a side-channel mutation.

## Optional Committee Coordination

Mesh can attach a swappable `CommitteeSelector`. `DeterministicCommitteeSelector` is the optional in-tree implementation.

The selector returns `Option<CommitteeSelection>` rather than assuming a committee always exists. In the in-tree selector, committee gating reads the mesh neighborhood estimate and committee ranking reads the mesh peer estimate. Route ordering and local coordination stay on the same topology-model interpretation.

Committee eligibility is stricter than forwarding value alone. A member must be route-capable for mesh, must present a usable shared service surface, and may be disqualified by bounded behavior-history penalties before ranking happens at all.

Selection is diversity-constrained. Controller diversity is mandatory, and discovery-scope diversity is enforced when alternatives exist. If discovery-scope diversity would suppress the minimum viable committee, mesh falls back to controller-only diversity rather than silently disabling coordination.

`None` means no committee applies. A selector error is not silently downgraded to `None`. Mesh surfaces it as a typed inadmissible candidate using `BackendUnavailable`. The result is advisory coordination evidence only and does not replace canonical route admission, route witness, or route lease ownership.

## Retention and Partition Handling

The mesh engine treats retention as part of the routing-engine contract. It uses the shared `RetentionStore` boundary for deferred-delivery payloads. While a route is in partition mode, `forward_payload` buffers payloads into the retention store instead of sending them immediately. Mesh then flushes those retained payloads on recovery or before handoff when a next hop becomes available. The typed partition fallback surface remains `RouteMaintenanceOutcome::HoldFallback`, which now carries the retained-object count visible on the route at the time the fallback was entered.

Retained payload identity flows through the shared `Hashing` boundary. Route and runtime checkpoints flow through the shared storage and route-event-log effects.

Storage keys and runtime checkpoints are scoped by the local engine identity so multiple local mesh engines can share one backend without overwriting one another.

V1 mesh now supports a scoped checkpoint round-trip for mesh-private active-route state and the latest topology epoch. That recovery surface is intentionally narrow: it restores the mesh-owned runtime object keyed by `RouteId`, while canonical route identity and lease ownership still remain on the router side.

## Swappable Trait Surface

The mesh engine exposes its internal seams as four traits in `jacquard-traits`. Substituting any of them replaces one mesh subcomponent without forking the engine.

### Topology Model

```rust
pub trait MeshTopologyModel {
    type PeerEstimate;
    type NeighborhoodEstimate;

    #[must_use]
    fn local_node(&self, local_node_id: &NodeId, configuration: &Configuration) -> Option<Node>;

    #[must_use]
    fn neighboring_nodes(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<(NodeId, Node)>;

    #[must_use]
    fn reachable_endpoints(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<LinkEndpoint>;

    #[must_use]
    fn adjacent_links(&self, local_node_id: &NodeId, configuration: &Configuration) -> Vec<Link>;

    #[must_use]
    fn peer_estimate(
        &self,
        local_node_id: &NodeId,
        peer_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<Self::PeerEstimate>;

    #[must_use]
    fn neighborhood_estimate(
        &self,
        local_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<Self::NeighborhoodEstimate>;
}
```

`MeshTopologyModel` is read-only. The associated estimate types are the important boundary. If a mesh implementation wants novelty scores, reach estimates, bridge heuristics, or neighborhood flow signals, those stay mesh-owned behind `MeshTopologyModel`. They are not promoted into `jacquard-core` as shared `Node`, `Link`, or `Environment` schema.

### Engine Binding

```rust
pub trait MeshRoutingEngine: RoutingEngine {
    type TopologyModel: MeshTopologyModel;
    type Transport: MeshTransport;
    type Retention: RetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel;

    fn transport(&self) -> &Self::Transport;

    fn transport_mut(&mut self) -> &mut Self::Transport;

    fn retention_store(&self) -> &Self::Retention;

    fn retention_store_mut(&mut self) -> &mut Self::Retention;
}
```

`MeshRoutingEngine` binds one concrete topology model, one transport implementation, and one retention store to a mesh engine instance. This keeps mesh-specific internals swappable without exposing them as shared cross-engine assumptions, while still letting mesh route choice depend on mesh-owned peer and neighborhood estimates behind that boundary.

### Transport

```rust
pub trait MeshTransport {
    #[must_use]
    fn transport_id(&self) -> TransportProtocol;

    fn send_frame(&mut self, endpoint: &LinkEndpoint, payload: &[u8])
        -> Result<(), TransportError>;

    fn poll_observations(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}
```

`MeshTransport` is the carrier boundary for sending frames and reporting transport observations. Any implementation that satisfies `MeshTransport` automatically satisfies `TransportEffects` from [Runtime Effects](108_runtime_effects.md) via blanket impl, so a mesh transport adapter doubles as a host transport effect handler.

### Retention

```rust
pub trait RetentionStore {
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError>;

    fn take_retained_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RetentionError>;

    fn contains_retained_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, RetentionError>;
}
```

`RetentionStore` is the storage boundary for opaque deferred-delivery payloads during partitions. New transport implementations such as BLE GATT, Wi-Fi LAN, or QUIC implement `MeshTransport` and are registered with the mesh routing engine. If transport implementations grow substantial platform logic, they should move into dedicated crates such as `jacquard-transport-ble`. `RetentionStore` stays intentionally narrow for the same reason.

## Runtime Services

`jacquard-mesh` routes host capabilities through the shared trait surfaces from `jacquard-traits`. It does not define parallel mesh-only runtime traits.

The mesh engine consumes `MeshTransport` for frame send and transport observations, and `RetentionStore` for deferred-delivery payload storage. It also uses the `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, and `Hashing` runtime surfaces. This concentrates mesh-specific logic in the mesh crate while preserving the stable runtime boundary.
