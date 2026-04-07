# Routing Engines

This page describes the trait surface for adding a routing algorithm to Jacquard. See [World Extensions](107_world_extensions.md) for the layering overview, [Runtime Effects](104_runtime_effects.md) for the host capability surface, and [Mesh Routing](109_mesh_routing.md) for the in-tree mesh implementation and its swappable subcomponents.

## Routing Engine Contract

A routing engine is a routing algorithm that consumes the shared world picture and realizes routes under router-provided identity. Mesh is the first-party engine. External engines such as onion routing plug into the same contract without depending on mesh internals.

```rust
pub trait RoutingEnginePlanner {
    #[must_use]
    fn engine_id(&self) -> RoutingEngineId;

    #[must_use]
    fn capabilities(&self) -> RoutingEngineCapabilities;

    #[must_use]
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

    fn engine_tick(&mut self, topology: &Observation<Configuration>) -> Result<(), RouteError> {
        Ok(())
    }

    fn maintain_route(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}
```

`RoutingEnginePlanner` is pure. `RoutingEngine` is effectful. The split keeps candidate production deterministic and keeps runtime mutation inside explicit realization and maintenance methods. The router allocates canonical route identity first. The engine realizes the admitted route under that identity and returns `RouteInstallation`. The final `MaterializedRoute` is assembled above the engine boundary as router-owned identity plus engine-owned runtime state, and maintenance only receives the mutable runtime portion.

That activation step also enforces the shared control-plane invariants. The admission decision must still be admissible. The realized protection must satisfy the objective protection floor. Lease validity must be checked explicitly before maintenance or publication proceeds.

## Engine Tick

`engine_tick` is the optional engine-wide bootstrap and convergence hook. An engine may use it as an internal middleware-style loop to refresh local regime estimates, decay stale local state, update coordination posture, or prepare engine-private planning state before any specific route is active. The host or router drives that cadence through the control plane's existing periodic tick path. The hook itself does not publish canonical route truth directly.

## Contract Rules

Two implementation rules are worth keeping explicit. If a planning or admission judgment depends on observations, the current topology must be passed into that method directly rather than read from ambient engine state. And if an engine keeps planner caches, those caches are memoization only: cache hits and misses must not change the semantic result for the same topology.

External routing engines should depend on `jacquard-core` and `jacquard-traits`. They should not depend on mesh internals, router internals, or simulator-private helpers. The stable shared contract includes `RouteSummary`, `Estimate<RouteEstimate>`, `RouteAdmissionCheck`, `RouteWitness`, `RouteHandle`, `RouteLease`, `RouteMaterializationInput`, `RouteInstallation`, `RouteCommitment`, `RouteMaintenanceResult`, `CommitteeSelection`, `SubstrateRequirements`, `SubstrateLease`, `LayerParameters`, `Observation<T>`, and `Fact<T>`. External engines must not assume mesh route shape, mesh topology structure, mesh-specific maintenance semantics, or any authority model outside those shared route objects.

## Policy And Coordination

Policy and coordination traits are separate from route realization. They cover host policy, optional local coordination results, and engine layering without direct engine-to-engine awareness.

```rust
pub trait PolicyEngine {
    #[must_use]
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        inputs: &RoutingPolicyInputs,
    ) -> AdaptiveRoutingProfile;
}

pub trait CommitteeSelector {
    type TopologyView;

    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, RouteError>;
}

pub trait CommitteeCoordinatedEngine {
    type Selector: CommitteeSelector;

    fn committee_selector(&self) -> Option<&Self::Selector>;
}

pub trait SubstratePlanner {
    #[must_use]
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

    fn release_substrate(&mut self, lease: &SubstrateLease) -> Result<(), RouteError>;

    fn observe_substrate_health(
        &self,
        lease: &SubstrateLease,
    ) -> Result<Observation<RouteHealth>, RouteError>;
}

pub trait LayeredRoutingEnginePlanner {
    #[must_use]
    fn candidate_routes_on_substrate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        substrate: &SubstrateLease,
        parameters: &LayerParameters,
    ) -> Vec<RouteCandidate>;

    fn admit_route_on_substrate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        substrate: &SubstrateLease,
        parameters: &LayerParameters,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;
}

pub trait LayeredRoutingEngine: RoutingEngine + LayeredRoutingEnginePlanner {
    fn materialize_route_on_substrate(
        &mut self,
        input: RouteMaterializationInput,
        substrate: SubstrateLease,
        parameters: LayerParameters,
    ) -> Result<RouteInstallation, RouteError>;
}

pub trait LayeringPolicyEngine {
    fn activate_layered_route(
        &mut self,
        objective: RoutingObjective,
        outer_engine: RoutingEngineId,
        substrate_requirements: SubstrateRequirements,
        parameters: LayerParameters,
    ) -> Result<MaterializedRoute, RouteError>;
}
```

`PolicyEngine`, `CommitteeSelector`, `CommitteeCoordinatedEngine`, `SubstratePlanner`, and `LayeredRoutingEnginePlanner` are planning or read-only surfaces. `SubstrateRuntime`, `LayeredRoutingEngine`, and `LayeringPolicyEngine` are effectful. `CommitteeSelector` is optional. Jacquard standardizes the `CommitteeSelection` result shape, not one formation algorithm, and selectors may return `None` when no committee applies.

Selector implementations may be engine-local, host-local, provisioned, or otherwise out of band. The substrate and layering traits are still forward-looking contract surfaces for host-owned composition.
