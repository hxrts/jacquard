//! `NullCandidateEngine` — a routing engine stub that never produces
//! candidates, useful for testing the router's handling of engines that
//! have no route opinions.

use jacquard_core::{
    Configuration, ConnectivityPosture, Observation, RouteProtectionClass,
    RouteRepairClass, RoutingObjective, SelectedRoutingParameters,
};

pub(crate) struct NullCandidateEngine {
    local_node_id: jacquard_core::NodeId,
    engine_id: jacquard_core::RoutingEngineId,
}

impl NullCandidateEngine {
    pub(crate) fn new(
        local_node_id: jacquard_core::NodeId,
        engine_id: jacquard_core::RoutingEngineId,
    ) -> Self {
        Self { local_node_id, engine_id }
    }
}

impl jacquard_traits::RoutingEnginePlanner for NullCandidateEngine {
    fn engine_id(&self) -> jacquard_core::RoutingEngineId {
        self.engine_id.clone()
    }

    fn capabilities(&self) -> jacquard_core::RoutingEngineCapabilities {
        jacquard_core::RoutingEngineCapabilities {
            engine: self.engine_id.clone(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_core::HoldSupport::Unsupported,
            decidable_admission: jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: jacquard_core::RouteShapeVisibility::NextHopOnly,
        }
    }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<jacquard_core::RouteCandidate> {
        Vec::new()
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &jacquard_core::RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmissionCheck, jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn admit_route(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: jacquard_core::RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmission, jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }
}

impl jacquard_traits::RoutingEngine for NullCandidateEngine {
    fn materialize_route(
        &mut self,
        _input: jacquard_core::RouteMaterializationInput,
    ) -> Result<jacquard_core::RouteInstallation, jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn route_commitments(
        &self,
        _route: &jacquard_core::MaterializedRoute,
    ) -> Vec<jacquard_core::RouteCommitment> {
        Vec::new()
    }

    fn maintain_route(
        &mut self,
        _identity: &jacquard_core::PublishedRouteRecord,
        _runtime: &mut jacquard_core::RouteRuntimeState,
        _trigger: jacquard_core::RouteMaintenanceTrigger,
    ) -> Result<jacquard_core::RouteMaintenanceResult, jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn teardown(&mut self, _route_id: &jacquard_core::RouteId) {}
}

impl jacquard_traits::RouterManagedEngine for NullCandidateEngine {
    fn local_node_id_for_router(&self) -> jacquard_core::NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        _route_id: &jacquard_core::RouteId,
        _payload: &[u8],
    ) -> Result<(), jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &jacquard_core::RouteId,
    ) -> Result<bool, jacquard_core::RouteError> {
        Ok(false)
    }
}
