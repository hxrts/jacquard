# Core Types

This page focuses on the core primitives that other routing objects build on. It does not try to enumerate every type in `core`. The goal is to show the stable building blocks that make the rest of the system readable.

See [Introduction](001_introduction.md) for repository scope. See [Time Model](020_time.md) for the time and ordering rules that shape many of these types. See [Routing Observation Boundary](025_routing_observation_boundary.md) for the routing-visible observation and estimation surfaces. See [Routing Architecture](030_routing_architecture.md) for how crates and runtime layers use them.

## Identity And Facts

`NodeId` identifies one running Contour client. `ControllerId` identifies the cryptographic actor that authenticates for that node. `NodeBinding` makes that relationship explicit instead of assuming one node identity is enough for every deployment.

`Observed<T>` and `Authoritative<T>` separate local observations from canonical published truth. `RoutingFact<T>` carries provenance for an observed value. This split matters because a recent topology sighting and an admitted route witness are different kinds of claim.

```rust
pub struct NodeBinding {
    pub node_id: NodeId,
    pub controller_id: ControllerId,
    pub binding_epoch: RouteEpoch,
    pub proof: NodeBindingProof,
}

pub struct RoutingFact<T> {
    pub value: T,
    pub evidence_class: RoutingEvidenceClass,
    pub trust_class: PeerTrustClass,
    pub observed_at_tick: Tick,
}

pub struct Observed<T> {
    pub fact: RoutingFact<T>,
}

pub struct Authoritative<T> {
    pub value: T,
    pub published_at_tick: Tick,
}
```

This group of types shows two important boundaries. `NodeBinding` says who controls a node. `Observed<T>` and `Authoritative<T>` say what kind of claim is being carried. Together they prevent the model from collapsing identity, evidence, and publication into one opaque record.

## Time And Bounds

`Tick`, `DurationMs`, `OrderStamp`, and `RouteEpoch` are the time and ordering primitives. They keep local time, bounded duration, deterministic ordering, and topology versioning distinct. `TimeWindow` and `TimeoutPolicy` are the first compound objects built on those primitives.

`KnownValue<T>` and `Limit<T>` are the two main qualifier types. `KnownValue<T>` says whether a measured or inferred quantity is currently known. `Limit<T>` says whether a budget is bounded or explicitly unlimited. Together they keep uncertainty and resource policy explicit in the model.

## Shared Surfaces

`TopologySnapshot` is the shared local view of the neighborhood. `TopologyNodeObservation` and `TopologyLinkObservation` carry node and link facts. `NodeRoutingIntrinsics` and `NodeRoutingObservation` describe stable limits and current local state. `PeerRoutingEstimate` and `NeighborhoodEstimate` describe the derived routing summaries that sit between raw observation and policy. `NodeRoutingIntrinsics` is where hard limits like maximum connections, neighbor-state capacity, active-route capacity, maintenance budget, and hold ceilings belong.

`RouteHandle`, `RouteMaterializationProof`, and `RouteCommitment` are the main runtime coordination objects in `core`. They are worth recognizing early because many later types point at them. They give the system strong route identity, proof-bearing materialization, and explicit outstanding work.

## What Is Not Here

This page stops at the shared building blocks. It does not try to describe how a route family turns observations into candidates or how the router decides between families. Those behaviors belong in [Routing Architecture](030_routing_architecture.md).

The key point is that `core` defines the language of the system. Other crates use that language to express decision inputs, route-family behavior, router orchestration, and simulation.
