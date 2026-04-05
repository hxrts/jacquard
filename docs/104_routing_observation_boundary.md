# Routing Observation Boundary

This page defines the abstraction boundary around the local node, peer connections, and local environment. The goal is to expose the information a routing algorithm needs without leaking raw device internals or physical-world details into the routing core.

## Purpose

The routing core sees budget, novelty, stability, confidence, and aggregate neighborhood conditions. It does not see battery chemistry, radio chipset details, GPS coordinates, or raw signal traces. This keeps the model portable across devices and transports.

Uncertain quantities are modeled as `Belief<T>`: either `Absent` or `Estimated(Estimate<T>)` with an explicit `confidence_permille`. Observations keep source and authentication separate. An `Observation<T>` may be local or remote, and its origin may be controlled, authenticated, or unauthenticated.

The model has four scopes: local node, link, peer, and environment. Each answers a different routing question. `world` defines the abstract objects. `observation` wraps them with provenance. `estimation` derives routing summaries. `policy` and `action` sit on top.

## Local Node

`Node` is split into `NodeProfile` (stable limits) and `NodeState` (current conditions). `Observation<Node>` is one local or remote claim about that node.

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

`NodeProfile` exposes device and local-policy constraints in a form the router can use without learning hardware details. `NodeState` says how much connection headroom, forwarding capacity, and retention space remain now. Routing decisions depend on future forwarding value, not only current free space. A node with spare capacity but a short `retention_horizon_ms` is a weak custody target.

## Peer And Connection

A peer is a `Node` plus a `PeerRoutingEstimate`. A connection is a `Link` split into `LinkProfile` (stable endpoint) and `LinkState` (changing quality).

```rust
pub struct Link {
    pub profile: LinkProfile,
    pub state: LinkState,
}

pub struct LinkProfile {
    pub endpoint: LinkEndpoint,
}

pub struct PeerRoutingEstimate {
    pub relay_budget: Belief<NodeRelayBudget>,
    pub information_summary: Belief<InformationSetSummary>,
    pub novelty_estimate: Belief<PeerNoveltyEstimate>,
    pub reach_score: Belief<HealthScore>,
    pub underserved_trajectory_score: Belief<HealthScore>,
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

`novelty_estimate` approximates what a peer has that this node lacks, and vice versa. `reach_score` is a local proxy for whether a peer can move information into other parts of the network. Both are intentionally abstract so the routing core consumes the score, not the method used to compute it.

`transfer_rate_bytes_per_sec` answers whether a meaningful exchange fits inside the contact window. `stability_horizon_ms` answers how long the contact is likely to remain useful. `delivery_confidence_permille` and `symmetry_permille` answer whether the link supports exchange in the expected direction.

## Environment

`Environment` carries family-neutral aggregate conditions: density, churn, and contention. `ConfigurationEstimate` adds bridging value and underserved-flow scoring on top.

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

These signals matter most in sparse and disrupted networks. A contact that looks mediocre in isolation may still be valuable if the neighborhood is sparse, churn is high, or the node bridges otherwise disjoint information sets.

`Environment` should not include family-specific concerns. Richer geometry, spatial embeddings, or transport-specific structure should extend `Configuration` in the family layer rather than inflating the base environment type.
