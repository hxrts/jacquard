use jacquard_core::{
    NodeId, PublishedRouteRecord, RouteCommitment, RouteError, RouteId,
    RouteInstallation, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteMaterializationInput, RouteRuntimeError, RouteRuntimeState,
    RouteSelectionError, RoutingTickChange, RoutingTickContext, RoutingTickHint,
    RoutingTickOutcome, Tick,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine};

use crate::FieldEngine;

impl<Transport, Effects> RoutingEngine for FieldEngine<Transport, Effects> {
    fn materialize_route(
        &mut self,
        _input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        Err(RouteSelectionError::NoCandidate.into())
    }

    fn route_commitments(
        &self,
        _route: &jacquard_core::MaterializedRoute,
    ) -> Vec<RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, RouteError> {
        let changed = self.latest_topology.as_ref() != Some(&tick.topology);
        self.latest_topology = Some(tick.topology.clone());
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: if changed {
                RoutingTickChange::PrivateStateUpdated
            } else {
                RoutingTickChange::NoChange
            },
            next_tick_hint: if changed {
                RoutingTickHint::Immediate
            } else {
                RoutingTickHint::WithinTicks(Tick(1))
            },
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

    fn teardown(&mut self, route_id: &RouteId) {
        self.active_routes.remove(route_id);
    }
}

impl<Transport, Effects> RouterManagedEngine for FieldEngine<Transport, Effects>
where
    Transport: jacquard_traits::TransportSenderEffects,
{
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
        route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        Ok(self.active_routes.contains_key(route_id))
    }
}
