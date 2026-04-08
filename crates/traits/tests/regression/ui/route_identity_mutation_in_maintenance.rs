use jacquard_traits::{
    jacquard_core::{
        AdaptiveRoutingProfile, Configuration, MaterializedRoute, MaterializedRouteIdentity,
        Observation, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCommitment,
        RouteError, RouteHandle, RouteId, RouteInstallation, RouteMaintenanceResult,
        RouteMaintenanceTrigger, RouteMaterializationInput, RouteRuntimeState,
        RoutingEngineCapabilities, RoutingEngineId, RoutingObjective,
    },
    RoutingEngine, RoutingEnginePlanner,
};

struct BadEngine;

impl RoutingEnginePlanner for BadEngine {
    fn engine_id(&self) -> RoutingEngineId {
        todo!()
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        todo!()
    }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        todo!()
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _candidate: &RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        todo!()
    }

    fn admit_route(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _candidate: RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        todo!()
    }
}

impl RoutingEngine for BadEngine {
    fn materialize_route(
        &mut self,
        _input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        todo!()
    }

    fn route_commitments(&self, _route: &MaterializedRoute) -> Vec<RouteCommitment> {
        todo!()
    }

    fn maintain_route(
        &mut self,
        identity: &MaterializedRouteIdentity,
        _runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        identity.handle = RouteHandle {
            route_id: RouteId([0; 16]),
            topology_epoch: identity.handle.topology_epoch,
            materialized_at_tick: identity.handle.materialized_at_tick,
            publication_id: identity.handle.publication_id,
        };
        todo!()
    }

    fn teardown(&mut self, _route_id: &RouteId) {}
}

fn main() {}
