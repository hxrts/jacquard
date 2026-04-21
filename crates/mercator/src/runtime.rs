//! `RoutingEngine` and `RouterManagedEngine` impls for `MercatorEngine`.

use jacquard_core::{
    Configuration, MaterializedRoute, NodeId, Observation, PublishedRouteRecord, RouteCommitment,
    RouteError, RouteId, RouteInstallation, RouteLifecycleEvent, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput, RouteRuntimeError,
    RouteRuntimeState, RouteSelectionError, RoutingTickChange, RoutingTickContext, RoutingTickHint,
    RoutingTickOutcome,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine};

use crate::{corridor, MercatorEngine, MERCATOR_ENGINE_ID};

impl RoutingEngine for MercatorEngine {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        let route_id = *input.handle.route_id();
        let backend_route_id = input.admission.backend_ref.backend_route_id.clone();
        let active = corridor::active_route_from_backend(backend_route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        let installation = corridor::materialize_admitted(input)?;
        self.active_routes.insert(route_id, active);
        Ok(installation)
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
        identity: &PublishedRouteRecord,
        _runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        if !self.active_routes.contains_key(identity.route_id()) {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Activated,
            outcome: RouteMaintenanceOutcome::Continued,
        })
    }

    fn teardown(&mut self, route_id: &RouteId) {
        self.active_routes.remove(route_id);
    }
}

impl RouterManagedEngine for MercatorEngine {
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        _payload: &[u8],
    ) -> Result<(), RouteError> {
        if self.active_routes.contains_key(route_id) {
            Ok(())
        } else {
            Err(RouteSelectionError::NoCandidate.into())
        }
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        Ok(false)
    }

    fn restore_route_runtime_with_record_for_router(
        &mut self,
        route: &MaterializedRoute,
        topology: &Observation<Configuration>,
    ) -> Result<bool, RouteError> {
        if route.identity.admission.backend_ref.engine != MERCATOR_ENGINE_ID {
            return Ok(false);
        }
        let Some(active) = corridor::active_route_from_backend(
            route
                .identity
                .admission
                .backend_ref
                .backend_route_id
                .clone(),
        ) else {
            return Ok(false);
        };
        self.latest_topology_epoch = Some(topology.value.epoch);
        self.active_routes
            .insert(route.identity.stamp.route_id, active);
        Ok(true)
    }

    fn analysis_snapshot_for_router(
        &self,
        _active_routes: &[MaterializedRoute],
    ) -> Option<Box<dyn std::any::Any>> {
        Some(Box::new(self.router_analysis_snapshot()))
    }
}
