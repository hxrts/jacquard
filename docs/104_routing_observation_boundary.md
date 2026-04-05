# Routing Observation Boundary

This page defines the abstraction boundary around the local node, peer connections, and local environment. The goal is to expose the information a routing algorithm needs without leaking raw device internals or physical-world details into the routing core.

See [Core Types](102_core_types.md) for the main shared model objects. See [Routing Architecture](105_routing_architecture.md) for the layer boundary that consumes these types.

## Purpose

The routing core should not need to know battery chemistry, radio chipset details, GPS coordinates, or raw signal traces. It should only see routing-relevant surfaces such as budget, novelty, stability, confidence, and aggregate neighborhood conditions.

This boundary keeps the model portable across devices and transports. It also keeps experimentation focused on routing-visible signals instead of tying policy to one platform or one measurement stack.

When a quantity is not directly required as hard truth, Jacquard models it as a `Belief<T>`. That keeps the model honest: the router either has no usable estimate yet, or it has an `Estimate<T>` plus an explicit `confidence_permille`.
Observations also keep source and authentication separate. An `Observation<T>` may be local or remote, and its origin may be controlled, authenticated, or unauthenticated. The routing core should not collapse those into one trust bucket.

The model is organized into four scopes. They are the local node, the link to a neighbor, the neighbor as a peer, and the local environment as an aggregate. Each scope answers a different routing question. The router should not collapse them into one device-shaped object.

Jacquard also separates world definition from observation and estimation. `world` owns the abstract definition of a node, a connection, and the local environment, plus the `Configuration` type that wires them together. `observation` wraps those instantiated world objects with provenance. `estimation` derives routing-relevant peer and configuration summaries from those observations over time. `policy` consumes both the direct observations and the derived estimates. `action` records the selected routing posture that policy produced.

```text
world
  -> observation
  -> estimation
  -> policy
  -> action
```

## Local Node

The local node is represented through the same `Node` object that appears in a `Configuration`. That object is a world primitive from `world`. It carries identity, service, endpoint, stable limits, and current routing-visible node state. It is still not a full device model.

The important split is between the world object and the observation wrapper. `Node` is the instantiated node in the routing world. `Observation<Node>` is one local or remote claim about that node.

```rust
pub struct Node {
    pub controller_id: ControllerId,
    pub profile: NodeProfile,
    pub state: NodeState,
}

pub struct NodeProfile {
    pub services: Vec<ServiceDescriptor>,
    pub endpoints: Vec<LinkEndpoint>,
    pub connection_count_max: u32,
    pub neighbor_state_count_max: u32,
    pub simultaneous_transfer_count_max: u32,
    pub active_route_count_max: u32,
    pub relay_work_budget_max: u32,
    pub maintenance_work_budget_max: u32,
    pub hold_item_count_max: u32,
    pub hold_capacity_bytes_max: ByteCount,
}

pub struct NodeState {
    pub relay_budget: Belief<NodeRelayBudget>,
    pub available_connection_count: Belief<u32>,
    pub hold_capacity_available_bytes: Belief<ByteCount>,
    pub information_summary: Belief<InformationSetSummary>,
}

pub struct NodeRelayBudget {
    pub relay_work_budget: Belief<u32>,
    pub utilization_permille: RatioPermille,
    pub retention_horizon_ms: Belief<DurationMs>,
}

pub struct Observation<T> {
    pub value: T,
    pub source_class: FactSourceClass,
    pub evidence_class: RoutingEvidenceClass,
    pub origin_authentication: OriginAuthenticationClass,
    pub observed_at_tick: Tick,
}
```

This snippet shows the intended level of abstraction. The router sees one node object split into stable profile and current state. It does not see battery voltage, buffer implementation details, or operating-system power state directly.

`NodeProfile` carries the stable node-limit fields. A node may only support a small number of concurrent connections. A node may only be able to track a bounded number of neighbors. A node may cap simultaneous transfers, active routes, repair work, retained items, and retained bytes for policy or transport reasons. These are device or local-policy constraints, but they are exposed in a form that the router can use without learning hardware details.

`NodeState` carries the current-state fields that say how much connection headroom remains now, how much forwarding work can still be accepted, and how much retained payload space is still available for deferred delivery.

