# Pathway Routing

`jacquard-pathway` is Jacquard's first-party routing-engine implementation. It consumes the shared world model from `jacquard-core` and implements the stable routing boundaries from `jacquard-traits`. Pathway-only heuristics, runtime caches, and repair state remain inside the pathway crate. Proactive engines such as `babel`, `batman-bellman`, `batman-classic`, and `olsrv2` are separate routing-engine crates that do not change pathway's explicit-path semantics.

## Shared Inputs

Pathway planning consumes the shared world picture from `jacquard-core`. The engine reads `Observation<Configuration>`, `Node`, `Link`, `Environment`, `ServiceDescriptor`, and `NodeRelayBudget` without wrapping or reshaping them.

The pathway engine treats `ServiceDescriptor` as the shared capability-advertisement plane. Route-capable pathway nodes expose the default Jacquard routing surface, which includes the `Discover`, `Move`, and `Hold` services along with relay headroom, hold capacity, link-quality observations, and coarse environment posture. Pathway does not add a second advertisement protocol or a pathway-global algorithm handshake on top of that surface.

The static `PATHWAY_CAPABILITIES` envelope is exercised by contract tests. The in-tree pathway crate proves its `Repair`, `Hold`, partition-tolerance, decidable-admission, and explicit-route-shape claims against live planner and runtime behavior.

## Deterministic Topology Model

`DeterministicPathwayTopologyModel` is the pathway-owned read-only query surface. It queries shared `Configuration` objects and then derives pathway-private estimate types such as `PathwayPeerEstimate` and `PathwayNeighborhoodEstimate`. Those estimates stay encapsulated in `jacquard-pathway` so engine-specific scoring does not leak into the shared cross-engine schema.

Peer and neighborhood estimates expose optional score components, so unknown and zero remain distinct without turning those pathway-private components into shared observed facts. The components are clamped to the crate's `HealthScore` range so composition stays bounded. Where service validity matters, the topology model receives `observed_at_tick` explicitly rather than reinterpreting `RouteEpoch` as time. Pathway uses these estimates directly in candidate ordering and committee selection, so swapping the topology model changes pathway-private route preference and coordination behavior without changing the shared world schema.

## Planning and Admission

The pathway engine implements the shared `RoutingEnginePlanner` contract, which produces candidates in five deterministic steps:

1. read the current topology snapshot
2. freeze a `PathwaySearchDomain` over deterministic `NodeId` graph state for that snapshot
3. resolve the routing objective into one v13 `SearchQuery`: `SearchQuery::SingleGoal` for one exact destination, or `SearchQuery::CandidateSet` for selector-style service and gateway objectives over a deterministic acceptable-destination set
4. run Telltale's canonical search machine once for that query under an explicit `SearchExecutionPolicy` plus declared fairness bundle
5. derive deterministic backend references, route ids, costs, and estimates from the selected-result witness and the final authoritative search state, then sort by path metric, pathway-private topology-model preference, and deterministic route key

This algorithm produces a stable candidate ordering across replays. The search metric is integer-only and combines hop count, delivery confidence, loss-derived congestion, symmetry, pathway-private peer and neighborhood estimates, protocol-repeat penalties, protocol-diversity bonuses, and a deferred-delivery bonus when the destination is honestly hold-capable. The shared `RouteCost` surface then reflects the chosen path's hop count, confidence, symmetry, congestion, protocol diversity, and deferred-delivery hold reservation without exposing the pathway-private estimate internals that shaped the search.

The direct per-link reliability inputs are `delivery_confidence_permille`, `symmetry_permille`, and `loss_permille`. Pathway turns these into weighted edge penalties during path search. Higher delivery confidence and better symmetry reduce path cost. Higher loss increases it.

These signals are then combined with pathway-private peer and neighborhood bonuses and penalties rather than collapsed into one shared reliability field.

`median_rtt_ms` is part of the shared link observation surface. Pathway does not currently use it in path scoring.

Deferred-delivery classification is deliberately stricter than capability advertisement alone. A destination only qualifies for retention-biased routing when its `Hold` service advertisement is currently valid for pathway, the advertised capacity hint reports positive `hold_capacity_bytes`, and the node state separately reports positive `hold_capacity_available_bytes`. A stale advertisement, an empty capacity hint, or unknown live capacity is not enough.

### Telltale Search Core

The generic search core lives in `telltale-search`, and Pathway supplies a domain adapter plus route-specific policy on top of it.

`telltale-search` owns:

