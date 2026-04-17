//! Route-maintenance state machine for active pathway routes.
//!
//! Control flow: `maintain_route` snapshots router-owned state first, then
//! dispatches here on the typed maintenance trigger. This module applies the
//! concrete pathway transition: repair the remaining suffix, enter or recover
//! from partition mode, hand the route off, or escalate to replacement or
//! failure. It mutates only pathway-private runtime state and returns the
//! shared maintenance outcome.

use jacquard_core::{
    Blake3Digest, Configuration, ContentId, LinkEndpoint, RouteError, RouteId, RouteLifecycleEvent,
    RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceResult,
    RouteMaintenanceTrigger, RouteProgressState, RouteRuntimeError, RouteRuntimeState,
    RouteSemanticHandoff, Tick,
};

use super::{
    super::{
        support::{current_segment, route_cost_for_segments, shortest_paths},
        ActivePathwayRoute, MaintenanceResultExt,
    },
    MaintenanceContext, PathwayEffectsBounds, PathwayEngine, PathwayHasherBounds,
    PathwaySelectorBounds, PathwayTransportBounds,
};
use crate::{choreography, PathwayNeighborhoodEstimateAccess, PathwayPeerEstimateAccess};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RetainedFlushPhase {
    BeforeProjection,
    AfterProjection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MaintenanceProtocolAction {
    RepairExchange,
    HandoffExchange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NormalizedMaintenanceInput {
    pub(super) trigger: RouteMaintenanceTrigger,
    pub(super) now: Tick,
    pub(super) handoff_receipt_id: jacquard_core::ReceiptId,
    pub(super) latest_topology_epoch: jacquard_core::RouteEpoch,
    pub(super) repaired_active_route: Option<ActivePathwayRoute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MaintenanceTransition {
    pub(super) next_active_route: ActivePathwayRoute,
    pub(super) next_runtime: RouteRuntimeState,
    pub(super) result: RouteMaintenanceResult,
    consume_anti_entropy_pressure: bool,
    flush_retained_payloads: Option<RetainedFlushPhase>,
    protocol_action: Option<MaintenanceProtocolAction>,
}

fn apply_enter_partition_mode(
    active_route: &mut ActivePathwayRoute,
    runtime: &mut RouteRuntimeState,
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
            retained_object_count: jacquard_core::HoldItemCount(
                u32::try_from(active_route.anti_entropy.retained_objects.len()).unwrap_or(u32::MAX),
            ),
        },
    }
}

fn replacement_required(trigger: RouteMaintenanceTrigger) -> RouteMaintenanceResult {
    RouteMaintenanceResult {
        event: RouteLifecycleEvent::Replaced,
        outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
    }
}

fn apply_route_expired(
    active_route: &mut ActivePathwayRoute,
    runtime: &mut RouteRuntimeState,
) -> RouteMaintenanceResult {
    active_route.last_lifecycle_event = RouteLifecycleEvent::Expired;
    runtime.last_lifecycle_event = RouteLifecycleEvent::Expired;
    runtime.progress.state = RouteProgressState::Failed;
    RouteMaintenanceResult {
        event: RouteLifecycleEvent::Expired,
        outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired),
    }
}

fn apply_continue(
    active_route: &ActivePathwayRoute,
    runtime: &mut RouteRuntimeState,
    now: Tick,
) -> RouteMaintenanceResult {
    runtime.progress.last_progress_at_tick = now;
    RouteMaintenanceResult {
        event: active_route.last_lifecycle_event,
        outcome: RouteMaintenanceOutcome::Continued,
    }
}

fn handoff_target(active_route: &ActivePathwayRoute) -> Option<jacquard_core::NodeId> {
    active_route
        .path
        .segments
        .get(usize::from(active_route.forwarding.next_hop_index))
        .map(|segment| segment.node_id)
}

