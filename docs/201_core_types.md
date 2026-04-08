# Core Types

This page focuses on the core primitives that other routing objects build on. See [Crate Architecture](999_crate_architecture.md) for the internal directory layout of `core`.

## Identity, Observation, And Fact

`NodeId` identifies one running Jacquard client. `ControllerId` identifies the cryptographic actor that authenticates for that node. `NodeBinding` makes that relationship explicit instead of assuming one node identity is enough for every deployment.

Jacquard now uses an explicit epistemic ladder. `Observation<T>` is raw local input or a received report with provenance attached. `Estimate<T>` is a belief update derived from one or more observations. `Fact<T>` is stronger: it is the value the system is willing to treat as established routing truth. This split matters because a recent topology sighting, a scored route candidate, and a published route witness are different kinds of claim.

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

`IdentityAssuranceClass` is a second identity-facing qualifier. It says how strongly a node identity is grounded for routing-control decisions. That keeps "who claims to exist" separate from "how much committee or admission weight that identity should receive".

## Time And Qualifiers

`Tick`, `DurationMs`, `OrderStamp`, `RouteEpoch`, and `ByteCount` are the core scalar units. They keep local time, bounded duration, deterministic ordering, topology versioning, and byte quantities distinct at the type level. `TimeWindow` and `TimeoutPolicy` are the first compound objects built on those primitives. See [Time Model](202_time.md) for the full time-domain rules and the validated `TimeWindow::new` constructor.

`Belief<T>` and `Limit<T>` are the two main qualifier types. `Belief<T>` distinguishes `Absent` from `Estimated(Estimate<T>)`, so the model can say both whether an estimate exists and how strong it is. `Limit<T>` says whether a budget is bounded or explicitly unlimited.

## World Schema

`Configuration` is the shared graph-shaped world object the router reasons about. It wires together `Node`, `Link`, and `Environment`. World extensions emit `Observation<ObservedValue>` items that contribute to that picture. See [Pipeline and World Observations](203_pipeline_observations.md) for the full schema and the observation surface.

Mesh-specific peer or neighborhood heuristics do not live here. Novelty scoring, bridge detection, reach estimation, and similar derived mesh signals stay behind the mesh trait boundary as engine-owned estimate types. `core` carries the world facts those heuristics are computed from, not the heuristics themselves.

## Route Lifecycle Objects

`RouteHandle`, `RouteLease`, `RouteMaterializationInput`, `RouteInstallation`, `RouteMaterializationProof`, and `RouteCommitment` are the main runtime coordination objects in `core`. The router allocates canonical route identity through `RouteHandle`, `RouteLease`, and `RouteMaterializationInput`. The engine returns `RouteInstallation` and `RouteMaterializationProof` to describe what it realized under that identity.

Live routes are split into router-owned `MaterializedRouteIdentity` and engine-mutable `RouteRuntimeState`, composed as `MaterializedRoute`. Canonical route state does not come directly from a transport callback or raw health observation. Activation enforces the structural invariants. The admission decision must be admissible, the realized protection must satisfy the objective protection floor, and lease validity must be checked explicitly before publication or maintenance continues.

See [Route Lifecycle](204_route_lifecycle.md) for the full lifecycle flow from objective through teardown.

## Coordination And Layering

`CommitteeSelection` is the main shared coordination object. It carries a selected member set, role declarations, lease window, evidence basis, claim strength, and identity-assurance posture. `core` exposes only the coordination result shape. It does not define one universal committee-formation algorithm, require a leader, or encode engine-local scoring policy.

`SubstrateRequirements`, `SubstrateCandidate`, `SubstrateLease`, and `LayerParameters` are the shared layering objects. They exist so a host-level orchestrator can compose engines without teaching one engine about another's internals. `core` exposes the carrier contract shape, not the host policy that decides when one engine should migrate to another.

`DiscoveryScopeId` is separate from the routing concept of a neighborhood. It is only a service-scope identifier used in `ServiceScope::Discovery`. It does not name a routing authority set or an engine-local topology object.
