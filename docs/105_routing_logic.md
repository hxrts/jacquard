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

If a family needs local coordination, that also lives in the control plane. A family may select a committee or witness set as part of planning, but those results are advisory inputs to canonical transitions. They are not canonical route truth by themselves.

The link layer is a frame carrier. It reports reachability, MTU, loss, and timing. It does not own canonical ordering or traffic control. If a route family needs sequencing or causal behavior, that appears as a routing-level message-flow assumption rather than a transport guarantee. Keeping the transport surface simple avoids head-of-line stalls on unstable links and prevents baking one delivery policy into every route family.

Layered composition follows the same rule. If one family uses another as a limited substrate, the layering decision belongs above both families in a host-owned coordinator. The lower layer exposes carrier capabilities and leases. The upper layer consumes those through a neutral contract. Neither family needs direct awareness of the other's private scoring or maintenance logic.

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

pub trait CommitteeSelector {
    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Result<CommitteeSelection, RouteError>;
}

pub trait SubstratePlanner {
    fn candidate_substrates(
        &self,
        requirements: &SubstrateRequirements,
        topology: &Observation<Configuration>,
    ) -> Vec<SubstrateCandidate>;
}

pub trait SubstrateRuntime {
    fn acquire_substrate(
        &mut self,
        candidate: SubstrateCandidate,
    ) -> Result<SubstrateLease, RouteError>;
}

pub trait LayeredRoutePlanner {
    fn candidate_routes_on_substrate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        substrate: &SubstrateLease,
        parameters: &LayerParameters,
    ) -> Vec<RouteCandidate>;
}

pub trait LayeredRouteFamily: RouteFamily + LayeredRoutePlanner {
    fn materialize_route_on_substrate(
        &mut self,
        input: RouteMaterializationInput,
        substrate: SubstrateLease,
        parameters: LayerParameters,
    ) -> Result<RouteInstallation, RouteError>;
}

pub trait RouteFamily: RoutePlanner {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError>;
}
```

This split shows the main route-building sequence. The important point is that route construction starts from shared observations, becomes inferential during candidate production, becomes proof-bearing at admission, and becomes canonical only when the router allocates route identity and the family realizes that admitted route under the router-provided `RouteMaterializationInput`. The planning side is deterministic and read-only with respect to canonical route state. Runtime mutation starts at `materialize_route`.

`CommitteeSelector` sits on the same planning side when a family uses it. Jacquard commits to the shared result shape of the committee, not to one universal committee-selection policy. Families may use leaderless threshold sets, role-differentiated committees, or no committee at all.

`SubstratePlanner` and `LayeredRoutePlanner` stay on the deterministic planning side. `SubstrateRuntime` and `LayeredRouteFamily` own the effectful acquisition and realization steps. That keeps layering aligned with the same purity rule as `RoutePlanner` versus `RouteFamily`, and it prevents composition from collapsing planning and runtime mutation into one trait.

## Family Boundary

`RoutePlanner` is the deterministic planning boundary. `RouteFamily` is the effectful runtime boundary on top of it. A planner produces candidates, checks admission, and admits a route. The router allocates canonical route identity. The family runtime realizes that route under the router-owned handle and lease, publishes commitments, and handles maintenance. The top-level router stays family-neutral: it compares candidates, enforces fallback rules, tracks materialized routes, and coordinates maintenance.

See [Extensibility](107_extensibility.md) for the full extension surface, including route families, transports, effects, hashing, content addressing, and simulation.

## Runtime Boundary

The routing core does not call platform APIs directly. Hashing, storage, audit emission, transport ingress, time, and ordering all cross explicit runtime-effect traits. See [Crate Architecture](106_crate_architecture.md) for the full effect trait inventory and the simulator reuse argument.
