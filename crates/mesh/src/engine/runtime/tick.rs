//! Engine tick, route commitments, and lease-expiry helpers for mesh runtime.

use jacquard_core::{
    MaterializedRoute, MaterializedRouteIdentity, RouteBinding, RouteCommitment,
    RouteCommitmentResolution, RouteError, RouteEvent, RouteInvalidationReason,
    RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteOperationId, RouteProgressState, RouteRuntimeError,
    RoutingTickChange, RoutingTickContext, RoutingTickOutcome, TimeoutPolicy,
};

use super::{
    super::{
        support::topology_epoch_storage_key, MESH_COMMITMENT_ATTEMPT_COUNT_MAX,
        MESH_COMMITMENT_BACKOFF_MS_MAX, MESH_COMMITMENT_INITIAL_BACKOFF_MS,
        MESH_COMMITMENT_OVERALL_TIMEOUT_MS,
    },
    MeshEffectsBounds, MeshEngine, MeshHasherBounds, MeshSelectorBounds,
    MeshTransportBounds,
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
            event:   RouteLifecycleEvent::Expired,
            outcome: RouteMaintenanceOutcome::Failed(
                RouteMaintenanceFailure::LeaseExpired,
            ),
        };
        self.record_event(RouteEvent::RouteMaintenanceCompleted {
            route_id: identity.handle.route_id,
            result:   result.clone(),
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
            RouteCommitmentResolution::Invalidated(
                RouteInvalidationReason::LeaseExpired,
            )
        };
        vec![RouteCommitment {
            commitment_id: self
                .commitment_id_for_route(&route.identity.handle.route_id),
            // OperationId reuses RouteId bytes directly; the route id is
            // already a stable content address for this path, so no
            // separate derivation is needed.
            operation_id: RouteOperationId(route.identity.handle.route_id.0),
            route_binding: RouteBinding::Bound(route.identity.handle.route_id),
            owner_node_id: route.identity.lease.owner_node_id,
            deadline_tick: route.identity.lease.valid_for.end_tick(),
            retry_policy: TimeoutPolicy {
                attempt_count_max:           MESH_COMMITMENT_ATTEMPT_COUNT_MAX,
                initial_backoff_ms:          jacquard_core::DurationMs(
                    MESH_COMMITMENT_INITIAL_BACKOFF_MS,
                ),
                backoff_multiplier_permille: jacquard_core::RatioPermille(1000),
                backoff_ms_max:              jacquard_core::DurationMs(
                    MESH_COMMITMENT_BACKOFF_MS_MAX,
                ),
                overall_timeout_ms:          jacquard_core::DurationMs(
                    MESH_COMMITMENT_OVERALL_TIMEOUT_MS,
                ),
            },
            resolution,
        }]
    }

    pub(super) fn engine_tick_inner(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, RouteError> {
        let topology = &tick.topology;
        let prior_topology_epoch =
            self.latest_topology.as_ref().map(|seen| seen.value.epoch);
        let prior_transport_summary = self.last_transport_summary.clone();
        let prior_control_state = self.control_state.clone();
        let prior_checkpointed_epoch = self.last_checkpointed_topology_epoch;
        let had_cached_candidates = !self.candidate_cache.borrow().is_empty();

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
        self.control_state = Some(
            self.next_control_state(topology, self.last_transport_summary.as_ref()),
        );
        // Include had_cached_candidates: a tick that clears a non-empty
        // cache invalidates prior plan tokens and must be reported as a
        // state update even when topology and transport are unchanged.
        let change = if prior_topology_epoch != Some(topology.value.epoch)
            || prior_transport_summary != self.last_transport_summary
            || prior_control_state != self.control_state
            || prior_checkpointed_epoch != self.last_checkpointed_topology_epoch
            || had_cached_candidates
        {
            RoutingTickChange::PrivateStateUpdated
        } else {
            RoutingTickChange::NoChange
        };
        Ok(RoutingTickOutcome {
            topology_epoch: topology.value.epoch,
            change,
        })
    }
}
