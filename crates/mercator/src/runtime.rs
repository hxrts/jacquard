//! `RoutingEngine` and `RouterManagedEngine` impls for `MercatorEngine`.

use jacquard_core::{
    Configuration, MaterializedRoute, NodeId, Observation, PublishedRouteRecord, RouteCommitment,
    RouteError, RouteId, RouteInstallation, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteMaterializationInput, RouteRuntimeError, RouteRuntimeState, RouteSelectionError,
    RoutingTickChange, RoutingTickContext, RoutingTickHint, RoutingTickOutcome,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine};

use crate::MercatorEngine;

impl RoutingEngine for MercatorEngine {
    fn materialize_route(
        &mut self,
        _input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        Err(RouteRuntimeError::Invalidated.into())
    }

    fn route_commitments(&self, _route: &MaterializedRoute) -> Vec<RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(&mut self, tick: &RoutingTickContext) -> Result<RoutingTickOutcome, RouteError> {
        self.latest_topology_epoch = Some(tick.topology.value.epoch);
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: RoutingTickChange::NoChange,
            next_tick_hint: RoutingTickHint::WithinTicks(self.config.bounds.engine_tick_within),
        })
    }

    fn maintain_route(
        &mut self,
        _identity: &PublishedRouteRecord,
        _runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        Err(RouteRuntimeError::Invalidated.into())
    }

    fn teardown(&mut self, _route_id: &RouteId) {}
}

impl RouterManagedEngine for MercatorEngine {
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        _route_id: &RouteId,
        _payload: &[u8],
    ) -> Result<(), RouteError> {
        Err(RouteSelectionError::NoCandidate.into())
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        Ok(false)
    }

    fn restore_route_runtime_with_record_for_router(
        &mut self,
        _route: &MaterializedRoute,
        topology: &Observation<Configuration>,
    ) -> Result<bool, RouteError> {
        self.latest_topology_epoch = Some(topology.value.epoch);
        Ok(false)
    }
}
