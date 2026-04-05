//! Abstract routing traits: adaptive controller, route planner, route family, router, and control/data planes.

use jacquard_core::{
    AdaptiveRoutingProfile, CommitteeSelection, Configuration, MaterializedRoute, Observation,
    RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCommitment, RouteError,
    RouteFamilyId, RouteHealth, RouteId, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RoutingFamilyCapabilities, RoutingObjective, RoutingPolicyInputs,
};

/// Owns the protection-versus-connectivity decision. In a mesh-only deployment,
/// this may return a fixed profile. Richer policy comes from the embedding host.
///
/// Pure deterministic boundary.
pub trait RoutingController {
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        inputs: &RoutingPolicyInputs,
    ) -> AdaptiveRoutingProfile;
}

/// Optional deterministic boundary for family-local committee selection.
///
/// This trait makes the result shape abstract without forcing Jacquard core to
/// standardize one committee algorithm across route families.
pub trait CommitteeSelector {
    type TopologyView;

    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<CommitteeSelection, RouteError>;
}

/// The pure or near-pure planning surface for one route family. Planner methods
/// should be deterministic with respect to their inputs and must not materialize,
/// activate, or mutate canonical route state.
///
/// Pure deterministic boundary.
pub trait RoutePlanner {
    fn family_id(&self) -> RouteFamilyId;

    fn capabilities(&self) -> RoutingFamilyCapabilities;

    /// Candidate enumeration consumes observational topology input and must
    /// return advisory route candidates rather than proof-bearing witnesses.
    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate>;

    /// Family-level feasibility check. May attach step bounds and cost estimates.
    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;
}

/// The effectful family boundary. Each route family (eg. mesh) implements
/// this trait. Jacquard core interacts with family runtime state only through this surface.
///
/// Effectful runtime boundary.
pub trait RouteFamily: RoutePlanner {
    /// Materialization is the canonical route-realization step. Success must return an
    /// `MaterializedRoute` carrying a strong canonical handle.
    fn materialize_route(
        &mut self,
        admission: RouteAdmission,
    ) -> Result<MaterializedRoute, RouteError>;

    /// Every unresolved or recently resolved family-side obligation must be
    /// expressible as an explicit route commitment.
    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment>;

    /// Maintenance returns a typed semantic result so replacement, handoff, and
    /// failure paths keep their payload rather than collapsing to a flag.
    fn maintain_route(
        &mut self,
        route: &mut MaterializedRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}

/// Cross-family orchestration entry point.
///
/// Effectful runtime boundary.
pub trait Router {
    fn register_family(&mut self, extension: Box<dyn RouteFamily>) -> Result<(), RouteError>;

    fn activate_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError>;

    fn route_commitments(&self, route_id: &RouteId) -> Result<Vec<RouteCommitment>, RouteError>;

    fn reselect_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<MaterializedRoute, RouteError>;
}

/// Control plane owns route truth. Data plane owns forwarding over admitted truth.
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
    ) -> Result<RouteMaintenanceResult, RouteError>;

    /// Periodic consistency sweep: expire leases, detect stale routes.
    fn anti_entropy_tick(&mut self) -> Result<(), RouteError>;
}

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
    fn observe_route_health(
        &self,
        route_id: &RouteId,
    ) -> Result<Observation<RouteHealth>, RouteError>;
}
