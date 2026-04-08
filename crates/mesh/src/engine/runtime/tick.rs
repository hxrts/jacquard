//! Engine-wide progress, commitment exposure, and router-facing sweep helpers.
//!
//! Control flow: the router drives `engine_tick`, which refreshes mesh's
//! private control state from the latest tick context and returns a small
//! shared outcome. The same module also exposes current route commitments and
//! lease-expiry helpers so the router can perform canonical sweeps without
//! reading mesh-private internals.

use jacquard_core::{
    MaterializedRoute, MaterializedRouteIdentity, RouteBinding, RouteCommitment,
    RouteCommitmentResolution, RouteError, RouteEvent, RouteInvalidationReason,
    RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteOperationId, RouteProgressState, RouteRuntimeError,
    RoutingTickChange, RoutingTickContext, RoutingTickOutcome, TimeoutPolicy,
};

use super::{
    super::{
        support::topology_epoch_storage_key, ActiveMeshRoute, MeshRouteClass,
        MESH_COMMITMENT_ATTEMPT_COUNT_MAX, MESH_COMMITMENT_BACKOFF_MS_MAX,
        MESH_COMMITMENT_INITIAL_BACKOFF_MS, MESH_COMMITMENT_OVERALL_TIMEOUT_MS,
    },
    MeshEffectsBounds, MeshEngine, MeshHasherBounds, MeshSelectorBounds,
    MeshTransportBounds,
};
use crate::choreography::{
    MeshAntiEntropySnapshot, MeshNeighborAdvertisementSnapshot, MeshRouteExportSnapshot,
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
        let observations = self
            .choreography_runtime()
            .poll_tick_ingress(topology.value.epoch)?;
        self.last_transport_summary = Self::next_transport_summary(
            self.last_transport_summary.as_ref(),
            Self::summarize_transport_observations(&observations),
            topology.observed_at_tick,
        );
        self.control_state = Some(
            self.next_control_state(topology, self.last_transport_summary.as_ref()),
        );
        self.emit_cooperative_tick_protocols(topology)?;
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

    fn emit_cooperative_tick_protocols(
        &mut self,
        topology: &jacquard_core::Observation<jacquard_core::Configuration>,
    ) -> Result<(), RouteError> {
        let neighbor_snapshot = self.neighbor_advertisement_snapshot(topology);
        let route_exports = self
            .active_routes
            .iter()
            .map(|(route_id, active_route)| {
                (*route_id, self.route_export_snapshot(active_route))
            })
            .collect::<Vec<_>>();
        let anti_entropy_snapshots = self
            .active_routes
            .iter()
            .filter_map(|(route_id, active_route)| {
                self.anti_entropy_snapshot(active_route)
                    .map(|snapshot| (*route_id, snapshot))
            })
            .collect::<Vec<_>>();

        self.choreography_runtime()
            .neighbor_advertisement_exchange(
                topology.value.epoch,
                &neighbor_snapshot,
            )?;
        for (route_id, snapshot) in route_exports {
            self.choreography_runtime()
                .route_export_exchange(&route_id, &snapshot)?;
        }
        for (route_id, snapshot) in anti_entropy_snapshots {
            self.choreography_runtime()
                .anti_entropy_exchange(&route_id, &snapshot)?;
        }
        Ok(())
    }

    fn neighbor_advertisement_snapshot(
        &self,
        topology: &jacquard_core::Observation<jacquard_core::Configuration>,
    ) -> MeshNeighborAdvertisementSnapshot {
        let service_count = topology
            .value
            .nodes
            .get(&self.local_node_id)
            .map(|node| u32::try_from(node.profile.services.len()).unwrap_or(u32::MAX))
            .unwrap_or(0);
        let adjacent_neighbor_count = u32::try_from(
            self.topology_model
                .neighboring_nodes(&self.local_node_id, &topology.value)
                .len(),
        )
        .unwrap_or(u32::MAX);
        MeshNeighborAdvertisementSnapshot {
            local_node_id: self.local_node_id,
            service_count,
            adjacent_neighbor_count,
        }
    }

    fn route_export_snapshot(
        &self,
        active_route: &ActiveMeshRoute,
    ) -> MeshRouteExportSnapshot {
        MeshRouteExportSnapshot {
            route_class:    match active_route.path.route_class {
                | MeshRouteClass::Direct => "direct",
                | MeshRouteClass::MultiHop => "multi-hop",
                | MeshRouteClass::Gateway => "gateway",
                | MeshRouteClass::DeferredDelivery => "deferred-delivery",
            }
            .to_owned(),
            hop_count:      u32::try_from(active_route.path.segments.len())
                .unwrap_or(u32::MAX),
            partition_mode: active_route.anti_entropy.partition_mode,
        }
    }

    fn anti_entropy_snapshot(
        &self,
        active_route: &ActiveMeshRoute,
    ) -> Option<MeshAntiEntropySnapshot> {
        let pressure_score = self
            .control_state
            .as_ref()
            .map_or(jacquard_core::HealthScore(0), |state| {
                state.anti_entropy.pressure_score
            });
        let retained_count =
            u32::try_from(active_route.anti_entropy.retained_objects.len())
                .unwrap_or(u32::MAX);
        if !active_route.anti_entropy.partition_mode
            && retained_count == 0
            && pressure_score.0 == 0
        {
            return None;
        }
        Some(MeshAntiEntropySnapshot {
            retained_count,
            pressure_score,
            partition_mode: active_route.anti_entropy.partition_mode,
        })
    }
}
