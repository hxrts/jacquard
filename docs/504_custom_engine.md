# Custom Engine

This guide walks through implementing a custom routing engine from scratch and registering it with the router. The worked example is a minimal no-op engine that declares opaque route visibility and always reports lost reachability. It is short and unhelpful as a routing algorithm, but it exercises every trait a real engine must implement.

See [Routing Engines](303_routing_engines.md) for the engine contract spec. For the in-tree engines that demonstrate specific patterns, see [Pathway Routing](404_pathway_routing.md), [Batman Routing](401_batman_routing.md), [Babel Routing](402_babel_routing.md), [OLSRv2 Routing](403_olsrv2_routing.md), [Scatter Routing](405_scatter_routing.md), and [Field Routing](406_field_routing.md).

## Required Traits

Every routing engine implements three traits from `jacquard-traits`. `RoutingEnginePlanner` is the pure surface: identity, capability advertisement, candidate enumeration, and admission check. `RoutingEngine` extends the planner with the effectful surface: materialization, maintenance, and teardown. `RouterManagedEngine` adds the hooks the generic router middleware needs for forwarding, restore, and ingress.

The model-trait family is optional. `RoutingEnginePlannerModel`, `RoutingEngineRoundModel`, `RoutingEngineMaintenanceModel`, and `RoutingEngineRestoreModel` let the simulator drive engine-owned pure reducers. A first pass can skip the model traits and implement them later when the engine integrates with experiment suites.

Define the engine struct and any private state first. The no-op example carries nothing beyond its identity and a handful of constants.

```rust
use jacquard_core::{NodeId, RoutingEngineId};

pub const OPAQUE_NOOP_ENGINE_ID: RoutingEngineId = RoutingEngineId::new(*b"jacquard.opnoop.");

pub struct OpaqueNoopEngine {
    local_node_id: NodeId,
}

impl OpaqueNoopEngine {
    pub fn new(local_node_id: NodeId) -> Self {
        Self { local_node_id }
    }
}
```

`RoutingEngineId` is a 16-byte identifier that distinguishes one engine from another in shared observation and service-descriptor surfaces. Keep it stable across releases so network peers can resolve the engine by id without a compatibility shim.

## Declaring Identity And Capabilities

The planner surface starts with identity and capability advertisement. Engines declare what they can do so the router can filter candidates and skip engines that cannot satisfy a given objective.

```rust
use jacquard_core::{
    RouteProtectionClass, RouteShapeVisibility, RouteRepairClass, RoutePartitionClass,
    RoutingEngineCapabilities,
};

impl OpaqueNoopEngine {
    fn capabilities_decl() -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: RoutePartitionClass::ConnectedOnly,
            repair_support: RouteRepairClass::Unsupported,
            hold_support: RoutePartitionClass::Unsupported,
            route_shape_visibility: RouteShapeVisibility::Opaque,
        }
    }
}
```

`RouteShapeVisibility` signals what shape of route the engine publishes. Pathway uses `ExplicitPath`, field uses `CorridorEnvelope`, the distance-vector engines use `NextHopOnly`, and scatter uses `Opaque`. Choose the weakest shape the engine actually provides. Routers will not promise callers a richer shape than the engine advertises.

Capability choices cascade into admission. An engine that advertises `Unsupported` for `repair_support` will be skipped for repair-bearing objectives.

## Minimum Planner Path

The planner surface takes a routing objective, a profile, and a topology observation. It returns candidate routes and admission decisions. The no-op example returns empty candidates, which is valid and leaves admission unreachable.

```rust
use jacquard_core::{
    Configuration, Observation, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteError,
    RoutingObjective, SelectedRoutingParameters,
};
use jacquard_traits::RoutingEnginePlanner;

impl RoutingEnginePlanner for OpaqueNoopEngine {
    fn engine_id(&self) -> jacquard_core::RoutingEngineId { OPAQUE_NOOP_ENGINE_ID }
    fn capabilities(&self) -> RoutingEngineCapabilities { Self::capabilities_decl() }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        Vec::new()
    }
    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        Err(RouteError::UnknownEngine)
    }
}
```

A real engine produces non-empty candidates from the current topology, tags each with a backend reference, and decides admission based on the engine-private assessment of that candidate. See [Routing Engines](303_routing_engines.md) for the planner contract rules, including the invariant that admission judgments must come from the current topology rather than hidden planner cache state.

## Materialization And Maintenance

The effectful surface completes the engine. Materialization installs runtime state under the router-owned canonical identity. Maintenance reports route health each round and drives replacement decisions. Teardown releases engine-private state when the router retires a route.

```rust
use jacquard_core::{
    MaterializedRoute, PublishedRouteRecord, RouteCommitment, RouteId, RouteInstallation,
    RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceResult,
    RouteMaintenanceTrigger, RouteMaterializationInput, RouteRuntimeState,
};
use jacquard_traits::RoutingEngine;

impl RoutingEngine for OpaqueNoopEngine {
    fn materialize_route(
        &mut self,
        _input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        Err(RouteError::NoCandidate)
    }
    fn route_commitments(&self, _route: &MaterializedRoute) -> Vec<RouteCommitment> {
        Vec::new()
    }
    fn maintain_route(
        &mut self,
        _identity: &PublishedRouteRecord,
        _runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        Ok(RouteMaintenanceResult {
            event: Default::default(),
            outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
        })
    }
    fn teardown(&mut self, _route_id: &RouteId) {}
}
```

A real engine materializes route-private runtime under the router's canonical identity and returns a `RouteInstallation` that describes what it realized. Maintenance returns a typed `RouteMaintenanceOutcome` that drives router behavior: `Continued` and `Repaired` preserve the route, `ReplacementRequired` triggers reselection, `HandedOff` transfers the lease, and `Failed` surfaces the failure variant.

The `RouterManagedEngine` trait fills in the remaining router-side hooks. Implement `local_node_id_for_router`, `forward_payload_for_router`, and the restore methods. Default implementations apply for the ingress-observation hook when the engine does not consume transport observations directly.

## Registering With The Router

An engine enters a live routing stack through either `MultiEngineRouter::register_engine` (manual composition) or through the engine-specific `ClientBuilder` entry point (reference client composition).

```rust
use jacquard_reference_client::{ClientBuilder, MultiEngineRouter};

// Manual composition:
let mut router = MultiEngineRouter::default();
router.register_engine(Box::new(OpaqueNoopEngine::new(local_node_id)));

// Reference client composition expects the in-tree constructors.
// A custom engine either lives inside a downstream fork of ClientBuilder,
// or the caller composes the router and engines manually and wraps them
// in a custom host harness.
```

Nodes advertise engine eligibility in their `ServiceDescriptor`. A destination that tags the custom engine's `RoutingEngineId` becomes eligible for candidate production from that engine. Tagging happens through the node profile, documented in [Custom Device](506_custom_device.md).

If the engine ships in its own crate, consider implementing `RoutingEnginePlannerModel` as well. The simulator drives model-lane fixtures through that trait, which lets the engine participate in the maintained experiment corpus without full-stack wiring. See [Simulator Architecture](306_simulator_architecture.md) for the model-lane design.

## Going Further

For contract enforcement, the `toolkit/checks/rust/routing_invariants.rs` policy check validates engine crates against the shared rules. Run `cargo xtask check routing-invariants` during development.

For integration into experiment suites, see [Running Experiments](502_running_experiments.md). For a capstone that threads a custom engine through the simulator and the report pipeline, see [Bringing It Together](507_bringing_it_together.md).
