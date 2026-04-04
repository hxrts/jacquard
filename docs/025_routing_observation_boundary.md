# Routing Observation Boundary

This page defines the abstraction boundary around the local node, peer connections, and local environment. The goal is to expose the information a routing algorithm needs without leaking raw device internals or physical-world details into the routing core.

See [Core Types](010_core_types.md) for the main shared model objects. See [Routing Architecture](030_routing_architecture.md) for the layer boundary that consumes these types.

## Purpose

The routing core should not need to know battery chemistry, radio chipset details, GPS coordinates, or raw signal traces. It should only see routing-relevant surfaces such as budget, novelty, stability, confidence, and aggregate neighborhood conditions.

This boundary keeps the model portable across devices and transports. It also keeps experimentation focused on routing-visible signals instead of tying policy to one platform or one measurement stack.

## Local Node

The local node is represented as a routing-facing observation, not as a full device model. It exposes how much relay work the node can still absorb, how much hold capacity it is willing to commit, and what information-summary state it currently carries.

```rust
pub struct NodeRelayBudget {
    pub relay_work_budget: KnownValue<u32>,
    pub utilization_permille: RatioPermille,
    pub retention_horizon_ms: KnownValue<DurationMs>,
}

pub struct NodeRoutingObservation {
    pub relay_budget: NodeRelayBudget,
    pub hold_capacity_bytes: KnownValue<u64>,
    pub information_summary: KnownValue<InformationSetSummary>,
}
```

This snippet shows the intended level of abstraction. The router sees remaining relay budget, current utilization, retention horizon, hold capacity, and information-summary state. It does not see battery voltage, buffer implementation details, or operating-system power state directly.

## Peer And Connection

Peers and connections are also modeled through routing-visible estimates. A peer is described by advertised identity and service facts plus a `PeerRoutingObservation`. A connection is described by a `TopologyLinkObservation`.

```rust
pub struct PeerRoutingObservation {
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

## Environment

The local environment is modeled as an aggregate neighborhood view. It exposes density, churn, contention, bridging value, and underserved-flow scoring through `NeighborhoodObservation`.

This keeps the control plane from mixing local device state with neighborhood-wide posture. Each scope stays distinct. That is the main abstraction boundary that lets Contour evolve routing policy without hard-coding one device model or one physical measurement model.
