//! Abstract routing traits: adaptive controller, route family, router, and control/data planes.

use jacquard_core::{
    AdaptiveRoutingProfile, Configuration, InstalledRoute, Observation, RouteAdmission,
    RouteAdmissionCheck, RouteCandidate, RouteCommitment, RouteError, RouteFamilyId, RouteHealth,
    RouteId, RouteMaintenanceResult, RouteMaintenanceTrigger, RoutingFamilyCapabilities,
    RoutingObjective, RoutingPolicyInputs,
};

/// Owns the protection-versus-connectivity decision. In a mesh-only deployment,
/// this may return a fixed profile. Richer policy comes from the embedding host.
pub trait AdaptiveRoutingController {
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        inputs: &RoutingPolicyInputs,
    ) -> AdaptiveRoutingProfile;
}

/// The family boundary. Each route family (mesh, onion, etc.) implements
/// this trait. Jacquard core interacts with families only through this surface.
pub trait RouteFamily {
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
        &mut self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;

    /// Installation is the materialization step. Success must return an
    /// `InstalledRoute` carrying a strong canonical handle.
    fn install_route(&mut self, admission: RouteAdmission) -> Result<InstalledRoute, RouteError>;

    /// Every unresolved or recently resolved family-side obligation must be
    /// expressible as an explicit route commitment.
    fn route_commitments(&self, route: &InstalledRoute) -> Vec<RouteCommitment>;

    /// Maintenance returns a typed semantic result so replacement, handoff, and
    /// failure paths keep their payload rather than collapsing to a flag.
    fn maintain_route(
        &mut self,
        route: &mut InstalledRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}

pub trait TopLevelRouter {
    fn register_family(&mut self, extension: Box<dyn RouteFamily>) -> Result<(), RouteError>;

    fn establish_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<InstalledRoute, RouteError>;

    fn route_commitments(&self, route_id: &RouteId) -> Result<Vec<RouteCommitment>, RouteError>;

    fn reselect_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<InstalledRoute, RouteError>;
}

/// Control plane owns route truth. Data plane owns forwarding over admitted truth.
pub trait RoutingControlPlane {
    fn establish_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<InstalledRoute, RouteError>;

    fn maintain_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    /// Periodic consistency sweep: expire leases, detect stale routes.
    fn anti_entropy_tick(&mut self) -> Result<(), RouteError>;
}

pub trait RoutingDataPlane {
    fn forward_payload(&mut self, route_id: &RouteId, payload: &[u8]) -> Result<(), RouteError>;

    /// Health reads are observational. They must not silently become canonical
    /// route truth without an explicit control-plane publication step.
    fn observe_route_health(
        &mut self,
        route_id: &RouteId,
    ) -> Result<Observation<RouteHealth>, RouteError>;
}
