//! Abstract routing traits: policy engines, routing engines, the router, and
//! control/data planes.

use jacquard_core::{
    AdaptiveRoutingProfile, CommitteeSelection, Configuration, LayerParameters,
    MaterializedRoute, MaterializedRouteIdentity, NodeId, Observation, RouteAdmission,
    RouteAdmissionCheck, RouteCandidate, RouteCommitment, RouteError, RouteHealth,
    RouteId, RouteInstallation, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteMaterializationInput, RouteRuntimeState, RouteSemanticHandoff,
    RouterMaintenanceOutcome, RouterTickOutcome, RoutingEngineCapabilities,
    RoutingEngineId, RoutingObjective, RoutingPolicyInputs, RoutingTickChange,
    RoutingTickContext, RoutingTickOutcome, SubstrateCandidate, SubstrateLease,
    SubstrateRequirements,
};
use jacquard_macros::purity;

#[purity(pure)]
/// Owns the protection-versus-connectivity decision. In a mesh-only deployment,
/// this may return a fixed profile. Richer policy comes from the embedding
/// host.
///
/// Pure deterministic boundary.
pub trait PolicyEngine {
    #[must_use]
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        inputs: &RoutingPolicyInputs,
    ) -> AdaptiveRoutingProfile;
}

#[purity(pure)]
/// Optional deterministic boundary for engine-local committee selection.
///
/// This trait makes the result shape abstract without forcing Jacquard core to
/// standardize one committee algorithm across routing engines. Selectors return
/// `Some(CommitteeSelection)` when a committee applies and `None` when local
/// coordination is not needed for the current regime or objective.
pub trait CommitteeSelector {
    type TopologyView;

    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, RouteError>;
}

#[purity(read_only)]
/// Optional read-only boundary for routing engines that delegate committee
/// formation to a swappable selector component.
///
/// Jacquard standardizes the shared `CommitteeSelection` result shape, not one
/// universal formation process. Engines that use committees may expose the
/// selector they depend on through this trait. Engines that do not use
/// committees simply do not implement it.
pub trait CommitteeCoordinatedEngine {
    type Selector: CommitteeSelector;

    #[must_use]
    fn committee_selector(&self) -> Option<&Self::Selector>;
}

#[purity(pure)]
/// Optional deterministic boundary for routing engines that can advertise
/// lower-layer carriage to other routing engines or to a host-level policy
/// engine.
///
/// This is a forward-looking contract surface. Jacquard commits to the shared
/// shape of substrate planning, but there is no in-tree production engine using
/// it yet beyond contract tests.
pub trait SubstratePlanner {
    #[must_use]
    fn candidate_substrates(
        &self,
        requirements: &SubstrateRequirements,
        topology: &Observation<Configuration>,
    ) -> Vec<SubstrateCandidate>;
}

#[purity(effectful)]
/// Optional effectful boundary for routing engines that can acquire and manage
/// substrate leases after planning has selected one.
///
/// This is a forward-looking contract surface. It exists so host-owned
/// composition can stabilize before every in-tree engine uses it in production.
pub trait SubstrateRuntime {
    fn acquire_substrate(
        &mut self,
        candidate: SubstrateCandidate,
    ) -> Result<SubstrateLease, RouteError>;

    fn release_substrate(&mut self, lease: &SubstrateLease) -> Result<(), RouteError>;

    /// Runtime observation over an acquired substrate lease. This is read-only
    /// with respect to canonical route truth.
    fn observe_substrate_health(
        &self,
        lease: &SubstrateLease,
    ) -> Result<Observation<RouteHealth>, RouteError>;
}

#[purity(pure)]
/// Optional deterministic boundary for routing engines that can plan over an
/// already-admitted substrate route rather than only over direct local links.
///
/// This is a forward-looking contract surface. The planner/runtime split is
/// intentional, but the semantics remain lightly exercised today.
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