These fields exist because routing decisions depend on future forwarding value, not only on current free space. A node with spare capacity but a short `retention_horizon_ms` is a weak custody target. A node with moderate capacity and a long retention horizon may be a better relay for deferred delivery.

## Peer And Connection

Peers and connections are split across world definition, observation, and estimation. A peer is described by the `Node` world object plus a `PeerRoutingEstimate`. A connection is described by the `Link` world object, itself split into `LinkProfile` and `LinkState`, and whatever `Observation<Link>` claims the local node has accepted.

```rust
pub struct PeerRoutingEstimate {
    pub relay_budget: Belief<NodeRelayBudget>,
    pub information_summary: Belief<InformationSetSummary>,
    pub novelty_estimate: Belief<PeerNoveltyEstimate>,
    pub reach_score: Belief<HealthScore>,
    pub underserved_trajectory_score: Belief<HealthScore>,
}

pub struct Link {
    pub profile: LinkProfile,
    pub state: LinkState,
}

pub struct LinkProfile {
    pub endpoint: LinkEndpoint,
}

pub struct LinkState {
    pub state: LinkRuntimeState,
    pub median_rtt_ms: DurationMs,
    pub transfer_rate_bytes_per_sec: Belief<u32>,
    pub stability_horizon_ms: Belief<DurationMs>,
    pub loss_permille: RatioPermille,
    pub delivery_confidence_permille: Belief<RatioPermille>,
    pub symmetry_permille: Belief<RatioPermille>,
}
```

This boundary keeps the router focused on actionable signals. It can reason about novelty, reach, transfer rate, stability horizon, delivery confidence, and symmetry. It does not need raw RSSI traces, antenna geometry, or motion-sensor data to make a routing decision.

`novelty_estimate` is the routing-visible derivative of the peer information set. The router does not need the peer's full buffer contents as protocol truth. It needs an approximate answer to a narrower question. What does this peer have that I do not have, and what do I have that this peer lacks.

`reach_score` is a local proxy for whether a peer can move information into other parts of the network. It is intentionally abstract. It may be derived from recent contact diversity, message-origin diversity, or other local evidence. The routing core should consume the score, not the device-specific method used to compute it.

The connection surface stays similarly narrow. `LinkProfile` identifies the stable endpoint surface. `LinkState` carries the changing quality values. `transfer_rate_bytes_per_sec` answers whether a meaningful exchange fits inside the contact window. `stability_horizon_ms` answers how long the contact is likely to remain useful. `delivery_confidence_permille` and `symmetry_permille` answer whether the link can reliably support exchange in the expected direction.

## Environment

The local environment is also a primitive. `Environment` carries only the family-neutral aggregate conditions for the current configuration: density, churn, and contention. `ConfigurationEstimate` adds bridging value and underserved-flow scoring on top of that environment.

```rust
pub struct Environment {
    pub reachable_neighbor_count: u32,
    pub churn_permille: RatioPermille,
    pub contention_permille: RatioPermille,
}

pub struct ConfigurationEstimate {
    pub environment: Environment,
    pub bridging_score: Belief<HealthScore>,
    pub underserved_flow_score: Belief<HealthScore>,
}
```

This aggregate view is where Jacquard captures conditions that are not properties of one peer. Density answers how selective the router can be. Churn answers whether the topology is stable enough to wait for better opportunities. Contention answers whether the medium can absorb more exchange now. Bridging value and underserved-flow scoring answer whether the local node sits between weakly connected regions or near one-sided information flow.

These environment signals are especially important in sparse and disrupted networks. A contact that looks mediocre in isolation may still be valuable if the neighborhood is sparse, churn is high, or the node appears to bridge otherwise disjoint information sets.

The important boundary is that `Environment` is not the place to encode every family-specific concern. Topology already lives in `Configuration` through `nodes` and `links`. Richer geometry, spatial embeddings, or other transport- and family-specific structure should extend `Configuration` in the family layer, not be forced into the base `Environment` type.

This separation is the core architectural point. `world` defines the possible routing world. `Observation<T>` turns instantiated world objects into local or remote claims. `estimation` updates beliefs over the partial `Configuration` that the node currently sees. That abstraction boundary makes the model portable, deterministic, and open to experimentation across different transports and devices.
