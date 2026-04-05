//! Abstract routing traits: policy engines, routing engines, the router, and
//! control/data planes.

use jacquard_core::{
    AdaptiveRoutingProfile, CommitteeSelection, Configuration, LayerParameters, MaterializedRoute,
    Observation, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCommitment, RouteError,
    RouteHealth, RouteId, RouteInstallation, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteMaterializationInput, RoutingEngineCapabilities, RoutingEngineId, RoutingObjective,
    RoutingPolicyInputs, SubstrateCandidate, SubstrateLease, SubstrateRequirements,
};
use jacquard_macros::purity;

#[purity(pure)]
/// Owns the protection-versus-connectivity decision. In a mesh-only deployment,
/// this may return a fixed profile. Richer policy comes from the embedding host.
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
/// standardize one committee algorithm across routing engines.
pub trait CommitteeSelector {
    type TopologyView;

    #[must_use]
    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<CommitteeSelection, RouteError>;
}

#[purity(pure)]
/// Optional deterministic boundary for routing engines that can advertise
/// lower-layer carriage to other routing engines or to a host-level policy
/// engine.
pub trait SubstratePlanner {
    #[must_use]
    fn candidate_substrates(
        &self,
        requirements: &SubstrateRequirements,
        topology: &Observation<Configuration>,
    ) -> Vec<SubstrateCandidate>;
}

#[purity(effectful)]
/// Optional effectful boundary for families that can acquire and manage
/// substrate leases after planning has selected one.
pub trait SubstrateRuntime {
    #[must_use]
    fn acquire_substrate(
        &mut self,
        candidate: SubstrateCandidate,
    ) -> Result<SubstrateLease, RouteError>;

    fn release_substrate(&mut self, lease: &SubstrateLease) -> Result<(), RouteError>;

    /// Runtime observation over an acquired substrate lease. This is read-only
    /// with respect to canonical route truth.
    #[must_use]
    fn observe_substrate_health(
        &self,
        lease: &SubstrateLease,
    ) -> Result<Observation<RouteHealth>, RouteError>;
}

#[purity(pure)]
/// Optional deterministic boundary for routing engines that can plan over an
/// already-admitted substrate route rather than only over direct local links.
pub trait LayeredRoutingEnginePlanner {
    #[must_use]
    fn candidate_routes_on_substrate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        substrate: &SubstrateLease,
        parameters: &LayerParameters,
    ) -> Vec<RouteCandidate>;

    #[must_use]
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
/// Optional effectful boundary for layered routing engines once planning has selected
/// a substrate-backed route candidate.
pub trait LayeredRoutingEngine: RoutingEngine + LayeredRoutingEnginePlanner {
    #[must_use]
    fn materialize_route_on_substrate(
        &mut self,
        input: RouteMaterializationInput,
        substrate: SubstrateLease,
        parameters: LayerParameters,
    ) -> Result<RouteInstallation, RouteError>;
}

#[purity(pure)]
/// The pure or near-pure planning surface for one routing engine. Planner methods
/// should be deterministic with respect to their inputs and must not materialize,
/// activate, or mutate canonical route state.
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

    /// Engine-level feasibility check. May attach step bounds and cost estimates.
    #[must_use]
    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    #[must_use]
    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
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
    #[must_use]
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError>;

    /// Every unresolved or recently resolved engine-side obligation must be
    /// expressible as an explicit route commitment.
    #[must_use]
    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment>;

    /// Maintenance returns a typed semantic result so replacement, handoff, and
    /// failure paths keep their payload rather than collapsing to a flag.
    #[must_use]
    fn maintain_route(
        &mut self,
        route: &mut MaterializedRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}

#[purity(effectful)]
/// Cross-engine orchestration entry point.
///
/// Effectful runtime boundary.
pub trait Router {
    fn register_engine(&mut self, extension: Box<dyn RoutingEngine>) -> Result<(), RouteError>;

    #[must_use]
    fn activate_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError>;

    #[must_use]
    fn route_commitments(&self, route_id: &RouteId) -> Result<Vec<RouteCommitment>, RouteError>;

    #[must_use]
    fn reselect_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<MaterializedRoute, RouteError>;
}

#[purity(effectful)]
/// Host-owned orchestration boundary for policy-driven composition across
/// routing engines. This is where smooth transition and limited layering live.
pub trait LayeringPolicyEngine {
    #[must_use]
    fn activate_layered_route(
        &mut self,
        objective: RoutingObjective,
        outer_engine: RoutingEngineId,
        substrate_requirements: SubstrateRequirements,
        parameters: LayerParameters,
    ) -> Result<MaterializedRoute, RouteError>;
}

#[purity(effectful)]
/// Control plane owns route truth. Data plane owns forwarding over admitted truth.
///
/// Effectful runtime boundary.
pub trait RoutingControlPlane {
    #[must_use]
    fn activate_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError>;

    #[must_use]
    fn maintain_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    /// Periodic consistency sweep: expire leases, detect stale routes.
    fn anti_entropy_tick(&mut self) -> Result<(), RouteError>;
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
    fn forward_payload(&mut self, route_id: &RouteId, payload: &[u8]) -> Result<(), RouteError>;

    /// Health reads are observational. They must not silently become canonical
    /// route truth without an explicit control-plane publication step.
    #[must_use]
    fn observe_route_health(
        &self,
        route_id: &RouteId,
    ) -> Result<Observation<RouteHealth>, RouteError>;
}