- canonical batch extraction and search-machine semantics over weighted graph state
- generalized `SearchQuery` handling, selected-result semantics, and witness publication
- `SearchExecutionPolicy` / `SearchRunConfig` validation with explicit fairness assumptions
- replay artifacts, canonical observation reconstruction, and observation comparison
- epoch reconfiguration through `EpochReconfigurationRequest` with explicit reseeding policy
- theorem-backed exactness and fairness claims for the exposed runtime profiles

Pathway owns:

- topology interpretation and edge-cost policy
- heuristic policy (`Zero` or the current hop-lower-bound heuristic)
- objective-to-`SearchQuery` mapping for `Node`, `Service`, and `Gateway` destinations
- candidate-path derivation from the final search state, route class, connectivity posture, and route summary
- admission policy, witness generation, and committee handling
- opaque backend-token encoding and cache-miss re-derivation

This split is intentional. Pathway uses the generic search machine as a deterministic planning substrate. The published route semantics remain Pathway-owned.

### Inherited Search Features

The Pathway engine inherits several capabilities directly from the v13 Telltale search system:

- canonical batch extraction instead of planner-local frontier bookkeeping
- one objective-scoped `SearchQuery` execution rather than a planner-local loop that rebuilds selector semantics out of repeated single-goal runs
- selected-result and witness semantics exported directly by the search runtime
- explicit execution-policy control through `SearchExecutionPolicy` and `SearchRunConfig`
- replay artifacts that preserve epoch trace, batch schedule, fairness bundle, and final authoritative state
- explicit epoch reconfiguration with a real reseeding policy, where Pathway currently uses `PreserveOpenAndIncons`

Pathway currently uses `SearchQuery::SingleGoal` for exact node destinations and `SearchQuery::CandidateSet` for service or gateway objectives that select among multiple acceptable destinations. For exact queries, the runtime can also emit the optional path-problem helper surfaces. Candidate-set queries stay on the generic selected-result surface and intentionally do not rely on a distinguished goal anchor.

Pathway currently exposes only exact run-to-completion profiles to the router. The supported public modes are canonical serial and threaded exact single-lane, both with `batch_width = 1`, `SearchCachingProfile::EphemeralPerStep`, and `SearchEffortProfile::RunToCompletion`.

Budgeted or bounded execution contracts remain part of the generic Telltale runtime surface. Pathway rejects them fail-closed for router-visible planning until it has a Pathway-owned policy for exposing them.

### Proof and Assurance Surface

Pathway also inherits proof-oriented guarantees and trace surfaces from the Telltale runtime:

- fail-closed configuration validation before execution, including scheduler profile, batch width, executor compatibility, caching profile, effort profile, and fairness bundle
- explicit determinism and fairness claims tied to the selected scheduler profile rather than hidden host-runtime assumptions
- replay and observation-comparison surfaces that can reconstruct and compare the final observed selected result
- state and artifact traces that expose canonical batches, normalized commits, fairness certificates, epoch transitions, and final authoritative machine state
- theorem-backed exact observable equivalence between canonical serial and threaded exact single-lane execution for the current Pathway domain

These guarantees belong to the Telltale search substrate, not to Pathway's route policy layer. Pathway relies on them to justify exactness, replayability, and debug visibility at the search boundary while still owning topology freezing, route-objective mapping, candidate derivation, and router publication semantics.

Pathway defaults to canonical serial search with `batch_width = 1`, `epsilon = 1.0`, `SearchCachingProfile::EphemeralPerStep`, `SearchEffortProfile::RunToCompletion`, and the minimum exact fairness bundle required by the generic runtime. `ThreadedExactSingleLane` is available as an explicit opt-in planner mode. Batched parallel, budgeted, and bounded profiles are not exposed because the weaker fairness or approximation story is not acceptable for default routing behavior.

### Admission Contract

