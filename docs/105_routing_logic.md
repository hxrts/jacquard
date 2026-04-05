# Routing Logic

This page describes how routing decisions are structured. It covers the pipeline from world state through policy to route materialization, the control/data plane split, and the decision path.

## Pipeline

Jacquard's shared model is organized as a pipeline:

```text
world
  -> observation
  -> estimation
  -> policy
  -> action
```

`world` defines the abstract objects and configuration the router reasons about. `observation` wraps instantiated world objects with provenance. `estimation` derives routing-relevant beliefs from those observations. `policy` computes what should be done. `action` records the selected routing action, such as the current `AdaptiveRoutingProfile`.

## Planes

The control plane owns candidate gathering, admission, materialization, commitments, maintenance, and anti-entropy. The data plane forwards payloads over already admitted route state. Data-plane observations may report health or failures, but the control plane decides whether that changes the active materialized route.

The link layer is a frame carrier. It reports reachability, MTU, loss, and timing. It does not own canonical ordering or traffic control. If a route family needs sequencing or causal behavior, that appears as a routing-level message-flow assumption rather than a transport guarantee. Keeping the transport surface simple avoids head-of-line stalls on unstable links and prevents baking one delivery policy into every route family.

## Decision Path

The routing decision path starts from `RoutingObjective` and `Observation<Configuration>`. A route planner turns those into `RouteCandidate` values. Each candidate carries an `Estimate<RouteEstimate>`, not a fact or published witness. The planner then checks one candidate, admits it under a stated profile, and a route family materializes it into `MaterializedRoute`.

```rust
pub trait RoutePlanner {
    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate>;

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;
}

pub trait RouteFamily: RoutePlanner {
    fn materialize_route(
        &mut self,
        admission: RouteAdmission,
    ) -> Result<MaterializedRoute, RouteError>;
}
```

This split shows the main route-building sequence. The important point is that route construction starts from shared observations, becomes inferential during candidate production, becomes proof-bearing at admission, and becomes canonical only at materialization. The planning side is deterministic and read-only with respect to canonical route state. Runtime mutation starts at `materialize_route`.

## Family Boundary

`RoutePlanner` is the deterministic planning boundary. `RouteFamily` is the effectful runtime boundary on top of it. A planner produces candidates, checks admission, and admits a route. A family runtime materializes it, publishes commitments, and handles maintenance. The top-level router stays family-neutral: it compares candidates, enforces fallback rules, tracks materialized routes, and coordinates maintenance.

The same pure/effectful split applies inside mesh. `MeshTopologyModel` is read-only. `MeshTransport` is the effectful frame carrier. `CustodyStore` is the effectful retention boundary. `MeshRouteFamily` ties those parts together without collapsing them into one blob.

## Runtime Boundary

The routing core does not call platform APIs directly. Hashing, storage, audit emission, transport ingress, time, and ordering all cross explicit runtime-effect traits. See [Crate Architecture](106_crate_architecture.md) for the full effect trait inventory and the simulator reuse argument.
