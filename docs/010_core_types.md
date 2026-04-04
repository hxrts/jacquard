# Core Types

This page focuses on the types that explain how Contour is put together. It does not try to enumerate every type in `core`. The goal is to show the system components through the objects that move between them.

See [Introduction](001_introduction.md) for repository scope. See [Time Model](020_time.md) for the time and ordering rules that shape many of these types. See [Routing Architecture](030_routing_architecture.md) for how crates and runtime layers use them.

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

`TopologySnapshot` is the shared local view of the neighborhood. `TopologyNodeObservation` and `TopologyLinkObservation` carry node and link facts. `NodeRoutingObservation`, `PeerRoutingObservation`, and `NeighborhoodObservation` give the router explicit surfaces for self-state, peer estimates, and aggregate local conditions.

## Route Lifecycle

`RoutingObjective` states what one operation wants from routing. It carries destination, requested service family, privacy target, privacy floor, connectivity target, and bounded latency and fallback requirements.

`RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, and `RouteWitness` describe the selection and admission path. `RouteCandidate` is observational. `RouteAdmissionCheck` says whether a family can justify a route under a stated profile. `RouteWitness` is the proof-bearing record of what the admitted route actually delivers.

`InstalledRoute` is the object to keep in mind when reading the rest of the system. It ties together identity, admission, ownership, health, and progress. It is the point where a family-specific plan becomes canonical router state.

`RouteHandle` is the strong handle issued at materialization time. `RouteMaterializationProof` binds that handle to the authoritative witness that justified installation. `RouteCommitment` tracks unresolved or recently resolved obligations such as setup, repair, or replacement work.

## Runtime Boundary

The most important traits live in the `traits` crate, but they are easiest to understand through the shared model they carry. `RouteFamilyExtension` is the family boundary. `TopLevelRouter`, `RoutingControlPlane`, and `RoutingDataPlane` are the router-facing orchestration surfaces.

```rust
pub trait RouteFamilyExtension {
    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observed<TopologySnapshot>,
    ) -> Vec<RouteCandidate>;

    fn admit_route(
        &mut self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;

    fn install_route(
        &mut self,
        admission: RouteAdmission,
    ) -> Result<InstalledRoute, RouteError>;
}
```

This trait fragment shows the main semantic path. A family starts from observations. It turns those into candidates. It turns one candidate into an admitted route with a witness. It then materializes that route into canonical installed state. That sequence explains why the shared model puts so much weight on `Observed<T>`, `RouteAdmission`, and `InstalledRoute`.

The runtime-effect traits keep platform concerns outside the pure routing model. `TimeEffects`, `OrderEffects`, `HashEffects`, `StorageEffects`, `AuditEffects`, and `TransportEffects` provide the narrow abstract runtime that mesh, router, and simulator code can depend on.

The key point is that `core` types describe the stable semantic objects, and `traits` describe how those objects move. That is the contract surface other crates and external families build on.
