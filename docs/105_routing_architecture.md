# Routing Architecture

Contour is organized around a small stable stack. `core` owns shared model types. `traits` owns the abstract routing and runtime-effect boundaries. Later crates implement mesh, router orchestration, transport adapters, and simulation on top of those two layers.

See [Introduction](101_introduction.md) for repository scope. See [Core Types](102_core_types.md) for the semantic objects this architecture moves. See [Time Model](103_time.md) for the deterministic time and ordering rules that constrain the whole stack. See [Routing Observation Boundary](104_routing_observation_boundary.md) for the routing-visible node, peer, link, and environment surfaces plus the estimation layer that the architecture consumes.

Inside `core`, the files are grouped to match this shape. `base/` holds cross-cutting primitives and fact wrappers. `model/` holds the world to action pipeline. `routing/` holds route lifecycle and runtime objects. Small transport and content files stay at the crate root for now.

## Pipeline

Contour’s shared model is organized as a pipeline:

```text
world
  -> observation
  -> estimation
  -> policy
  -> action
```

`world` defines the abstract objects and configuration the router reasons about. `observation` wraps instantiated world objects with provenance. `estimation` derives routing-relevant beliefs from those observations. `policy` computes what should be done. `action` records the selected routing action, such as the current `AdaptiveRoutingProfile`.

## Planes

The routing contract separates control-plane work from data-plane work. The control plane owns candidate gathering, admission, installation, commitments, maintenance, and anti-entropy. The data plane forwards payloads over already admitted route state.

This split prevents forwarding code from inventing canonical route truth. Data-plane observations may report health or failures, but the control plane decides whether that changes the installed route.

The control plane is also where the pipeline is assembled into decisions. The router consumes node, connection, and environment primitives from `world`, then layers observations, estimates, policy inputs, and policy outputs on top. It does not consume raw device or physical-world details directly.

## Decision Path

The routing decision path starts from `RoutingObjective` and `Observation<Configuration>`. A route family turns those into `RouteCandidate` values. Each candidate carries an `Estimate<RouteEstimate>`, not a fact or published witness. The family then checks one candidate, admits it under a stated profile, and materializes it into `InstalledRoute`.

```rust
pub trait RouteFamily {
    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
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

This trait fragment shows the main route-building sequence. The important point is that route construction starts from shared observations, becomes inferential during candidate production, becomes proof-bearing at admission, and becomes canonical only at installation.

## Family Boundary

`RouteFamily` is the family boundary. A family produces observational candidates, checks admission, admits a route, installs it, publishes commitments, and handles family-local maintenance. Contour implements `Mesh` in-tree. Other families can integrate through the same boundary.

The top-level router stays family-neutral. It compares candidates, enforces fallback rules, tracks installed routes, and coordinates maintenance. Family-private planning and runtime state stay behind the family boundary.

## Runtime Boundary

The routing core does not call platform APIs directly. Hashing, storage, audit emission, transport ingress, time, and ordering all cross explicit runtime-effect traits. That is how native execution, tests, and simulation share one semantic model instead of drifting apart.

This architecture is also the main reason the simulator can reuse the same routing contract. The simulator does not need a second routing model. It drives the same shared objects and effect boundaries under a different runtime implementation.
