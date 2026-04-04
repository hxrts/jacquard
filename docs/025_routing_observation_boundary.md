# Routing Observation Boundary

This page defines the abstraction boundary around the local node, peer connections, and local environment. The goal is to expose the information a routing algorithm needs without leaking raw device internals or physical-world details into the routing core.

See [Core Types](010_core_types.md) for the main shared model objects. See [Routing Architecture](030_routing_architecture.md) for the layer boundary that consumes these types.

## Purpose

The routing core should not need to know battery chemistry, radio chipset details, GPS coordinates, or raw signal traces. It should only see routing-relevant surfaces such as budget, novelty, stability, confidence, and aggregate neighborhood conditions.

This boundary keeps the model portable across devices and transports. It also keeps experimentation focused on routing-visible signals instead of tying policy to one platform or one measurement stack.

The model is organized into four scopes. They are the local node, the link to a neighbor, the neighbor as a peer, and the local environment as an aggregate. Each scope answers a different routing question. The router should not collapse them into one device-shaped object.

Contour also separates direct observation from estimation. Observation collects current local and link-visible facts. Estimation derives routing-relevant peer and neighborhood summaries from those facts over time. Policy then consumes both the direct observations and the derived estimates.

## Local Node

The local node is represented through the same shared node shape that appears in topology snapshots. That shape carries identity, service, endpoint, trust, and routing-facing observation data. It is still not a full device model.

The important split is between intrinsics and current state. Intrinsics come from stable device or local-policy limits that the router should be allowed to see. Current state comes from what that node can still do right now.

```rust
pub struct TopologyNodeObservation {
    pub controller_id: ControllerId,
    pub services: Vec<ServiceDescriptor>,
    pub endpoints: Vec<LinkEndpoint>,
    pub routing_estimate: KnownValue<PeerRoutingEstimate>,
    pub trust_class: PeerTrustClass,
    pub last_seen_at_tick: Tick,
}

pub struct NodeRoutingIntrinsics {
    pub connection_count_max: KnownValue<u32>,
    pub neighbor_state_count_max: KnownValue<u32>,
    pub simultaneous_transfer_count_max: KnownValue<u32>,
    pub active_route_count_max: KnownValue<u32>,
    pub relay_work_budget_max: KnownValue<u32>,
    pub maintenance_work_budget_max: KnownValue<u32>,
    pub hold_item_count_max: KnownValue<u32>,
    pub hold_capacity_bytes_max: KnownValue<u64>,
}

pub struct PeerRoutingEstimate {
    pub relay_budget: KnownValue<NodeRelayBudget>,
    pub information_summary: KnownValue<InformationSetSummary>,
    pub novelty_estimate: KnownValue<PeerNoveltyEstimate>,
    pub reach_score: KnownValue<HealthScore>,
    pub underserved_trajectory_score: KnownValue<HealthScore>,
}

pub struct NodeRelayBudget {
    pub relay_work_budget: KnownValue<u32>,
    pub utilization_permille: RatioPermille,
    pub retention_horizon_ms: KnownValue<DurationMs>,
}

pub struct NodeRoutingObservation {
    pub intrinsics: NodeRoutingIntrinsics,
    pub relay_budget: NodeRelayBudget,
    pub available_connection_count: KnownValue<u32>,
    pub hold_capacity_available_bytes: KnownValue<u64>,
    pub information_summary: KnownValue<InformationSetSummary>,
}
```

This snippet shows the intended level of abstraction. The router sees controller binding, advertised services, link endpoints, trust posture, intrinsic limits, relay budget, information summary, novelty estimate, and peer reach signals. It does not see battery voltage, buffer implementation details, or operating-system power state directly.

`NodeRoutingIntrinsics` is where stable routing-facing limits belong. A node may only support a small number of concurrent connections. A node may only be able to track a bounded number of neighbors. A node may cap simultaneous transfers, active routes, repair work, retained items, and retained bytes for policy or transport reasons. These are device or local-policy constraints, but they are exposed in a form that the router can use without learning hardware details.

