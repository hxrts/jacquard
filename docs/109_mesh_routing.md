# Mesh Routing

`jacquard-mesh` is Jacquard's first-party routing-engine implementation. It consumes the shared world model from `jacquard-core` and implements the stable routing boundaries from `jacquard-traits`. Mesh-only heuristics, runtime caches, and repair state remain inside the mesh crate.

## Shared Inputs

Mesh planning consumes the shared world picture from `jacquard-core`. The engine reads `Observation<Configuration>`, `Node`, `Link`, `Environment`, `ServiceDescriptor`, and `NodeRelayBudget` without wrapping or reshaping them.

The mesh engine treats `ServiceDescriptor` as the shared capability-advertisement plane. Route-capable mesh nodes expose the default Jacquard routing surface, which includes the `Discover`, `Move`, and `Hold` services along with relay headroom, hold capacity, link-quality observations, and coarse environment posture. Mesh does not add a second advertisement protocol or a mesh-global algorithm handshake on top of that surface.

The static `MESH_CAPABILITIES` envelope is exercised by contract tests. The in-tree mesh crate proves its `Repair`, `Hold`, partition-tolerance, decidable-admission, and explicit-route-shape claims against live planner and runtime behavior.

## Deterministic Topology Model

`DeterministicMeshTopologyModel` is the mesh-owned read-only query surface. It queries shared `Configuration` objects and then derives four mesh-private estimate types: `MeshPeerEstimate`, `MeshNeighborhoodEstimate`, `MeshMediumState`, and `MeshNodeIntrinsicState`. These estimates are encapsulated in `jacquard-mesh` so engine-specific scoring doesn't leak into the shared cross-engine schema.

Peer and neighborhood estimates expose optional score components, so unknown and zero remain distinct without turning those mesh-private components into shared observed facts. The components are clamped to the crate's `HealthScore` range so composition stays bounded. Where service validity matters, the topology model receives `observed_at_tick` explicitly rather than reinterpreting `RouteEpoch` as time. Mesh uses these estimates directly in candidate ordering and committee selection, so swapping the topology model changes mesh-private route preference and coordination behavior without changing the shared world schema.

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

Admission and witness generation operate on shared result objects. The mesh engine returns `RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, and `RouteWitness` values. This keeps mesh interoperable with the common router and layering surfaces. The routing-invariants check in `crates/xtask/src/checks/routing_invariants.rs` enforces the planning rules below.

- if a planning judgment depends on observations, the current topology must be passed explicitly to the planner method that makes that judgment
- `BackendRouteRef` stays opaque at the shared boundary, but in mesh it is a self-contained plan token rather than a cache key
- mesh may memoize derived candidates internally, but cache hits and misses must produce the same result for the same topology
- admitted routes carry that opaque backend ref forward so `materialize_route` can decode the selected mesh plan without searching planner cache state
- materialization still revalidates that decoded plan against the latest observed topology, the shared topology epoch, and the plan validity window before issuing a proof

Mesh route ids are path identities in v1. The stable route id is derived from source, destination, route class, and concrete segment path. Epoch stays in the plan token and proof instead of becoming part of the stable route identity. Mesh-private plan tokens, route-identity bytes, ordering keys, and runtime checkpoints all use the same versioned canonical binary encoding policy so replay, hashing, and checkpoint recovery stay aligned.

## Engine Middleware

`RoutingEngine::engine_tick` is the engine-wide progress hook for mesh. The router or host supplies a shared `RoutingTickContext`, and mesh returns a `RoutingTickOutcome` that reports whether the tick changed mesh-private state. Inside `jacquard-mesh`, this hook is the engine-internal middleware loop.

```text
topology observation
  -> refresh mesh-private estimates
  -> summarize transport ingress
  -> update bounded control state
  -> clear stale candidate cache
  -> checkpoint current mesh runtime state
