//! Route-maintenance state machine for active mesh routes.
//!
//! Control flow: `maintain_route` snapshots router-owned state first, then
//! dispatches here on the typed maintenance trigger. This module applies the
//! concrete mesh transition: repair the remaining suffix, enter or recover
//! from partition mode, hand the route off, or escalate to replacement or
//! failure. It mutates only mesh-private runtime state and returns the shared
//! maintenance outcome.

use jacquard_core::{
    Blake3Digest, Configuration, ContentId, LinkEndpoint, MaterializedRouteIdentity,
    RouteError, RouteId, RouteLifecycleEvent, RouteMaintenanceFailure,
    RouteMaintenanceOutcome, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteProgressState, RouteRuntimeError, RouteSemanticHandoff,
};

use super::{
    super::{
        support::{route_cost_for_segments, shortest_paths},
        ActiveMeshRoute,
    },
    MaintenanceContext, MeshEffectsBounds, MeshEngine, MeshHasherBounds,
    MeshSelectorBounds, TransportEffectsBounds,
};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::MeshTopologyBounds,
    Topology::PeerEstimate: jacquard_traits::MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: jacquard_traits::MeshNeighborhoodEstimateAccess,
    Transport: TransportEffectsBounds,
    Retention: super::super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn apply_repair(
        &mut self,
        route_id: &jacquard_core::RouteId,
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        now: jacquard_core::Tick,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        active_route.repair.steps_remaining =
            active_route.repair.steps_remaining.saturating_sub(1);
        active_route.repair.last_repaired_at_tick = Some(now);
        active_route.last_lifecycle_event = RouteLifecycleEvent::Repaired;
        runtime.last_lifecycle_event = RouteLifecycleEvent::Repaired;
        runtime.progress.last_progress_at_tick = now;
        self.choreography_runtime().repair_exchange(route_id)?;
        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Repaired,
            outcome: RouteMaintenanceOutcome::Repaired,
        })
    }

    fn enter_partition_mode(
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> RouteMaintenanceResult {
        active_route.anti_entropy.partition_mode = true;
        active_route.last_lifecycle_event = RouteLifecycleEvent::EnteredPartitionMode;
        runtime.last_lifecycle_event = RouteLifecycleEvent::EnteredPartitionMode;
        runtime.progress.state = RouteProgressState::Blocked;
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::EnteredPartitionMode,
            outcome: RouteMaintenanceOutcome::HoldFallback {
                trigger,
                retained_object_count: u32::try_from(
                    active_route.anti_entropy.retained_objects.len(),
                )
                .unwrap_or(u32::MAX),
            },
        }
    }

    fn handoff_result(
        &mut self,
        identity: &MaterializedRouteIdentity,
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        handoff_receipt_id: jacquard_core::ReceiptId,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let Some(next_owner) = Self::handoff_target(active_route) else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let handoff = RouteSemanticHandoff {
            route_id: identity.handle.route_id,
            from_node_id: active_route.forwarding.current_owner_node_id,
            to_node_id: next_owner,
            handoff_epoch: active_route.current_epoch,
            receipt_id: handoff_receipt_id,
        };
        active_route.forwarding.current_owner_node_id = next_owner;
        active_route.forwarding.next_hop_index =
            active_route.forwarding.next_hop_index.saturating_add(1);
        active_route.handoff.last_receipt_id = Some(handoff_receipt_id);
        active_route.handoff.last_handoff_at_tick =
            Some(runtime.progress.last_progress_at_tick);
        active_route.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
        runtime.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
        self.choreography_runtime()
            .handoff_exchange(&identity.handle.route_id)?;
        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::HandedOff,
            outcome: RouteMaintenanceOutcome::HandedOff(handoff),
        })
    }

    fn replacement_required(
        trigger: RouteMaintenanceTrigger,
    ) -> RouteMaintenanceResult {
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::Replaced,
            outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
        }
    }

    fn route_expired_result(
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
    ) -> RouteMaintenanceResult {
        active_route.last_lifecycle_event = RouteLifecycleEvent::Expired;
        runtime.last_lifecycle_event = RouteLifecycleEvent::Expired;
        runtime.progress.state = RouteProgressState::Failed;
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::Expired,
            outcome: RouteMaintenanceOutcome::Failed(
                RouteMaintenanceFailure::LeaseExpired,
            ),
        }
    }

    fn continue_result(
        active_route: &ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        now: jacquard_core::Tick,
    ) -> RouteMaintenanceResult {
        runtime.progress.last_progress_at_tick = now;
        RouteMaintenanceResult {
            event: active_route.last_lifecycle_event,
            outcome: RouteMaintenanceOutcome::Continued,
        }
    }

    fn final_destination_node_id(
        active_route: &ActiveMeshRoute,
    ) -> Option<jacquard_core::NodeId> {
        active_route
            .path
            .segments
            .last()
            .map(|segment| segment.node_id)
    }

    fn repair_remaining_suffix(
        &self,
        active_route: &mut ActiveMeshRoute,
        topology: &Configuration,
    ) -> bool {
        let Some(destination_node_id) = Self::final_destination_node_id(active_route)
        else {
            return false;
        };
        if active_route.forwarding.current_owner_node_id == destination_node_id {
            return false;
        }

        let shortest =
            shortest_paths(&active_route.forwarding.current_owner_node_id, topology);
        let Some(node_path) = shortest.get(&destination_node_id) else {
            return false;
        };
        let Some(repaired_suffix) = self.derive_segments(topology, node_path) else {
            return false;
        };

        // Truncate to the already-traversed prefix, then graft the BFS
        // repair suffix onto it. Guards against a stale hop index that
        // exceeds the current segment list length.
        let suffix_start = usize::from(active_route.forwarding.next_hop_index);
        if suffix_start > active_route.path.segments.len() {
            return false;
        }
        active_route.path.segments.truncate(suffix_start);
        active_route.path.segments.extend(repaired_suffix);
        let node_path = std::iter::once(active_route.path.source)
            .chain(
                active_route
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.node_id),
            )
            .collect::<Vec<_>>();
        active_route.route_cost = route_cost_for_segments(
            &node_path,
            &active_route.path.segments,
            &active_route.path.route_class,
            topology,
        );
        true
    }

    fn flush_retained_payloads(
        &mut self,
        active_route: &mut ActiveMeshRoute,
    ) -> Result<(), RouteError> {
        let Some(next_endpoint) = self.next_replay_endpoint(active_route) else {
            return Ok(());
        };

        for object_id in self.retained_object_ids(active_route) {
            self.flush_retained_object(active_route, object_id, next_endpoint.clone())?;
        }

        Ok(())
    }

    fn next_replay_endpoint(
        &self,
        active_route: &ActiveMeshRoute,
    ) -> Option<LinkEndpoint> {
        active_route
            .path
            .segments
            .get(usize::from(active_route.forwarding.next_hop_index))
            .map(|segment| segment.endpoint.clone())
    }

    fn retained_object_ids(
        &self,
        active_route: &ActiveMeshRoute,
    ) -> Vec<ContentId<Blake3Digest>> {
        // Snapshot before iterating because successful sends remove entries
        // from `retained_objects` as the loop progresses.
        active_route
            .anti_entropy
            .retained_objects
            .iter()
            .cloned()
            .collect()
    }

    fn flush_retained_object(
        &mut self,
        active_route: &mut ActiveMeshRoute,
        object_id: ContentId<Blake3Digest>,
        next_endpoint: LinkEndpoint,
    ) -> Result<(), RouteError> {
        let Some(payload) =
            self.recover_retained_payload_for_flush(active_route, object_id)?
        else {
            active_route
                .anti_entropy
                .retained_objects
                .remove(&object_id);
            return Ok(());
        };

        self.replay_retained_payload(
            &active_route.path.route_id,
            object_id,
            next_endpoint,
            &payload,
        )?;
        active_route
            .anti_entropy
            .retained_objects
            .remove(&object_id);
        active_route.forwarding.in_flight_frames =
            active_route.forwarding.in_flight_frames.saturating_add(1);
        active_route.forwarding.last_ack_at_tick = Some(self.effects.now_tick());
        Ok(())
    }

    fn recover_retained_payload_for_flush(
        &mut self,
        active_route: &ActiveMeshRoute,
        object_id: ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RouteError> {
        self.choreography_runtime()
            .recover_held_payload(&active_route.path.route_id, &object_id)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::MaintenanceFailed))
    }

    fn replay_retained_payload(
        &mut self,
        route_id: &RouteId,
        object_id: ContentId<Blake3Digest>,
        next_endpoint: LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        if let Err(error) = self.choreography_runtime().replay_to_next_hop(
            route_id,
            object_id,
            next_endpoint,
            payload.to_vec(),
        ) {
            // Best-effort re-retain: the replay send failed; try to keep the
            // payload for the next flush. The primary RouteError is
            // returned below regardless.
            let _ = self
                .choreography_runtime()
                .retain_for_replay(route_id, object_id, payload);
            return Err(error);
        }
        Ok(())
    }

    fn recover_from_partition_if_possible(
        &mut self,
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        now: jacquard_core::Tick,
    ) -> Result<Option<RouteMaintenanceResult>, RouteError> {
        if !active_route.anti_entropy.partition_mode {
            return Ok(None);
        }

        self.flush_retained_payloads(active_route)?;
        active_route.anti_entropy.partition_mode = false;
        active_route.anti_entropy.last_refresh_at_tick = Some(now);
        active_route.last_lifecycle_event = RouteLifecycleEvent::RecoveredFromPartition;
        runtime.last_lifecycle_event = RouteLifecycleEvent::RecoveredFromPartition;
        runtime.progress.last_progress_at_tick = now;
        runtime.progress.state = RouteProgressState::Satisfied;
        Ok(Some(RouteMaintenanceResult {
            event: RouteLifecycleEvent::RecoveredFromPartition,
            outcome: RouteMaintenanceOutcome::Continued,
        }))
    }

    pub(super) fn apply_maintenance_trigger(
        &mut self,
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
        context: &MaintenanceContext<'_>,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        match trigger {
            | RouteMaintenanceTrigger::LinkDegraded
            | RouteMaintenanceTrigger::EpochAdvanced => {
                self.handle_repair_trigger(active_route, runtime, trigger, context)
            },
            | RouteMaintenanceTrigger::CapacityExceeded => {
                Ok(Self::replacement_required(trigger))
            },
            | RouteMaintenanceTrigger::PartitionDetected => {
                Ok(Self::enter_partition_mode(active_route, runtime, trigger))
            },
            | RouteMaintenanceTrigger::PolicyShift => {
                // Flush retained payloads before advancing the owner:
                // after handoff current_owner_node_id changes, making
                // the endpoint used by flush_retained_payloads wrong.
                self.flush_retained_payloads(active_route)?;
                self.handoff_result(
                    context.identity,
                    active_route,
                    runtime,
                    context.handoff_receipt_id,
                )
            },
            | RouteMaintenanceTrigger::LeaseExpiring => {
                Ok(Self::replacement_required(trigger))
            },
            | RouteMaintenanceTrigger::RouteExpired => {
                Ok(Self::route_expired_result(active_route, runtime))
            },
            | RouteMaintenanceTrigger::AntiEntropyRequired => {
                self.consume_anti_entropy_pressure(context.now);
                if let Some(recovered) = self.recover_from_partition_if_possible(
                    active_route,
                    runtime,
                    context.now,
                )? {
                    Ok(recovered)
                } else {
                    Ok(Self::continue_result(active_route, runtime, context.now))
                }
            },
        }
    }

    fn handle_repair_trigger(
        &mut self,
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
        context: &MaintenanceContext<'_>,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        if !self.repair_allowed(active_route) {
            return Ok(Self::replacement_required(trigger));
        }
        let Some(topology) = context.latest_topology else {
            return Ok(Self::replacement_required(trigger));
        };
        let repaired = self.repair_remaining_suffix(active_route, &topology.value);
        if !repaired {
            return Ok(Self::replacement_required(trigger));
        }
        active_route.current_epoch = topology.value.epoch;
        let result = self.apply_repair(
            &context.identity.handle.route_id,
            active_route,
            runtime,
            context.now,
        )?;
        if let Some(recovered) =
            self.recover_from_partition_if_possible(active_route, runtime, context.now)?
        {
            Ok(recovered)
        } else {
            Ok(result)
        }
    }
}