`NodeRoutingObservation` is the current-state companion to those intrinsics. `available_connection_count` says how much connection headroom remains now. `relay_budget` says how much forwarding work can still be accepted. `hold_capacity_available_bytes` says how much retained payload space is still available for deferred delivery.

These fields exist because routing decisions depend on future forwarding value, not only on current free space. A node with spare capacity but a short `retention_horizon_ms` is a weak custody target. A node with moderate capacity and a long retention horizon may be a better relay for deferred delivery.

## Peer And Connection

Peers and connections are split across observation and estimation. A peer is described by advertised identity and service facts plus a `PeerRoutingEstimate`. A connection is described by a `TopologyLinkObservation`.

```rust
pub struct PeerRoutingEstimate {
    pub relay_budget: KnownValue<NodeRelayBudget>,
    pub information_summary: KnownValue<InformationSetSummary>,
    pub novelty_estimate: KnownValue<PeerNoveltyEstimate>,
    pub reach_score: KnownValue<HealthScore>,
    pub underserved_trajectory_score: KnownValue<HealthScore>,
}

pub struct TopologyLinkObservation {
    pub endpoint: LinkEndpoint,
    pub state: LinkRuntimeState,
    pub median_rtt_ms: DurationMs,
    pub transfer_rate_bytes_per_sec: KnownValue<u32>,
    pub stability_horizon_ms: KnownValue<DurationMs>,
    pub loss_permille: RatioPermille,
    pub delivery_confidence_permille: KnownValue<RatioPermille>,
    pub symmetry_permille: KnownValue<RatioPermille>,
    pub last_seen_at_tick: Tick,
}
```

This boundary keeps the router focused on actionable signals. It can reason about novelty, reach, transfer rate, stability horizon, delivery confidence, and symmetry. It does not need raw RSSI traces, antenna geometry, or motion-sensor data to make a routing decision.

`novelty_estimate` is the routing-visible derivative of the peer information set. The router does not need the peer's full buffer contents as protocol truth. It needs an approximate answer to a narrower question. What does this peer have that I do not have, and what do I have that this peer lacks.

`reach_score` is a local proxy for whether a peer can move information into other parts of the network. It is intentionally abstract. It may be derived from recent contact diversity, message-origin diversity, or other local evidence. The routing core should consume the score, not the device-specific method used to compute it.

The connection surface stays similarly narrow. `transfer_rate_bytes_per_sec` answers whether a meaningful exchange fits inside the contact window. `stability_horizon_ms` answers how long the contact is likely to remain useful. `delivery_confidence_permille` and `symmetry_permille` answer whether the link can reliably support exchange in the expected direction.

## Environment

The local environment is observed first and then summarized through `NeighborhoodEstimate`. The direct observation carries density, churn, and contention. The estimate adds bridging value and underserved-flow scoring.

```rust
pub struct NeighborhoodObservation {
    pub reachable_neighbor_count: u32,
    pub churn_permille: RatioPermille,
    pub contention_permille: RatioPermille,
}

pub struct NeighborhoodEstimate {
    pub observation: NeighborhoodObservation,
    pub bridging_score: KnownValue<HealthScore>,
    pub underserved_flow_score: KnownValue<HealthScore>,
}
```

This aggregate view is where Contour captures conditions that are not properties of one peer. Density answers how selective the router can be. Churn answers whether the topology is stable enough to wait for better opportunities. Contention answers whether the medium can absorb more exchange now. Bridging value and underserved-flow scoring answer whether the local node sits between weakly connected regions or near one-sided information flow.

These environment signals are especially important in sparse and disrupted networks. A contact that looks mediocre in isolation may still be valuable if the neighborhood is sparse, churn is high, or the node appears to bridge otherwise disjoint information sets.

This separation is the core architectural point. Contour should expose routing-relevant observations without forcing the routing layer to understand batteries, radios, coordinates, or raw physical measurements. That abstraction boundary makes the model portable, deterministic, and open to experimentation across different transports and devices.
