//! Abstract routing traits: adaptive controller, family extension, router, and control/data planes.

use contour_core::{
    AdaptiveRoutingProfile, InstalledRoute, RouteAdmission, RouteAdmissionCheck, RouteCandidate,
    RouteError, RouteHealth, RouteId, RouteMaintenanceDisposition, RouteMaintenanceTrigger,
    RouteTransition, RoutingFamilyCapabilities, RoutingFamilyId, RoutingObjective,
    RoutingObservations, TopologySnapshot,
};

pub trait AdaptiveRoutingController {
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        observations: &RoutingObservations,
    ) -> AdaptiveRoutingProfile;
}

pub trait RouteFamilyExtension {
    fn family_id(&self) -> RoutingFamilyId;

    fn capabilities(&self) -> RoutingFamilyCapabilities;

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &TopologySnapshot,
    ) -> Vec<RouteCandidate>;

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

    fn install_route(&mut self, admission: RouteAdmission) -> Result<InstalledRoute, RouteError>;

    fn maintain_route(
        &mut self,
        route: &mut InstalledRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceDisposition, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}

pub trait TopLevelRouter {
    fn register_family(
        &mut self,
        extension: Box<dyn RouteFamilyExtension>,
    ) -> Result<(), RouteError>;

    fn establish_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<InstalledRoute, RouteError>;

    fn reselect_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<InstalledRoute, RouteError>;
}

pub trait RoutingControlPlane {
    fn establish_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<InstalledRoute, RouteError>;

    fn maintain_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteTransition, RouteError>;

    fn anti_entropy_tick(&mut self) -> Result<(), RouteError>;
}

pub trait RoutingDataPlane {
    fn forward_payload(&mut self, route_id: &RouteId, payload: &[u8]) -> Result<(), RouteError>;

    fn observe_route_health(&mut self, route_id: &RouteId) -> Result<RouteHealth, RouteError>;
}
