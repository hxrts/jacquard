//! Engine tick, route commitments, and lease-expiry helpers for mesh runtime.

use jacquard_core::{
    Configuration, MaterializedRoute, MaterializedRouteIdentity, Observation, RouteBinding,
    RouteCommitment, RouteCommitmentResolution, RouteError, RouteEvent, RouteInvalidationReason,
    RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceResult,
    RouteOperationId, RouteProgressState, RouteRuntimeError, TimeoutPolicy,
};

use super::super::{
    support::topology_epoch_storage_key, MESH_COMMITMENT_ATTEMPT_COUNT_MAX,
    MESH_COMMITMENT_BACKOFF_MS_MAX, MESH_COMMITMENT_INITIAL_BACKOFF_MS,
    MESH_COMMITMENT_OVERALL_TIMEOUT_MS,
};
use super::{
    MeshEffectsBounds, MeshEngine, MeshHasherBounds, MeshSelectorBounds, MeshTransportBounds,
};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::MeshTopologyBounds,
    Topology::PeerEstimate: jacquard_traits::MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: jacquard_traits::MeshNeighborhoodEstimateAccess,
    Transport: MeshTransportBounds,
    Retention: super::super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    pub(super) fn expired_lease_result(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut jacquard_core::RouteRuntimeState,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let mut next_runtime = runtime.clone();
        next_runtime.last_lifecycle_event = RouteLifecycleEvent::Expired;
        next_runtime.progress.state = RouteProgressState::Failed;
        let result = RouteMaintenanceResult {
            event: RouteLifecycleEvent::Expired,
            outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired),
        };
        self.record_event(RouteEvent::RouteMaintenanceCompleted {
            route_id: identity.handle.route_id,
            result: result.clone(),
        })?;
        *runtime = next_runtime;
        Ok(result)
    }

    pub(super) fn route_commitments_inner(
        &self,
        route: &MaterializedRoute,
    ) -> Vec<RouteCommitment> {
        let resolution = if route.identity.lease.is_valid_at(self.effects.now_tick()) {
            RouteCommitmentResolution::Pending
        } else {
            RouteCommitmentResolution::Invalidated(RouteInvalidationReason::LeaseExpired)
        };
        vec![RouteCommitment {
            commitment_id: self.commitment_id_for_route(&route.identity.handle.route_id),
            operation_id: RouteOperationId(route.identity.handle.route_id.0),
            route_binding: RouteBinding::Bound(route.identity.handle.route_id),
            owner_node_id: route.identity.lease.owner_node_id,
            deadline_tick: route.identity.lease.valid_for.end_tick(),
            retry_policy: TimeoutPolicy {
                attempt_count_max: MESH_COMMITMENT_ATTEMPT_COUNT_MAX,
                initial_backoff_ms: jacquard_core::DurationMs(MESH_COMMITMENT_INITIAL_BACKOFF_MS),
                backoff_multiplier_permille: jacquard_core::RatioPermille(1000),
                backoff_ms_max: jacquard_core::DurationMs(MESH_COMMITMENT_BACKOFF_MS_MAX),
                overall_timeout_ms: jacquard_core::DurationMs(MESH_COMMITMENT_OVERALL_TIMEOUT_MS),
            },
            resolution,
        }]
    }

    pub(super) fn engine_tick_inner(
        &mut self,
        topology: &Observation<Configuration>,
    ) -> Result<(), RouteError> {
        self.latest_topology = Some(topology.clone());
        self.candidate_cache.borrow_mut().clear();
        if self.last_checkpointed_topology_epoch != Some(topology.value.epoch) {
            let epoch_bytes = topology.value.epoch.0.to_le_bytes();
            let epoch_key = topology_epoch_storage_key(&self.local_node_id);
            self.effects
                .store_bytes(&epoch_key, &epoch_bytes)
                .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))?;
            self.last_checkpointed_topology_epoch = Some(topology.value.epoch);
        }
        let observations = self.transport.poll_observations()?;
        self.last_transport_summary = Self::next_transport_summary(
            self.last_transport_summary.as_ref(),
            Self::summarize_transport_observations(&observations),
            topology.observed_at_tick,
        );
        self.control_state =
            Some(self.next_control_state(topology, self.last_transport_summary.as_ref()));
        Ok(())
    }
}