Admission and witness generation operate on shared result objects. The pathway engine returns `RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, and `RouteWitness` values. This keeps pathway interoperable with the common router and layering surfaces. The routing-invariants check in `toolkit/checks/rust/routing_invariants.rs` enforces the planning rules below.

- if a planning judgment depends on observations, the current topology must be passed explicitly to the planner method that makes that judgment
- `BackendRouteRef` stays opaque at the shared boundary, but in pathway it is a self-contained plan token rather than a cache key
- pathway may memoize derived candidates internally, but cache hits and misses must produce the same result for the same topology
- admitted routes carry that opaque backend ref forward so `materialize_route` can decode the selected pathway plan without searching planner cache state
- materialization still revalidates that decoded plan against the latest observed topology, the shared topology epoch, and the plan validity window before issuing a proof

Pathway route ids are path identities. The stable route id is derived from source, destination, route class, and concrete segment path. Epoch stays in the plan token and proof instead of becoming part of the stable route identity. Pathway-private plan tokens, route-identity bytes, ordering keys, and runtime checkpoints all use the same versioned canonical binary encoding policy so replay, hashing, and checkpoint recovery stay aligned.

Planner cache state is advisory only. `candidate_routes` populates `candidate_cache` for reuse by `check_candidate` and `admit_route`, but cache misses re-derive the same candidate and admission result from the backend token plus explicit topology. Pathway does not let planner decisions depend on hidden mutable cache state.

The simulator consumes Pathway planning through the shared `RoutingEnginePlannerModel` contract. `jacquard-pathway` owns the planner seed and the seed-to-private-state translation, so simulator fixtures describe route objectives and expected outcomes rather than pathway-private search or repair internals.

## Engine Middleware

`RoutingEngine::engine_tick` is the engine-wide progress hook for pathway. The router or host supplies a shared `RoutingTickContext`, and pathway returns a `RoutingTickOutcome` that reports whether the tick changed pathway-private state. Inside `jacquard-pathway`, this hook is the engine-internal middleware loop.

```text
topology observation
  -> refresh pathway-private estimates
  -> summarize transport ingress
  -> update bounded control state
  -> clear stale candidate cache
  -> checkpoint current pathway runtime state
