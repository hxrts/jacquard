//! `RoutingEngine` and `MeshRoutingEngine` implementations for `MeshEngine`.
//!
//! Materialization enforces the active-route budget, verifies lease
//! validity, assembles the mesh-private `ActiveMeshRoute` record, and
//! checkpoints it before recording a lifecycle event. Maintenance
//! dispatches per `RouteMaintenanceTrigger` into small helpers that
//! each return a typed `RouteMaintenanceResult`. `engine_tick` is the
//! engine-internal middleware loop that refreshes the latest topology,
//! clears stale candidate-cache entries, checkpoints the current epoch,
//! and polls transport ingress.

mod health;
mod maintenance;
mod materialization;
mod tick;

use jacquard_core::{
    Configuration, MaterializedRoute, MaterializedRouteIdentity, Observation,
    RouteCommitment, RouteError, RouteId, RouteInstallation, RouteMaintenanceResult,
    RouteMaintenanceTrigger, RouteMaterializationInput, RouteSelectionError,
    RoutingTickContext, RoutingTickOutcome,
};
use jacquard_traits::{CommitteeCoordinatedEngine, MeshRoutingEngine, RoutingEngine};

use super::{
    MeshEffectsBounds, MeshEngine, MeshHasherBounds, MeshSelectorBounds,
    TransportEffectsBounds,
};
use crate::{MeshNeighborhoodEstimateAccess, MeshPeerEstimateAccess};

struct MaintenanceContext<'a> {
    identity: &'a MaterializedRouteIdentity,
    now: jacquard_core::Tick,
    handoff_receipt_id: jacquard_core::ReceiptId,
    latest_topology: Observation<Configuration>,
}
impl<Topology, Transport, Retention, Effects, Hasher, Selector> RoutingEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
    Transport: TransportEffectsBounds,
    Retention: super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        self.materialize_route_inner(&input)
    }

    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment> {
        self.route_commitments_inner(route)
    }

    fn engine_tick(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, RouteError> {
        self.engine_tick_inner(tick)
    }

    fn maintain_route(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        self.maintain_route_inner(identity, runtime, trigger)
    }

    fn teardown(&mut self, route_id: &RouteId) {
        // Checkpoint removal is best-effort during teardown: the route
        // is going away regardless, and leaving stale bytes behind is
        // less harmful than refusing to drop the in-memory active
        // route. v1 mesh does not reconcile orphaned checkpoints later;
        // hosts that care about storage hygiene must sweep them out of band.
        let _ = self.choreography_runtime().clear_route_protocols(route_id);
        let _ = self.remove_checkpoint(route_id);
        self.active_routes.remove(route_id);
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
    Transport: TransportEffectsBounds,
    Retention: super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn maintain_route_inner(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let now = self.effects.now_tick();
        let handoff_receipt_id = self.receipt_id_for_route(&identity.stamp.route_id);
        if !identity.lease.is_valid_at(now) {
            return self.expired_lease_result(identity, runtime);
        }
        // Maintenance requires an observed topology. Without one the route
        // cannot be re-evaluated, so the only safe result is replacement.
        let Some(latest_topology) = self.latest_topology.clone() else {
            return Ok(jacquard_core::RouteMaintenanceResult {
                event: jacquard_core::RouteLifecycleEvent::Replaced,
                outcome: jacquard_core::RouteMaintenanceOutcome::ReplacementRequired {
                    trigger,
                },
            });
        };

        let original_active_route = self
            .active_routes
            .get(&identity.stamp.route_id)
            .cloned()
            .ok_or(RouteSelectionError::NoCandidate)?;
        let mut next_active_route = original_active_route.clone();
        let mut next_runtime = runtime.clone();
        let result = self.apply_maintenance_trigger(
            &mut next_active_route,
            &mut next_runtime,
            trigger,
            &MaintenanceContext {
                identity,
                now,
                handoff_receipt_id,
                latest_topology,
            },
        )?;

        next_runtime.health = self.current_route_health(Some(&next_active_route), now);
        self.store_checkpoint(&next_active_route)?;
        self.active_routes
            .insert(identity.stamp.route_id, next_active_route);
        *runtime = next_runtime;
        Ok(result)
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> MeshRoutingEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
    Transport: TransportEffectsBounds,
    Retention: super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    type Retention = Retention;
    type TopologyModel = Topology;

    fn topology_model(&self) -> &Self::TopologyModel {
        &self.topology_model
    }

    fn retention_store(&self) -> &Self::Retention {
        &self.retention
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    CommitteeCoordinatedEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Selector: MeshSelectorBounds,
{
    type Selector = Selector;

    fn committee_selector(&self) -> Option<&Self::Selector> {
        Some(&self.selector)
    }
}