// long-block-exception: pathway maintenance reduction preserves the full
// repair, handoff, partition, and anti-entropy transition ladder in one place.
pub(super) fn reduce_maintenance_transition(
    active_route: &ActivePathwayRoute,
    runtime: &RouteRuntimeState,
    input: &NormalizedMaintenanceInput,
) -> Result<MaintenanceTransition, RouteError> {
    match input.trigger {
        RouteMaintenanceTrigger::LinkDegraded | RouteMaintenanceTrigger::EpochAdvanced => {
            let Some(mut next_active_route) = input.repaired_active_route.clone() else {
                return Ok(MaintenanceTransition {
                    next_active_route: active_route.clone(),
                    next_runtime: runtime.clone(),
                    result: replacement_required(input.trigger),
                    consume_anti_entropy_pressure: false,
                    flush_retained_payloads: None,
                    protocol_action: None,
                });
            };
            let mut next_runtime = runtime.clone();
            let topology_advanced = next_active_route.current_epoch != input.latest_topology_epoch;
            next_active_route.current_epoch = input.latest_topology_epoch;
            if !matches!(input.trigger, RouteMaintenanceTrigger::EpochAdvanced)
                || !topology_advanced
            {
                next_active_route.repair.steps_remaining =
                    next_active_route.repair.steps_remaining.saturating_sub(1);
            }
            next_active_route.repair.last_repaired_at_tick = Some(input.now);
            next_runtime.progress.last_progress_at_tick = input.now;
            let (result, flush_retained_payloads) = if active_route.is_in_partition_mode() {
                next_active_route.anti_entropy.partition_mode = false;
                next_active_route.anti_entropy.last_refresh_at_tick = Some(input.now);
                next_active_route.last_lifecycle_event =
                    RouteLifecycleEvent::RecoveredFromPartition;
                next_runtime.last_lifecycle_event = RouteLifecycleEvent::RecoveredFromPartition;
                next_runtime.progress.state = RouteProgressState::Satisfied;
                (
                    RouteMaintenanceResult {
                        event: RouteLifecycleEvent::RecoveredFromPartition,
                        outcome: RouteMaintenanceOutcome::Continued,
                    },
                    Some(RetainedFlushPhase::AfterProjection),
                )
            } else {
                next_active_route.last_lifecycle_event = RouteLifecycleEvent::Repaired;
                next_runtime.last_lifecycle_event = RouteLifecycleEvent::Repaired;
                (
                    RouteMaintenanceResult {
                        event: RouteLifecycleEvent::Repaired,
                        outcome: RouteMaintenanceOutcome::Repaired,
                    },
                    None,
                )
            };
            Ok(MaintenanceTransition {
                next_active_route,
                next_runtime,
                result,
                consume_anti_entropy_pressure: false,
                flush_retained_payloads,
                protocol_action: Some(MaintenanceProtocolAction::RepairExchange),
            })
        }
        RouteMaintenanceTrigger::CapacityExceeded | RouteMaintenanceTrigger::LeaseExpiring => {
            Ok(MaintenanceTransition {
                next_active_route: active_route.clone(),
                next_runtime: runtime.clone(),
                result: replacement_required(input.trigger),
                consume_anti_entropy_pressure: false,
                flush_retained_payloads: None,
                protocol_action: None,
            })
        }
        RouteMaintenanceTrigger::PartitionDetected => {
            let mut next_active_route = active_route.clone();
            let mut next_runtime = runtime.clone();
            let result = apply_enter_partition_mode(
                &mut next_active_route,
                &mut next_runtime,
                input.trigger,
            );
            Ok(MaintenanceTransition {
                next_active_route,
                next_runtime,
                result,
                consume_anti_entropy_pressure: false,
                flush_retained_payloads: None,
                protocol_action: None,
            })
        }
        RouteMaintenanceTrigger::PolicyShift => {
            let Some(next_owner) = handoff_target(active_route) else {
                return Err(RouteRuntimeError::Invalidated.into());
            };
            let mut next_active_route = active_route.clone();
            let mut next_runtime = runtime.clone();
            let handoff = RouteSemanticHandoff {
                route_id: active_route.path.route_id,
                from_node_id: active_route.forwarding.current_owner_node_id,
                to_node_id: next_owner,
                handoff_epoch: active_route.current_epoch,
                receipt_id: input.handoff_receipt_id,
            };
            next_active_route.forwarding.current_owner_node_id = next_owner;
            next_active_route.forwarding.next_hop_index = next_active_route
                .forwarding
                .next_hop_index
                .saturating_add(1);
            next_active_route.handoff.last_receipt_id = Some(input.handoff_receipt_id);
            next_active_route.handoff.last_handoff_at_tick =
                Some(runtime.progress.last_progress_at_tick);
            next_active_route.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
            next_runtime.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
            Ok(MaintenanceTransition {
                next_active_route,
                next_runtime,
                result: RouteMaintenanceResult {
                    event: RouteLifecycleEvent::HandedOff,
                    outcome: RouteMaintenanceOutcome::HandedOff(handoff),
                },
                consume_anti_entropy_pressure: false,
                flush_retained_payloads: Some(RetainedFlushPhase::BeforeProjection),
                protocol_action: Some(MaintenanceProtocolAction::HandoffExchange),
            })
        }
        RouteMaintenanceTrigger::RouteExpired => {
            let mut next_active_route = active_route.clone();
            let mut next_runtime = runtime.clone();
            let result = apply_route_expired(&mut next_active_route, &mut next_runtime);
            Ok(MaintenanceTransition {
                next_active_route,
                next_runtime,
                result,
                consume_anti_entropy_pressure: false,
                flush_retained_payloads: None,
                protocol_action: None,
            })
        }
        RouteMaintenanceTrigger::AntiEntropyRequired => {
            let mut next_active_route = active_route.clone();
            let mut next_runtime = runtime.clone();
            let (result, flush_retained_payloads) = if active_route.is_in_partition_mode() {
                next_active_route.anti_entropy.partition_mode = false;
                next_active_route.anti_entropy.last_refresh_at_tick = Some(input.now);
                next_active_route.last_lifecycle_event =
                    RouteLifecycleEvent::RecoveredFromPartition;
                next_runtime.last_lifecycle_event = RouteLifecycleEvent::RecoveredFromPartition;
                next_runtime.progress.last_progress_at_tick = input.now;
                next_runtime.progress.state = RouteProgressState::Satisfied;
                (
                    RouteMaintenanceResult {
                        event: RouteLifecycleEvent::RecoveredFromPartition,
                        outcome: RouteMaintenanceOutcome::Continued,
                    },
                    Some(RetainedFlushPhase::AfterProjection),
                )
            } else {
                (
                    apply_continue(active_route, &mut next_runtime, input.now),
                    None,
                )
            };
            Ok(MaintenanceTransition {
                next_active_route,
                next_runtime,
                result,
                consume_anti_entropy_pressure: true,
                flush_retained_payloads,
                protocol_action: None,
            })
        }
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::PathwayTopologyBounds,
    Topology::PeerEstimate: PathwayPeerEstimateAccess,
    Topology::NeighborhoodEstimate: PathwayNeighborhoodEstimateAccess,
    Transport: PathwayTransportBounds,
    Retention: super::super::PathwayRetentionBounds,
    Effects: PathwayEffectsBounds,
    Hasher: PathwayHasherBounds,
    Selector: PathwaySelectorBounds,
{
    fn final_destination_node_id(
        active_route: &ActivePathwayRoute,
    ) -> Option<jacquard_core::NodeId> {
        active_route
            .path
            .segments
            .last()
            .map(|segment| segment.node_id)
    }

    fn repair_remaining_suffix(
        &self,
        active_route: &mut ActivePathwayRoute,
        topology: &Configuration,
    ) -> bool {
        let Some(destination_node_id) = Self::final_destination_node_id(active_route) else {
            return false;
        };
        if active_route.forwarding.current_owner_node_id == destination_node_id {
            return false;
        }

        let shortest = shortest_paths(&active_route.forwarding.current_owner_node_id, topology);
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
        active_route: &mut ActivePathwayRoute,
    ) -> Result<(), RouteError> {
        let Some(next_endpoint) = self.next_replay_endpoint(active_route) else {
            return Ok(());
        };

        for object_id in self.retained_object_ids(active_route) {
            self.flush_retained_object(active_route, object_id, next_endpoint.clone())?;
        }

        Ok(())
    }

    fn next_replay_endpoint(&self, active_route: &ActivePathwayRoute) -> Option<LinkEndpoint> {
        current_segment(active_route).map(|segment| segment.endpoint.clone())
    }

    fn retained_object_ids(
        &self,
        active_route: &ActivePathwayRoute,
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
        active_route: &mut ActivePathwayRoute,
        object_id: ContentId<Blake3Digest>,
        next_endpoint: LinkEndpoint,
    ) -> Result<(), RouteError> {
        let Some(payload) = self.recover_retained_payload_for_flush(active_route, object_id)?
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
        let now_tick = self.current_tick();
        active_route
            .anti_entropy
            .retained_objects
            .remove(&object_id);
        active_route.forwarding.in_flight_frames =
            active_route.forwarding.in_flight_frames.saturating_add(1);
        active_route.forwarding.last_ack_at_tick = Some(now_tick);
        Ok(())
    }

    fn recover_retained_payload_for_flush(
        &mut self,
        active_route: &ActivePathwayRoute,
        object_id: ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RouteError> {
        choreography::recover_held_payload(
            &mut self.transport,
            &mut self.retention,
            &mut self.effects,
            &active_route.path.route_id,
            &object_id,
        )
        .maintenance_failed()
    }

    fn replay_retained_payload(
        &mut self,
        route_id: &RouteId,
        object_id: ContentId<Blake3Digest>,
        next_endpoint: LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        if let Err(error) = choreography::replay_to_next_hop(
            &mut self.transport,
            &mut self.retention,
            &mut self.effects,
            route_id,
            object_id,
            next_endpoint,
            payload.to_vec(),
        ) {
            // Best-effort re-retain: the replay send failed; try to keep the
            // payload for the next flush. The primary RouteError is
            // returned below regardless.
            let _re_retain_failed = choreography::retain_for_replay(
                &mut self.transport,
                &mut self.retention,
                &mut self.effects,
                route_id,
                object_id,
                payload,
            )
            .is_err();
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn prepared_repair_projection(
        &self,
        active_route: &ActivePathwayRoute,
        topology: &Configuration,
    ) -> Option<ActivePathwayRoute> {
        if !self.repair_allowed(active_route) {
            return None;
        }
        let mut repaired_active_route = active_route.clone();
        self.repair_remaining_suffix(&mut repaired_active_route, topology)
            .then_some(repaired_active_route)
    }

    fn merge_retained_flush_projection(
        next_active_route: &mut ActivePathwayRoute,
        flushed_active_route: &ActivePathwayRoute,
    ) {
        next_active_route.anti_entropy.retained_objects =
            flushed_active_route.anti_entropy.retained_objects.clone();
        next_active_route.forwarding.in_flight_frames =
            flushed_active_route.forwarding.in_flight_frames;
        next_active_route.forwarding.last_ack_at_tick =
            flushed_active_route.forwarding.last_ack_at_tick;
    }

    pub(super) fn execute_maintenance_transition(
        &mut self,
        original_active_route: &ActivePathwayRoute,
        transition: MaintenanceTransition,
        context: &MaintenanceContext<'_>,
    ) -> Result<
        (
            ActivePathwayRoute,
            RouteRuntimeState,
            RouteMaintenanceResult,
        ),
        RouteError,
    > {
        let mut next_active_route = transition.next_active_route;
        let next_runtime = transition.next_runtime;
        if transition.consume_anti_entropy_pressure {
            self.consume_anti_entropy_pressure(context.now);
        }
        if transition.flush_retained_payloads == Some(RetainedFlushPhase::BeforeProjection) {
            let mut flushed_active_route = original_active_route.clone();
            self.flush_retained_payloads(&mut flushed_active_route)?;
            Self::merge_retained_flush_projection(&mut next_active_route, &flushed_active_route);
        }
        if transition.protocol_action == Some(MaintenanceProtocolAction::RepairExchange) {
            choreography::repair_exchange(
                &mut self.transport,
                &mut self.retention,
                &mut self.effects,
                &context.identity.stamp.route_id,
            )?;
        }
        if transition.flush_retained_payloads == Some(RetainedFlushPhase::AfterProjection) {
            self.flush_retained_payloads(&mut next_active_route)?;
        }
        if transition.protocol_action == Some(MaintenanceProtocolAction::HandoffExchange) {
            choreography::handoff_exchange(
                &mut self.transport,
                &mut self.retention,
                &mut self.effects,
                &context.identity.stamp.route_id,
            )?;
        }
        Ok((next_active_route, next_runtime, transition.result))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use jacquard_core::{
        Blake3Digest, ByteCount, ContentId, DeterministicOrderKey, EndpointLocator, HealthScore,
        Limit, LinkEndpoint, Observation, PenaltyPoints, ReachabilityState, ReceiptId, RouteCost,
        RouteEpoch, RouteHandle, RouteId, RouteIdentityStamp, RouteLease, RouteLifecycleEvent,
        RouteMaterializationInput, RouteProgressContract, RouteProgressState, RouteProtectionClass,
        RouteRepairClass, RouteReplacementPolicy, RouteServiceKind, SelectedRoutingParameters,
        Tick, TimeWindow, TransportKind,
    };
    use jacquard_mem_link_profile::{
        InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
    };
    use jacquard_traits::{Blake3Hashing, RoutingEngine, RoutingEnginePlanner};

    use super::*;
    use crate::engine::{
        PathwayForwardingState, PathwayHandoffState, PathwayPath, PathwayRepairState,
        PathwayRouteAntiEntropyState, PathwayRouteClass, PathwayRouteSegment,
    };
    use crate::{DeterministicPathwayTopologyModel, PathwayEngine};

    fn node(byte: u8) -> jacquard_core::NodeId {
        jacquard_core::NodeId([byte; 32])
    }

    fn sample_runtime() -> RouteRuntimeState {
        RouteRuntimeState {
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: jacquard_core::RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: HealthScore(1000),
                congestion_penalty_points: PenaltyPoints(0),
                last_validated_at_tick: Tick(2),
            },
            progress: RouteProgressContract {
                productive_step_count_max: jacquard_core::Limit::Bounded(4),
                total_step_count_max: jacquard_core::Limit::Bounded(4),
                last_progress_at_tick: Tick(4),
                state: RouteProgressState::Satisfied,
            },
        }
    }

    fn opaque_endpoint(byte: u8) -> LinkEndpoint {
        LinkEndpoint::new(
            TransportKind::WifiAware,
            EndpointLocator::Opaque(vec![byte]),
            ByteCount(128),
        )
    }

    fn sample_active_route(partition_mode: bool) -> ActivePathwayRoute {
        ActivePathwayRoute {
            path: PathwayPath {
                route_id: RouteId([7; 16]),
                epoch: RouteEpoch(2),
                source: node(1),
                destination: jacquard_core::DestinationId::Node(node(3)),
                segments: vec![
                    PathwayRouteSegment {
                        node_id: node(2),
                        endpoint: opaque_endpoint(2),
                    },
                    PathwayRouteSegment {
                        node_id: node(3),
                        endpoint: opaque_endpoint(3),
                    },
                ],
                valid_for: TimeWindow::new(Tick(2), Tick(10)).expect("window"),
                route_class: PathwayRouteClass::MultiHop,
            },
            committee: None,
            current_epoch: RouteEpoch(2),
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            route_cost: RouteCost {
                message_count_max: Limit::Bounded(1),
                byte_count_max: Limit::Bounded(ByteCount(1024)),
                hop_count: 2,
                repair_attempt_count_max: Limit::Bounded(1),
                hold_bytes_reserved: Limit::Bounded(ByteCount(0)),
                work_step_count_max: Limit::Bounded(2),
            },
            ordering_key: DeterministicOrderKey {
                stable_key: RouteId([7; 16]),
                tie_break: jacquard_core::OrderStamp(17),
            },
            forwarding: PathwayForwardingState {
                current_owner_node_id: node(1),
                next_hop_index: 0,
                in_flight_frames: 1,
                last_ack_at_tick: Some(Tick(3)),
            },
            repair: PathwayRepairState {
                steps_remaining: 3,
                last_repaired_at_tick: Some(Tick(3)),
            },
            handoff: PathwayHandoffState::default(),
            anti_entropy: PathwayRouteAntiEntropyState {
                partition_mode,
                retained_objects: if partition_mode {
                    BTreeSet::from([ContentId {
                        digest: Blake3Digest([6; 32]),
                    }])
                } else {
                    BTreeSet::new()
                },
                last_refresh_at_tick: Some(Tick(3)),
            },
        }
    }

    fn sample_topology() -> Observation<jacquard_core::Configuration> {
        Observation {
            value: jacquard_core::Configuration {
                epoch: RouteEpoch(4),
                nodes: BTreeMap::from([
                    (
                        node(1),
                        jacquard_testkit::topology::node(1).pathway().build(),
                    ),
                    (
                        node(2),
                        jacquard_testkit::topology::node(2).pathway().build(),
                    ),
                    (
                        node(3),
                        jacquard_testkit::topology::node(3).pathway().build(),
                    ),
                ]),
                links: BTreeMap::from([
                    (
                        (node(1), node(2)),
                        jacquard_testkit::topology::link(2)
                            .observed_at(Tick(4))
                            .build(),
                    ),
                    (
                        (node(2), node(1)),
                        jacquard_testkit::topology::link(1)
                            .observed_at(Tick(4))
                            .build(),
                    ),
                    (
                        (node(2), node(3)),
                        jacquard_testkit::topology::link(3)
                            .observed_at(Tick(4))
                            .build(),
                    ),
                    (
                        (node(3), node(2)),
                        jacquard_testkit::topology::link(2)
                            .observed_at(Tick(4))
                            .build(),
                    ),
                ]),
                environment: jacquard_core::Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: jacquard_core::RatioPermille(0),
                    contention_permille: jacquard_core::RatioPermille(0),
                },
            },
            source_class: jacquard_core::FactSourceClass::Local,
            evidence_class: jacquard_core::RoutingEvidenceClass::AdmissionWitnessed,
            origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(4),
        }
    }

    fn sample_objective() -> jacquard_core::RoutingObjective {
        jacquard_core::RoutingObjective {
            destination: jacquard_core::DestinationId::Node(node(3)),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: jacquard_core::ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
            latency_budget_ms: Limit::Bounded(jacquard_core::DurationMs(250)),
            protection_priority: jacquard_core::PriorityPoints(10),
            connectivity_priority: jacquard_core::PriorityPoints(20),
        }
    }

    fn sample_profile() -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: jacquard_core::ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: RouteReplacementPolicy::Allowed,
        }
    }

    fn sample_lease(epoch: RouteEpoch) -> RouteLease {
        RouteLease {
            owner_node_id: node(1),
            lease_epoch: epoch,
            valid_for: TimeWindow::new(Tick(2), Tick(1000)).expect("lease window"),
        }
    }

    type TestEngine = PathwayEngine<
        DeterministicPathwayTopologyModel,
        InMemoryTransport,
        InMemoryRetentionStore,
        InMemoryRuntimeEffects,
        Blake3Hashing,
    >;
    type MaterializedEngine = (
        TestEngine,
        jacquard_core::PublishedRouteRecord,
        jacquard_core::RouteRuntimeState,
        Observation<jacquard_core::Configuration>,
    );

    fn materialized_engine() -> MaterializedEngine {
        let topology = sample_topology();
        let mut engine = PathwayEngine::without_committee_selector(
            node(1),
            DeterministicPathwayTopologyModel::new(),
            InMemoryTransport::new(),
            InMemoryRetentionStore::default(),
            InMemoryRuntimeEffects {
                now: Tick(2),
                ..Default::default()
            },
            Blake3Hashing,
        );
        engine
            .engine_tick(&jacquard_core::RoutingTickContext::new(topology.clone()))
            .expect("seed latest topology");

        let objective = sample_objective();
        let profile = sample_profile();
        let candidate = engine
            .candidate_routes(&objective, &profile, &topology)
            .into_iter()
            .next()
            .expect("pathway candidate");
        let route_id = candidate.route_id;
        let admission = engine
            .admit_route(&objective, &profile, candidate, &topology)
            .expect("admit pathway candidate");
        let lease = sample_lease(topology.value.epoch);
        let input = RouteMaterializationInput {
            handle: RouteHandle {
                stamp: RouteIdentityStamp {
                    route_id,
                    topology_epoch: topology.value.epoch,
                    materialized_at_tick: Tick(2),
                    publication_id: jacquard_core::PublicationId([7; 16]),
                },
            },
            admission,
            lease,
        };
        let installation = engine
            .materialize_route(input.clone())
            .expect("materialize pathway route");
        let runtime = jacquard_core::RouteRuntimeState {
            last_lifecycle_event: installation.last_lifecycle_event,
            health: installation.health,
            progress: installation.progress,
        };
        let identity = jacquard_core::PublishedRouteRecord {
            stamp: input.handle.stamp,
            proof: installation.materialization_proof,
            admission: input.admission,
            lease: input.lease,
        };
        (engine, identity, runtime, topology)
    }

    #[test]
    fn policy_shift_reducer_requests_preprojection_flush_and_handoff() {
        let active_route = sample_active_route(true);
        let runtime = sample_runtime();
        let transition = reduce_maintenance_transition(
            &active_route,
            &runtime,
            &NormalizedMaintenanceInput {
                trigger: RouteMaintenanceTrigger::PolicyShift,
                now: Tick(5),
                handoff_receipt_id: ReceiptId([9; 16]),
                latest_topology_epoch: RouteEpoch(2),
                repaired_active_route: None,
            },
        )
        .expect("policy shift transition");

        assert_eq!(
            transition.flush_retained_payloads,
            Some(RetainedFlushPhase::BeforeProjection)
        );
        assert_eq!(
            transition.protocol_action,
            Some(MaintenanceProtocolAction::HandoffExchange)
        );
        assert_eq!(
            transition
                .next_active_route
                .forwarding
                .current_owner_node_id,
            node(2)
        );
        assert_eq!(transition.next_active_route.forwarding.next_hop_index, 1);
        assert!(matches!(
            transition.result.outcome,
            RouteMaintenanceOutcome::HandedOff(_)
        ));
    }

    #[test]
    fn anti_entropy_recovery_reducer_returns_recovered_projection() {
        let active_route = sample_active_route(true);
        let runtime = sample_runtime();
        let transition = reduce_maintenance_transition(
            &active_route,
            &runtime,
            &NormalizedMaintenanceInput {
                trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
                now: Tick(6),
                handoff_receipt_id: ReceiptId([9; 16]),
                latest_topology_epoch: RouteEpoch(2),
                repaired_active_route: None,
            },
        )
        .expect("anti-entropy transition");

        assert!(transition.consume_anti_entropy_pressure);
        assert_eq!(
            transition.flush_retained_payloads,
            Some(RetainedFlushPhase::AfterProjection)
        );
        assert!(!transition.next_active_route.anti_entropy.partition_mode);
        assert_eq!(
            transition.next_active_route.last_lifecycle_event,
            RouteLifecycleEvent::RecoveredFromPartition
        );
        assert_eq!(
            transition.result.event,
            RouteLifecycleEvent::RecoveredFromPartition
        );
        assert_eq!(
            transition.result.outcome,
            RouteMaintenanceOutcome::Continued
        );
    }

    #[test]
    fn maintenance_wrapper_matches_reducer_projection_for_policy_shift() {
        let (mut engine, identity, mut runtime, topology) = materialized_engine();
        let now = engine.current_tick();
        let original_active_route = engine
            .active_routes
            .get(&identity.stamp.route_id)
            .cloned()
            .expect("active route present");
        let transition = reduce_maintenance_transition(
            &original_active_route,
            &runtime,
            &NormalizedMaintenanceInput {
                trigger: RouteMaintenanceTrigger::PolicyShift,
                now,
                handoff_receipt_id: engine.receipt_id_for_route(&identity.stamp.route_id),
                latest_topology_epoch: topology.value.epoch,
                repaired_active_route: None,
            },
        )
        .expect("policy shift transition");
        let mut expected_runtime = transition.next_runtime.clone();
        expected_runtime.health =
            engine.current_route_health(Some(&transition.next_active_route), now);

        let result = engine
            .maintain_route_inner(
                &identity,
                &mut runtime,
                RouteMaintenanceTrigger::PolicyShift,
            )
            .expect("policy shift maintenance");

        assert_eq!(result, transition.result);
        assert_eq!(runtime, expected_runtime);
        assert_eq!(
            engine
                .active_routes
                .get(&identity.stamp.route_id)
                .expect("updated route present"),
            &transition.next_active_route
        );
        assert!(
            !engine.effects.storage.is_empty(),
            "maintenance wrapper should checkpoint the projected route"
        );
    }
}