```

Each tick ingests the latest topology observation, refreshes the pathway-private estimate caches, summarizes the latest bounded transport observations, and folds that evidence into a bounded control state. That control state carries transport stability, repair pressure, and anti-entropy pressure with deterministic decay. Pathway uses it to tighten route health, escalate repair posture under sustained pressure, make `AntiEntropyRequired` consume real anti-entropy debt rather than acting as pure bookkeeping, and drive the cooperative route-export, neighbor-advertisement, and anti-entropy protocol exchanges described below. The hook then evicts stale candidate entries and writes the scoped topology-epoch checkpoint.

Discovery enters the pathway engine through the shared world picture: nodes, links, environment, and service advertisements are already merged into `Observation<Configuration>` before the engine plans. Pathway then derives its route-export, neighbor-advertisement, and anti-entropy choreography payloads from those shared observations plus active shared route objects rather than maintaining a second hidden advertisement schema.

## Internal Choreography Surface

Pathway carries a private Telltale choreography layer inside `jacquard-pathway`. This does not change the shared Jacquard routing contract. Router-facing planning, admission, materialization, maintenance, and tick flow use the shared `RoutingEngine` trait plus the pathway-owned `PathwayRoutingEngine` extension seam.

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
  - route export exchange
  - neighbor advertisement exchange
  - anti-entropy exchange

Pathway protocols live inline in the pathway crate as `tell!` definitions. That keeps the generated protocol/session code adjacent to the Rust host logic that enters those protocols and avoids a second file-based choreography source of truth.

Pathway also keeps one pathway-owned choreography interpreter surface above the shared runtime traits. That interpreter maps protocol-local requests onto the existing Jacquard boundaries:

- `TransportSenderEffects` for endpoint-addressed payload sends
- `RetentionStore` for deferred-delivery payload storage
- `RouteEventLogEffects` for replay-visible route events
- router-owned checkpoint orchestration for persisted pathway-private state

Host-owned ingress draining stops outside pathway itself. The router or bridge drains `TransportDriver`, converts raw ingress into shared observations, and feeds those observations into pathway through explicit router ingestion before a synchronous round. Inside pathway, those observations enter a bounded pending-ingress queue. A round consumes that queue deterministically and records a host-facing pathway round-progress snapshot that reports whether the round advanced state, waited quietly, or dropped excess ingress fail-closed.

This is intentionally still pathway-private. The router should only observe shared route objects, shared tick context, shared round outcome, and shared checkpoint orchestration. It should not depend on pathway-private choreography payloads or generated effect interfaces.

The generated or protocol-local Telltale effect interfaces are not the shared Jacquard effect contract. They stay inside `jacquard-pathway` as implementation-facing protocol surfaces. Concrete host adapters still implement the shared traits from `jacquard-traits`, and the pathway choreography interpreter translates protocol-local requests onto those stable cross-engine traits instead of replacing them.

At runtime, pathway entry points cross one private guest-runtime layer before touching transport send capability, retention, or route-event logging directly. `forward_payload`, materialization-side activation, maintenance-side repair and handoff, retained-payload replay, round-side ingress recording, route export, neighbor advertisement, and anti-entropy exchange all enter that pathway-local choreography boundary first.

The guest runtime resolves stable inline protocol metadata for the protocol being entered, fails closed if that metadata is unavailable, and then records small protocol checkpoints keyed by protocol kind plus route or tick session. Recovery does not depend on hidden in-memory sequencing state. Telltale session futures remain confined to choreography modules. The engine and runtime layer itself stays synchronous and driver-free.

## Runtime and Repair

Materialization stores a pathway-private active-route object under the router-owned canonical identity. That object contains the explicit `PathwayPath`, optional `CommitteeSelection`, and a deterministic ordering key plus four route-private substates:

- `PathwayForwardingState` for current owner, owner-relative next-hop cursor, in-flight frames, and last ack
- `PathwayRepairState` for bounded repair budget and last repair tick
- `PathwayHandoffState` for the last handoff receipt and handoff tick
- `PathwayRouteAntiEntropyState` for partition mode, retained objects, and last anti-entropy refresh

Canonical route identity, admission, and lease ownership remain outside this pathway-private runtime object.

Pathway decodes the admitted opaque backend ref during materialization instead of recovering route shape from planner cache state. Token decode alone is not enough. The runtime re-derives the candidate against the latest observed topology and fails closed if the plan epoch, handle epoch, witness epoch, latest topology epoch, or plan validity window do not still agree. Materialization itself fails closed until the engine has observed topology through `engine_tick`, so pathway does not synthesize a pre-observation route health or an empty-world fallback.

### Route Health

Route health is derived from the active route's remaining suffix rather than from engine-global topology presence. Pathway validates the current owner-relative suffix against the latest observed topology and folds first-hop transport observations into that route-local view when available. It publishes `ReachabilityState::Unknown` when it lacks route-local validation data rather than pretending the route is generically reachable or unreachable.

The runtime route-health calculation currently combines three signal groups:

| Signal group | Inputs |
| --- | --- |
| First-hop transport summary | remote-link stability score, remote-link congestion penalty |
| Remaining-suffix topology view | delivery confidence, symmetry, loss-derived congestion penalty |
| Pathway control state | transport stability score, anti-entropy pressure |

As in planning, `median_rtt_ms` is not currently part of the published route-health calculation.

### Lifecycle and Maintenance

Lifecycle sequencing is explicit and fail-closed. Pathway validates first, builds the next active-route state off to the side, persists the checkpoint, records the route event, and only then publishes the in-memory runtime mutation. If checkpoint or route-event logging fails, the new state is not committed.

Protocol checkpoints follow the same fail-closed rule. Pathway writes or updates the protocol checkpoint through the choreography guest runtime before treating that step as complete, and rollback paths remove route-scoped protocol checkpoints when materialization or teardown does not commit. Those checkpoints carry protocol metadata derived from the live inline protocol modules themselves, including the protocol name, declared roles, and source-path identity, so replay and recovery stay aligned with the live generated protocol surface rather than with a handwritten local label only.

Maintenance is expressed through the shared `RouteMaintenanceResult` surface. Repair means a bounded local suffix-repair algorithm over the latest observed topology. `LinkDegraded` and `EpochAdvanced` attempt to recompute the remaining suffix from the current owner to the final destination, consume one repair step on success, and escalate to typed replacement when no bounded patch is available or the repair budget is exhausted.

The maintenance path follows the same reducer split as the other engines. Pathway first normalizes one maintenance input from the active route, latest topology epoch, trigger, and handoff receipt. A pure transition planner then returns the next route or runtime projection plus ordered effect requests such as repair exchange, retained-payload flush, handoff exchange, and anti-entropy pressure consumption. The runtime wrapper executes those requested effects fail-closed and only checkpoints or publishes the projected route state after every requested effect succeeds.

`CapacityExceeded` returns `ReplacementRequired` without flipping partition mode, since it indicates replacement pressure rather than partition evidence. `PartitionDetected` enters partition mode and reports the current retained-object count through `HoldFallback`. `PolicyShift` performs handoff and `AntiEntropyRequired` flushes retained payloads to recover. Pathway exposes one current commitment per route, so repair, handoff, and deferred-delivery posture stay inside the route runtime state rather than becoming separate concurrent commitments.

### Forwarding

A handoff advances the owner-relative cursor to the remaining suffix under the next owner. Forwarding then succeeds only for the current owner of that suffix.

Old owners fail closed with `StaleOwner`, exhausted owner-relative paths fail with `Invalidated`, and malformed admitted plan tokens fail the same way during materialization. Each case maps to a typed `RouteMaintenanceResult` value rather than a side-channel mutation.

## Optional Committee Coordination

Pathway can attach a swappable `CommitteeSelector`, with `DeterministicCommitteeSelector` as the optional in-tree implementation. The selector returns `Option<CommitteeSelection>` rather than assuming a committee always exists. The in-tree selector reads the pathway neighborhood estimate for committee gating and the pathway peer estimate for ranking. Route ordering and local coordination stay on the same topology-model interpretation.

Committee eligibility is stricter than forwarding value alone. A member must be route-capable for pathway, must present a usable shared service surface, and may be disqualified by bounded behavior-history penalties before ranking happens. Selection is diversity-constrained: controller diversity is mandatory, and discovery-scope diversity is enforced when alternatives exist. If discovery-scope diversity would suppress the minimum viable committee, pathway falls back to controller-only diversity rather than silently disabling coordination.

`None` means no committee applies, and a selector error is not silently downgraded to `None`. Pathway surfaces a selector error as a typed inadmissible candidate using `BackendUnavailable`. The result is advisory coordination evidence only and does not replace canonical route admission, route witness, or route lease ownership.

## Retention and Storage

The pathway engine uses the shared `RetentionStore` boundary for deferred-delivery payloads. While a route is in partition mode, `forward_payload` buffers payloads into the retention store instead of sending them immediately. Pathway then flushes those retained payloads on recovery or before handoff when a next hop becomes available. The typed partition fallback surface is `RouteMaintenanceOutcome::HoldFallback`, which carries the retained-object count visible on the route at the time the fallback was entered.

Retained payload identity flows through the shared `Hashing` boundary. Route and runtime checkpoints flow through the shared storage and route-event-log effects. Storage keys and runtime checkpoints are scoped by the local engine identity so multiple local pathway engines can share one backend without overwriting one another.

Pathway supports a scoped checkpoint round-trip for pathway-private route runtime and the latest topology epoch. The route checkpoint is intentionally narrower than the full active-route object: it stores the mutable forwarding, repair, handoff, anti-entropy, and current-epoch substate only. Path shape, committee selection, route cost, route identity, and lifecycle event are rebuilt from the router-owned `MaterializedRoute` record and the self-contained backend plan token during restore.

The choreography layer adds a second scoped recovery surface. Protocol checkpoints are keyed by protocol kind plus route session or tick session and round-trip through the same storage boundary.

Route recovery uses the reduced route checkpoint plus the router-owned materialized record. Protocol recovery uses the protocol checkpoint catalog. Neither requires ambient hidden state outside engine-owned checkpoint storage and router-owned canonical route truth.

## Swappable Trait Surface

The pathway engine exposes its narrow read-only pathway seams as two traits in `jacquard-pathway`: `PathwayTopologyModel` and `PathwayRoutingEngine`. Substituting either one replaces a pathway subcomponent without forking the engine, and the coupling is pathway-specific rather than leaking into `jacquard-traits`. `RetentionStore` remains a shared runtime boundary on the neutral effect surface. For host runtime effects beyond these seams, the engine uses the shared `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, and `Hashing` surfaces from `jacquard-traits`.