#[purity(effectful)]
/// Optional effectful boundary for layered routing engines once planning has
/// selected a substrate-backed route candidate.
///
/// This is a forward-looking contract surface. Contract tests cover the shape,
/// not a mature in-tree layering implementation.
pub trait LayeredRoutingEngine: RoutingEngine + LayeredRoutingEnginePlanner {
    fn materialize_route_on_substrate(
        &mut self,
        input: RouteMaterializationInput,
        substrate: SubstrateLease,
        parameters: LayerParameters,
    ) -> Result<RouteInstallation, RouteError>;
}

#[purity(pure)]
/// The pure or near-pure planning surface for one routing engine. Planner
/// methods should be deterministic with respect to their inputs and must not
/// materialize, activate, or mutate canonical route state.
///
/// Pure deterministic boundary.
pub trait RoutingEnginePlanner {
    #[must_use]
    fn engine_id(&self) -> RoutingEngineId;

    #[must_use]
    fn capabilities(&self) -> RoutingEngineCapabilities;

    /// Candidate enumeration consumes observational topology input and must
    /// return advisory route candidates rather than proof-bearing witnesses.
    #[must_use]
    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate>;

    /// Engine-level feasibility check against the current observed topology.
    ///
    /// Rule:
    /// - if a planning judgment depends on observations, that observation
    ///   context must be explicit in the method inputs
    /// - backend refs may be opaque engine-private plan tokens, but engines
    ///   must not depend semantically on hidden mutable planner caches
    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    /// Admit one candidate against the current observed topology.
    ///
    /// Candidate admission may reuse internal memoization, but the topology
    /// argument remains authoritative. Engines must be able to re-derive the
    /// admission result from the candidate plus explicit observation context
    /// rather than depending on ambient planner state.
    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError>;
}

#[purity(effectful)]
/// The effectful routing-engine boundary. Each routing engine (eg. mesh)
/// implements this trait. Jacquard core interacts with engine runtime state
/// only through this surface.
///
/// Effectful runtime boundary.
pub trait RoutingEngine: RoutingEnginePlanner {
    /// Realize runtime state for a route under router-owned canonical identity.
    ///
    /// The router allocates the canonical handle and lease first, then the
    /// routing engine installs the admitted route under that identity and
    /// returns the engine-owned installation artifacts.
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError>;

    /// Every unresolved or recently resolved engine-side obligation must be
    /// expressible as an explicit route commitment.
    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment>;

    /// Optional engine-wide periodic progress hook.
    ///
    /// Engines may use this to refresh engine-private adaptive state, decay
    /// stale observations, update coordination posture, or perform other
    /// bootstrap and convergence logic that is not tied to one active route.
    /// The default implementation is a no-op.
    ///
    /// This hook must not publish canonical route truth directly. Any
    /// resulting activation, replacement, or maintenance decisions still flow
    /// through the router/control-plane path.
    fn engine_tick(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, RouteError> {
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: RoutingTickChange::NoChange,
        })
    }

    /// Maintenance receives immutable router-owned route identity plus mutable
    /// engine-owned runtime state. Engines must not mutate canonical handle,
    /// lease, or admission through this surface.
    ///
    /// Maintenance returns a typed semantic result so replacement, handoff,
    /// and failure paths keep their payload rather than collapsing to a flag.
    fn maintain_route(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}

#[purity(effectful)]
/// Supplemental engine boundary used by the generic router middleware.
///
/// The shared `RoutingEngine` trait intentionally stops at canonical planning,
/// materialization, commitments, and route-private maintenance. A host-owned
/// router still needs three engine-owned hooks:
/// - the local node identity used for router-scoped checkpoint namespacing
/// - data-plane forwarding over an already-admitted route
/// - restoration of engine-private runtime state during router-led recovery
///
/// This remains generic middleware surface area rather than family-specific
/// mesh behavior. Any engine that wants to sit behind the in-tree router must
/// provide these hooks without exposing engine-private internals.
pub trait RouterManagedEngine: RoutingEngine {
    #[must_use]
    fn local_node_id_for_router(&self) -> NodeId;

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError>;

