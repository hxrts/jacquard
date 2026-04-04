# Core Types

This page focuses on the core primitives that other routing objects build on. It does not try to enumerate every type in `core`. The goal is to show the stable building blocks that make the rest of the system readable.

See [Introduction](001_introduction.md) for repository scope. See [Time Model](020_time.md) for the time and ordering rules that shape many of these types. See [Routing Observation Boundary](025_routing_observation_boundary.md) for the routing-visible observation and estimation surfaces. See [Routing Architecture](030_routing_architecture.md) for how crates and runtime layers use them.

The `core` crate now groups these types into three internal areas. `base/` holds primitives such as identity, time, qualifiers, constants, errors, and fact wrappers. `model/` holds the world-to-action pipeline. `routing/` holds route lifecycle and runtime coordination objects.

## Identity, Observation, And Fact

`NodeId` identifies one running Contour client. `ControllerId` identifies the cryptographic actor that authenticates for that node. `NodeBinding` makes that relationship explicit instead of assuming one node identity is enough for every deployment.

Contour now uses an explicit epistemic ladder. `Observation<T>` is raw local input or a received report with provenance attached. `Estimate<T>` is a belief update derived from one or more observations. `Fact<T>` is stronger: it is the value the system is willing to treat as established routing truth. This split matters because a recent topology sighting, a scored route candidate, and a published route witness are different kinds of claim.

```rust
pub struct NodeBinding {
    pub node_id: NodeId,
    pub controller_id: ControllerId,
    pub binding_epoch: RouteEpoch,
    pub proof: NodeBindingProof,
}

pub struct Observation<T> {
    pub value: T,
    pub source_class: FactSourceClass,
    pub evidence_class: RoutingEvidenceClass,
    pub origin_authentication: OriginAuthenticationClass,
    pub observed_at_tick: Tick,
}

pub struct Estimate<T> {
    pub value: T,
    pub confidence_permille: RatioPermille,
    pub updated_at_tick: Tick,
}

pub struct Fact<T> {
    pub value: T,
    pub basis: FactBasis,
    pub established_at_tick: Tick,
}
```

This group of types shows two important boundaries. `NodeBinding` says who controls a node. `Observation<T>`, `Estimate<T>`, and `Fact<T>` say what kind of claim is being carried. Together they prevent the model from collapsing identity, evidence, inference, and publication into one opaque record.

`FactSourceClass` and `OriginAuthenticationClass` are intentionally separate. One says whether the fact is local or remote. The other says whether the origin is controlled, authenticated, or unauthenticated. That keeps provenance and authentication from collapsing into one mixed enum.

## Time And Bounds

`Tick`, `DurationMs`, `OrderStamp`, `RouteEpoch`, and `ByteCount` are the core scalar units. They keep local time, bounded duration, deterministic ordering, topology versioning, and byte quantities distinct. `TimeWindow` and `TimeoutPolicy` are the first compound objects built on those primitives.

`Belief<T>` and `Limit<T>` are the two main qualifier types. `Belief<T>` is the Bayesian-flavored wrapper for optional estimate state. It distinguishes `Absent` from `Estimated(Estimate<T>)`, so the model can say both whether an estimate exists and how strong it is. `Limit<T>` says whether a budget is bounded or explicitly unlimited. Together they keep uncertainty and resource policy explicit in the model.

## Shared Surfaces

`Configuration` is the shared world object the router reasons about. It is a wired-together set of `Node` and `Link` objects plus one `Environment`. `world` owns those instantiated world objects directly. `Observation<T>` wraps those objects when they are locally seen or remotely reported. `PeerRoutingEstimate`, `ConfigurationEstimate`, and `RouteEstimate` describe the derived routing summaries that sit between raw observation and policy. `AdaptiveRoutingProfile` is the main shared action object produced by policy.

`RouteHandle`, `RouteMaterializationProof`, and `RouteCommitment` are the main runtime coordination objects in `core`. They are worth recognizing early because many later types point at them. They give the system strong route identity, proof-bearing materialization, and explicit outstanding work.

## What Is Not Here

This page stops at the shared building blocks. It does not try to describe how a route family turns observations into candidates or how the router decides between families. Those behaviors belong in [Routing Architecture](030_routing_architecture.md).

The key point is that `core` defines the language of the system. Other crates use that language to express decision inputs, route-family behavior, router orchestration, and simulation.