### Topology Model

```rust
pub trait PathwayTopologyModel {
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

`PathwayTopologyModel` is read-only. The associated estimate types are the important boundary. If a pathway implementation wants novelty scores, reach estimates, bridge heuristics, or neighborhood flow signals, those stay pathway-owned behind `PathwayTopologyModel`. They are not promoted into `jacquard-core` as shared `Node`, `Link`, or `Environment` schema.

### Engine Binding

```rust
pub trait PathwayRoutingEngine: RoutingEngine {
    type TopologyModel: PathwayTopologyModel;
    type Retention: RetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel;

    fn retention_store(&self) -> &Self::Retention;
}
```

`PathwayRoutingEngine` binds one concrete topology model and one retention store to a pathway engine instance. It stays narrow on purpose: hosts can inspect the read-only pathway subcomponents without gaining a mutation hook into pathway-private runtime state. Transport send capability and transport ingress ownership are split cleanly: pathway consumes the shared `TransportSenderEffects` capability, while the host/router owns ingress supervision and delivers explicit observations before each round.

### Shared Retention Boundary

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

`RetentionStore` is the storage boundary for opaque deferred-delivery payloads during partitions. It stays intentionally narrow so platform-specific persistence can substitute without forcing the rest of the pathway engine to know about it. It is not treated as a pathway-specific trait surface.
