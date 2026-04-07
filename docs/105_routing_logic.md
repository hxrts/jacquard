# Routing Decisions

This page picks up the routing pipeline at estimation and walks through policy, action, and route realization. It covers the control and data plane split, the candidate-to-materialization decision path, and the routing-engine boundary. See [Pipeline and World Observations](104_pipeline_observations.md) for the world and observation stages that feed into this page.

## Planes

The control plane owns candidate gathering, admission, canonical identity allocation, materialized-route assembly, commitments, maintenance, and anti-entropy. The data plane forwards payloads over already admitted route state. Data-plane observations may report health or failures. The control plane decides whether any of that changes the active materialized route.

Local coordination also belongs to the control plane. A routing engine may select a committee or witness set as part of planning. Those results are advisory inputs to canonical transitions rather than canonical route truth.

The link layer is a frame carrier. It reports reachability, MTU, loss, and timing. It does not own canonical ordering or traffic control. A routing engine that needs sequencing or causal behavior expresses it as a routing-level message-flow assumption rather than a transport guarantee.

### Layered Composition

Layered composition follows the same rule as plane separation. If one routing engine uses another as a limited substrate, the layering decision belongs above both engines in a host-owned policy engine.

The lower engine exposes carrier capabilities and leases. The upper engine consumes those through a neutral contract. Neither engine needs direct awareness of the other's private scoring or maintenance logic.

## Decision Path

The routing decision path starts from a `RoutingObjective` and a current `Observation<Configuration>`. A routing-engine planner turns those into `RouteCandidate` values. Each candidate carries an `Estimate<RouteEstimate>` rather than a fact or published witness.

The planner checks a candidate and admits it under a stated profile. The router then allocates canonical route identity. The routing engine realizes that admitted route under `RouteMaterializationInput`. The control plane assembles the final `MaterializedRoute` from router-owned `MaterializedRouteIdentity` plus engine-owned `RouteRuntimeState`.

```rust
pub trait RoutingEnginePlanner {
    fn engine_id(&self) -> RoutingEngineId;
    fn capabilities(&self) -> RoutingEngineCapabilities;

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate>;

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError>;
}

pub trait RoutingEngine: RoutingEnginePlanner {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError>;

    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment>;

    fn engine_tick(
        &mut self,
        topology: &Observation<Configuration>,
    ) -> Result<(), RouteError> { Ok(()) }

    fn maintain_route(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}
```

`RoutingEnginePlanner` is the pure planning surface. Runtime mutation is confined to `RoutingEngine::materialize_route`, `maintain_route`, and `teardown`. The control plane enforces the objective protection floor at materialization. Expired leases surface as a typed failure rather than silently continuing.

### Contract Rules

Two implementation rules keep the planning surface honest. First, any planning or admission judgment that depends on observed world state must receive `&Observation<Configuration>` explicitly rather than reading an ambient topology snapshot. Second, planner caches are optimization only.

Cache misses must still lead to the same planning or admission result for the same topology. Admitted routes must carry enough opaque engine-private plan state forward that materialization can proceed without a planner-cache lookup. Materialization must fail closed when required observed topology is missing. Successful lifecycle transitions must remain replay-visible before public or durable state is committed, so engines should stage the next runtime state off to the side until checkpointing and route-event logging succeed.

Route health should also stay route-scoped rather than engine-global. When an engine can validate the active route's remaining suffix against current observations, it should publish route-local reachability and stability. When it cannot, it should use an explicit unknown reachability state rather than silently treating the route as healthy or dead from unrelated engine-level signals.

`engine_tick` is the optional engine-wide convergence hook for refreshing local regime estimates, decaying stale state, retaining bounded observational summaries, or updating coordination posture before any specific route is active. In richer engines that can include maintaining bounded repair pressure, anti-entropy pressure, or transport-derived health posture. Committee selection, substrate planning, and layered routing follow the same pure and effectful split. See [Extensibility](106_extensibility.md) for the full trait signatures.

### Overlay Example

Layering lets an overlay engine use mesh as a carrier without awareness of mesh-private topology. Mesh provides substrate reachability inside one cluster. The overlay engine consumes those paths as leased substrates for inter-cluster carriage or egress.

```mermaid
graph TD
    subgraph overlay["overlay engine"]
        o_plan["planning: role classification, candidate refresh"]
        o_activate["activation: route materialization"]
        o_maintain["maintenance: replace or teardown"]

        o_plan --> o_activate --> o_maintain
    end

    substrate["substrate lease"]

    subgraph mesh["mesh engine"]
        m_plan["planning: topology model, candidate production"]
        m_activate["activation: route materialization"]
        m_maintain["maintenance: forwarding, repair, retention"]

        m_plan --> m_activate --> m_maintain
    end

    o_maintain --> substrate --> m_plan
```

The overlay engine's `engine_tick` drives the middleware stages shown above. It classifies the local node as member, bridge, or gateway. It then refreshes overlay posture and candidate state before any specific route is activated. Route activation, maintenance, and teardown still use the shared `RoutingEngine` traits.
