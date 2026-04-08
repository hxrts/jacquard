use jacquard_traits::{
    jacquard_core::{
        SelectedRoutingParameters, Configuration, MaterializedRoute, PublishedRouteRecord,
        Observation, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCommitment,
        RouteError, RouteId, RouteInstallation, RouteMaintenanceResult,
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
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        todo!()
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        todo!()
    }

    fn admit_route(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
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
        identity: &PublishedRouteRecord,
        _runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        identity.stamp = jacquard_traits::jacquard_core::RouteIdentityStamp {
            route_id: RouteId([0; 16]),
            topology_epoch: identity.stamp.topology_epoch,
            materialized_at_tick: identity.stamp.materialized_at_tick,
            publication_id: identity.stamp.publication_id,
        };
        todo!()
    }

    fn teardown(&mut self, _route_id: &RouteId) {}
}

fn main() {}