    fn restore_route_runtime_for_router(
        &mut self,
        route_id: &RouteId,
    ) -> Result<bool, RouteError>;
}

#[purity(effectful)]
/// Registry boundary for router-managed engines.
///
/// This is the composable middleware seam between the generic router and
/// concrete routing engines. Hosts register engines here, then the router's
/// orchestration layer enumerates candidates, chooses one engine's evidence,
/// and publishes canonical route truth above that boundary.
pub trait RouterEngineRegistry {
    fn register_engine(
        &mut self,
        extension: Box<dyn RouterManagedEngine>,
    ) -> Result<(), RouteError>;

    #[must_use]
    fn registered_engine_ids(&self) -> Vec<RoutingEngineId>;

    fn registered_engine_capabilities(
        &self,
        engine_id: &RoutingEngineId,
    ) -> Option<RoutingEngineCapabilities>;
}

#[purity(effectful)]
/// Router-owned middleware runtime for composable engines.
///
/// This is the engine-agnostic orchestration layer: it owns authoritative
/// topology input, engine recovery, and policy input refresh while delegating
/// route-private planning/runtime work to registered engines.
pub trait RoutingMiddleware: RouterEngineRegistry {
    fn replace_topology(&mut self, topology: Observation<Configuration>);

    fn replace_policy_inputs(&mut self, inputs: RoutingPolicyInputs);

    fn recover_checkpointed_routes(&mut self) -> Result<usize, RouteError>;
}

#[purity(effectful)]
/// Cross-engine canonical control-plane entry point.
///
/// Effectful runtime boundary.
pub trait Router {
    fn activate_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError>;

    fn route_commitments(
        &self,
        route_id: &RouteId,
    ) -> Result<Vec<RouteCommitment>, RouteError>;

    fn reselect_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<MaterializedRoute, RouteError>;

    fn transfer_route_lease(
        &mut self,
        route_id: &RouteId,
        handoff: RouteSemanticHandoff,
    ) -> Result<MaterializedRoute, RouteError>;
}

#[purity(effectful)]
/// Host-owned orchestration boundary for policy-driven composition across
/// routing engines. This is where smooth transition and limited layering live.
///
/// This is a forward-looking contract surface. Jacquard exposes it so hosts can
/// build gradual migration and limited layering without engine cross-awareness,
/// but current in-tree coverage is still contract-oriented.
pub trait LayeringPolicyEngine {
    fn activate_layered_route(
        &mut self,
        objective: RoutingObjective,
        outer_engine: RoutingEngineId,
        substrate_requirements: SubstrateRequirements,
        parameters: LayerParameters,
    ) -> Result<MaterializedRoute, RouteError>;
}

#[purity(effectful)]
/// Control plane owns route truth. Data plane owns forwarding over admitted
/// truth.
///
/// Effectful runtime boundary.
pub trait RoutingControlPlane {
    fn activate_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError>;

    fn maintain_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouterMaintenanceOutcome, RouteError>;

    /// Periodic consistency sweep: refresh engine-wide adaptive state, expire
    /// leases, and detect stale routes.
    fn anti_entropy_tick(&mut self) -> Result<RouterTickOutcome, RouteError>;
}

#[purity(effectful)]
/// Forwarding and observational reads over already admitted route state.
///
/// `observe_route_health` is read-only with respect to canonical routing truth.
/// It may inspect data-plane state, but it must not publish canonical route
/// changes on its own.
///
/// Effectful runtime boundary with read-only observation methods.
pub trait RoutingDataPlane {
    fn forward_payload(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError>;

    /// Health reads are observational. They must not silently become canonical
    /// route truth without an explicit control-plane publication step.
    fn observe_route_health(
        &self,
        route_id: &RouteId,
    ) -> Result<Observation<RouteHealth>, RouteError>;
}
