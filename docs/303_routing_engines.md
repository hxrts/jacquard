# Routing Engines

This page describes the trait surface for adding a routing algorithm to Jacquard. It also captures the host capability boundary that engines consume and the in-tree engine shapes. See [Pathway Routing](401_pathway_routing.md) for the explicit-path engine, [Batman Routing](402_batman_routing.md) for the batman-bellman and batman-classic next-hop engines, [Field Routing](403_field_routing.md) for the corridor-envelope engine, [Babel Routing](404_babel_routing.md) for the RFC 8966 distance-vector engine, and [OLSRv2 Routing](405_olsrv2_routing.md) for the deterministic link-state engine.

## Routing Engine Contract

A routing engine is a routing algorithm that consumes the shared world picture and realizes routes under router-provided identity. Jacquard ships six in-tree engines: `pathway` (explicit-path), `field` (corridor-envelope), `batman-bellman` (Bellman-Ford-enhanced next-hop), `batman-classic` (spec-faithful BATMAN IV next-hop), `babel` (RFC 8966 distance-vector), and `olsrv2` (OLSRv2 link-state). External engines such as onion routing plug into the same contract without depending on any in-tree engine's internals.

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
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate>;

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
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
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, RouteError> {
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: RoutingTickChange::NoChange,
            next_tick_hint: RoutingTickHint::HostDefault,
        })
    }

    fn maintain_route(
        &mut self,
        identity: &PublishedRouteRecord,
        runtime: &mut RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}
```

`RoutingEnginePlanner` is pure. `RoutingEngine` is effectful. The split keeps candidate production deterministic and keeps runtime mutation inside explicit realization and maintenance methods. The router allocates canonical route identity first. The engine realizes the admitted route under that identity and returns `RouteInstallation`. The final `MaterializedRoute` is assembled above the engine boundary as router-owned identity plus engine-owned runtime state, and maintenance only receives the mutable runtime portion.

That activation step also enforces the shared control-plane invariants. The admission decision must still be admissible. The realized protection must satisfy the objective protection floor. Lease validity must be checked explicitly before maintenance or publication proceeds.

## Engine Tick

`engine_tick` is the optional engine-wide bootstrap and convergence hook. The router or host owns cadence and passes a shared `RoutingTickContext` containing the authoritative merged topology observation for that step. The engine returns a small `RoutingTickOutcome` so the router can observe whether the tick changed engine-private state without standardizing engine internals. The hook itself does not publish canonical route truth directly.

`RoutingTickOutcome.next_tick_hint` is advisory scheduling pressure, not self-scheduling authority. Proactive engines such as Babel- or BATMAN-style implementations can report that more work is due soon, but the host/router still owns final cadence.

An engine may still use a richer internal runtime model behind that hook. First-party pathway, for example, now drives protocol-side ingress and bounded control-state refresh through a private choreography guest runtime while keeping the shared `engine_tick` signature unchanged.

That private choreography runtime does not replace the shared Jacquard effect traits. Generated Telltale effect interfaces remain engine-private implementation details, and the pathway interpreter adapts them onto the stable `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, and `TransportSenderEffects` surfaces exposed by `jacquard-traits`. Host-owned `TransportDriver` implementations now stop at the router or bridge layer, which delivers explicit ingress before each synchronous router round.

First-party field follows the same ownership rule, but with a narrower proof
boundary: the deterministic local observer-controller remains the semantic
owner of corridor belief and posture choice, while any field-private
choreography layer may provide only observational summary inputs. Canonical
route publication remains router-owned.

## Runtime Effect Boundary

The host capability surface stays narrow on purpose.

- `TransportSenderEffects` is the shared synchronous send capability engines use during a deterministic round.
- `TransportDriver` is the host-owned ingress and supervision surface.
- `TimeEffects`, `OrderEffects`, `StorageEffects`, and `RouteEventLogEffects` remain capability traits, not runtime-owner traits.

Engines do not own async streams, driver supervision loops, or Jacquard time assignment. Hosts and bridges own those responsibilities and inject observations before the next synchronous router round.

## Contract Rules

Two implementation rules are worth keeping explicit. If a planning or admission judgment depends on observations, the current topology must be passed into that method directly rather than read from ambient engine state. And if an engine keeps planner caches, those caches are memoization only: cache hits and misses must not change the semantic result for the same topology.

External routing engines should depend on `jacquard-core` and `jacquard-traits`. They should not depend on pathway internals, router internals, or simulator-private helpers. The stable shared contract includes `RouteSummary`, `Estimate<RouteEstimate>`, `RouteAdmissionCheck`, `RouteWitness`, `RouteHandle`, `RouteLease`, `RouteMaterializationInput`, `RouteInstallation`, `RouteCommitment`, `RouteMaintenanceResult`, `CommitteeSelection`, `SubstrateRequirements`, `SubstrateLease`, `LayerParameters`, `Observation<T>`, and `Fact<T>`. External engines must not assume pathway route shape, pathway topology structure, pathway-specific maintenance semantics, or any authority model outside those shared route objects.

## Route Shape Visibility

Jacquard does not require every routing engine to expose a full hop-by-hop path.

- `ExplicitPath` - engine can expose an actual route path shape
- `CorridorEnvelope` - engine exposes a conservative end-to-end corridor envelope without claiming an explicit path
- `NextHopOnly` - engine only claims best-next-hop visibility toward the destination
- `Opaque` - engine does not expose useful route shape beyond viability

This matters for proactive engines. Pathway is `ExplicitPath`. Field is `CorridorEnvelope`. The batman engines (bellman and classic), babel, and olsrv2 are `NextHopOnly`.

## In-Tree Engines

See [Pathway Routing](401_pathway_routing.md), [Batman Routing](402_batman_routing.md), [Field Routing](403_field_routing.md), [Babel Routing](404_babel_routing.md), and [OLSRv2 Routing](405_olsrv2_routing.md) for engine-specific models, capability assumptions, and maintenance behavior.

## Policy And Coordination

Policy and coordination traits are separate from route realization. They cover host policy, optional local coordination results, and engine layering without direct engine-to-engine awareness.

```rust
pub trait PolicyEngine {
    #[must_use]
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        inputs: &RoutingPolicyInputs,
    ) -> SelectedRoutingParameters;
}

pub trait CommitteeSelector {
    type TopologyView;

    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
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
        profile: &SelectedRoutingParameters,
        substrate: &SubstrateLease,
        parameters: &LayerParameters,
    ) -> Vec<RouteCandidate>;

    fn admit_route_on_substrate(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
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