```

Each tick ingests the latest topology observation, refreshes the mesh-private estimate caches, summarizes the latest bounded transport observations, and folds that evidence into a bounded control state. In v1 that control state carries transport stability, repair pressure, and anti-entropy pressure with deterministic decay. Mesh uses it to tighten route health, escalate repair posture under sustained pressure, and make `AntiEntropyRequired` consume real anti-entropy debt rather than acting as pure bookkeeping. The hook then evicts stale candidate entries and writes the scoped topology-epoch checkpoint.

Discovery enters mesh through the shared world picture: nodes, links, environment, and service advertisements are already merged into `Observation<Configuration>` before the engine plans. Richer route exports, neighbor-advertisement choreography, and mesh-private anti-entropy state remain future mesh work rather than hidden v1 runtime behavior.

## Internal Choreography Surface

Mesh now carries a private Telltale choreography layer inside `jacquard-mesh`. This does not change the shared Jacquard routing contract. Router-facing planning, admission, materialization, maintenance, and tick flow still use the shared `RoutingEngine` and `MeshRoutingEngine` traits.

The internal split is:

- planner-local deterministic Rust:
  - topology interpretation
  - candidate search and ranking
  - committee scoring
  - route-health derivation
- choreography-backed cooperative protocols:
  - forwarding hop
  - activation handshake
  - bounded suffix repair
  - semantic handoff
  - hold / replay exchange

Larger multi-role protocols live as `.tell` sources compiled through the normal Telltale pipeline. Small helper protocols can stay inline with `tell!` when adjacency to Rust glue materially improves readability.

Mesh also keeps one mesh-owned choreography interpreter surface above the shared runtime traits. That interpreter maps protocol-local requests onto the existing Jacquard boundaries:

- `MeshTransport` for endpoint-addressed frame sends and ingress observations
- `RetentionStore` for deferred-delivery payload storage
- `RouteEventLogEffects` for replay-visible route events
- router-owned checkpoint orchestration for persisted mesh-private state

This is intentionally still mesh-private. The router should only observe shared route objects, shared tick context, shared tick outcome, and shared checkpoint orchestration. It should not depend on mesh-private choreography payloads or generated effect interfaces.

The generated or protocol-local Telltale effect interfaces are not the shared Jacquard effect contract. They stay inside `jacquard-mesh` as implementation-facing protocol surfaces. Concrete host adapters still implement the shared traits from `jacquard-traits`, and the mesh choreography interpreter translates protocol-local requests onto those stable cross-engine traits instead of replacing them.

At runtime, mesh entry points now cross one private guest-runtime layer before touching transport, retention, or route-event logging directly. `forward_payload`, materialization-side activation bookkeeping, maintenance-side repair and handoff bookkeeping, retained-payload replay, and tick ingress all enter that mesh-local choreography boundary first. The guest runtime stores small protocol checkpoints keyed by protocol kind plus route or tick session so recovery does not depend on hidden in-memory sequencing state.

## Runtime and Repair

Materialization stores a mesh-private active-route object under the router-owned canonical identity. In v1 that object contains the explicit `MeshPath`, optional `CommitteeSelection`, and a deterministic ordering key plus four route-private substates:

- `MeshForwardingState` for current owner, owner-relative next-hop cursor, in-flight frames, and last ack
- `MeshRepairState` for bounded repair budget and last repair tick
- `MeshHandoffState` for the last handoff receipt and handoff tick
- `MeshRouteAntiEntropyState` for partition mode, retained objects, and last anti-entropy refresh

Canonical route identity, admission, and lease ownership remain outside this mesh-private runtime object.

Mesh decodes the admitted opaque backend ref during materialization instead of recovering route shape from planner cache state. Token decode alone is not enough. The runtime re-derives the candidate against the latest observed topology and fails closed if the plan epoch, handle epoch, witness epoch, latest topology epoch, or plan validity window do not still agree. Materialization itself fails closed until the engine has observed topology through `engine_tick`, so mesh no longer synthesizes a pre-observation route health or an empty-world fallback.

### Route Health

Route health is derived from the active route's remaining suffix rather than from engine-global topology presence. Mesh validates the current owner-relative suffix against the latest observed topology and folds first-hop transport observations into that route-local view when available. It publishes `ReachabilityState::Unknown` when it lacks route-local validation data rather than pretending the route is generically reachable or unreachable.

### Lifecycle and Maintenance

Lifecycle sequencing is explicit and fail-closed. Mesh validates first, builds the next active-route state off to the side, persists the checkpoint, records the route event, and only then publishes the in-memory runtime mutation. If checkpoint or route-event logging fails, the new state is not committed.

Protocol checkpoints follow the same fail-closed rule. Mesh writes or updates the protocol checkpoint through the choreography guest runtime before treating that step as complete, and rollback paths remove route-scoped protocol checkpoints when materialization or teardown does not commit.

Maintenance is expressed through the shared `RouteMaintenanceResult` surface. In v1 mesh, repair means a bounded local suffix-repair algorithm over the latest observed topology. `LinkDegraded` and `EpochAdvanced` attempt to recompute the remaining suffix from the current owner to the final destination, consume one repair step on success, and escalate to typed replacement when no bounded patch is available or the repair budget is exhausted.

`CapacityExceeded` returns `ReplacementRequired` without flipping partition mode, since it indicates replacement pressure rather than partition evidence. `PartitionDetected` enters partition mode and reports the current retained-object count through `HoldFallback`. `PolicyShift` performs handoff and `AntiEntropyRequired` flushes retained payloads to recover. V1 mesh exposes one current commitment per route, so repair, handoff, and deferred-delivery posture stay inside the route runtime state rather than becoming separate concurrent commitments.

### Forwarding

A handoff advances the owner-relative cursor to the remaining suffix under the next owner. Forwarding then succeeds only for the current owner of that suffix.

Old owners fail closed with `StaleOwner`, exhausted owner-relative paths fail with `Invalidated`, and malformed admitted plan tokens fail the same way during materialization. Each case maps to a typed `RouteMaintenanceResult` value rather than a side-channel mutation.

## Optional Committee Coordination

Mesh can attach a swappable `CommitteeSelector`, with `DeterministicCommitteeSelector` as the optional in-tree implementation. The selector returns `Option<CommitteeSelection>` rather than assuming a committee always exists. The in-tree selector reads the mesh neighborhood estimate for committee gating and the mesh peer estimate for ranking. Route ordering and local coordination stay on the same topology-model interpretation.

Committee eligibility is stricter than forwarding value alone. A member must be route-capable for mesh, must present a usable shared service surface, and may be disqualified by bounded behavior-history penalties before ranking happens. Selection is diversity-constrained: controller diversity is mandatory, and discovery-scope diversity is enforced when alternatives exist. If discovery-scope diversity would suppress the minimum viable committee, mesh falls back to controller-only diversity rather than silently disabling coordination.

`None` means no committee applies, and a selector error is not silently downgraded to `None`. Mesh surfaces a selector error as a typed inadmissible candidate using `BackendUnavailable`. The result is advisory coordination evidence only and does not replace canonical route admission, route witness, or route lease ownership.

## Retention and Storage

The mesh engine uses the shared `RetentionStore` boundary for deferred-delivery payloads. While a route is in partition mode, `forward_payload` buffers payloads into the retention store instead of sending them immediately. Mesh then flushes those retained payloads on recovery or before handoff when a next hop becomes available. The typed partition fallback surface remains `RouteMaintenanceOutcome::HoldFallback`, which now carries the retained-object count visible on the route at the time the fallback was entered.

Retained payload identity flows through the shared `Hashing` boundary. Route and runtime checkpoints flow through the shared storage and route-event-log effects. Storage keys and runtime checkpoints are scoped by the local engine identity so multiple local mesh engines can share one backend without overwriting one another.

V1 mesh supports a scoped checkpoint round-trip for mesh-private active-route state and the latest topology epoch. That recovery surface is intentionally narrow: it restores the mesh-owned runtime object keyed by `RouteId`, while canonical route identity and lease ownership remain on the router side.

The choreography layer adds a second scoped recovery surface: protocol checkpoints are keyed by protocol kind plus route session or tick session and round-trip through the same storage boundary. Route recovery still uses the active-route checkpoint; protocol recovery uses the protocol checkpoint catalog. Neither requires ambient hidden state outside the engine-owned checkpoint store.

## Swappable Trait Surface

The mesh engine exposes its internal seams as four traits in `jacquard-traits`. Substituting any of them replaces one mesh subcomponent without forking the engine. For host runtime effects beyond these mesh-specific seams, the engine uses the shared `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, and `Hashing` surfaces from `jacquard-traits`.

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
pub struct MeshFrame<'a> {
    pub endpoint: &'a LinkEndpoint,
    pub payload: &'a [u8],
}

pub trait MeshTransport {
    #[must_use]
    fn transport_id(&self) -> TransportProtocol;

    fn send_frame(&mut self, frame: MeshFrame<'_>) -> Result<(), TransportError>;

    fn poll_observations(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}
```

`MeshTransport` is the frame-carrier boundary for sending explicit endpoint-addressed mesh frames and reporting transport observations. Mesh routes forwarding through this frame-shaped trait directly. Any implementation that satisfies `MeshTransport` also satisfies `TransportEffects` from [Runtime Effects](104_runtime_effects.md) via blanket impl, so a mesh transport adapter can double as a host transport effect handler without collapsing the mesh-specific carrier boundary. New transport implementations such as BLE GATT, Wi-Fi LAN, or QUIC implement this trait and register with the mesh routing engine, with substantial platform logic moving into dedicated crates such as `jacquard-transport-ble`.

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

`RetentionStore` is the storage boundary for opaque deferred-delivery payloads during partitions. It stays intentionally narrow so platform-specific persistence can substitute without forcing the rest of the mesh engine to know about it.
