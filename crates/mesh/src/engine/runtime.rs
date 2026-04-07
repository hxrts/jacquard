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

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use jacquard_core::{
    Configuration, Fact, FactBasis, HealthScore, MaterializedRoute, MaterializedRouteIdentity,
    Observation, PenaltyPoints, ReachabilityState, RouteBinding, RouteCommitment,
    RouteCommitmentResolution, RouteError, RouteEvent, RouteHealth, RouteId, RouteInstallation,
    RouteInvalidationReason, RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
    RouteMaterializationProof, RouteOperationId, RoutePolicyError, RouteProgressContract,
    RouteProgressState, RouteRuntimeError, RouteSelectionError, RouteSemanticHandoff,
    TimeoutPolicy, TransportObservation,
};
use jacquard_traits::{CommitteeCoordinatedEngine, MeshRoutingEngine, RoutingEngine};

use super::{
    support::{
        decode_backend_token, deterministic_order_key, encode_path_bytes, limit_u32,
        node_path_from_plan_token, route_cost_for_segments, shortest_paths,
        topology_epoch_storage_key,
    },
    ActiveMeshRoute, MeshControlState, MeshEffectsBounds, MeshEngine, MeshHasherBounds,
    MeshObservedRemoteLink, MeshSelectorBounds, MeshTransportBounds,
    MeshTransportObservationSummary, MESH_ACTIVE_ROUTE_COUNT_MAX,
    MESH_COMMITMENT_ATTEMPT_COUNT_MAX, MESH_COMMITMENT_BACKOFF_MS_MAX,
    MESH_COMMITMENT_INITIAL_BACKOFF_MS, MESH_COMMITMENT_OVERALL_TIMEOUT_MS,
};

struct MaintenanceContext<'a> {
    identity: &'a MaterializedRouteIdentity,
    now: jacquard_core::Tick,
    handoff_receipt_id: jacquard_core::ReceiptId,
    latest_topology: Option<&'a Observation<Configuration>>,
}
impl<Topology, Transport, Retention, Effects, Hasher, Selector> RoutingEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Transport: MeshTransportBounds,
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

    fn engine_tick(&mut self, topology: &Observation<Configuration>) -> Result<(), RouteError> {
        self.engine_tick_inner(topology)
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
        // route. The next `engine_tick` reconciles the storage side.
        let _ = self.remove_checkpoint(route_id);
        self.active_routes.remove(route_id);
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Transport: MeshTransportBounds,
    Retention: super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn summarize_transport_observations(
        observations: &[TransportObservation],
    ) -> Option<MeshTransportObservationSummary> {
        let mut last_observed_at_tick = None;
        let mut payload_event_count = 0_u16;
        let mut observed_link_count = 0_u16;
        let mut reachable_remote_nodes = std::collections::BTreeSet::new();
        let mut stability_sum = 0_u32;
        let mut loss_sum = 0_u32;
        let mut remote_links = BTreeMap::new();

        for observation in observations {
            match observation {
                TransportObservation::PayloadReceived {
                    from_node_id,
                    observed_at_tick,
                    ..
                } => {
                    payload_event_count = payload_event_count.saturating_add(1);
                    reachable_remote_nodes.insert(*from_node_id);
                    last_observed_at_tick = Some(
                        last_observed_at_tick
                            .map_or(*observed_at_tick, |current: jacquard_core::Tick| {
                                current.max(*observed_at_tick)
                            }),
                    );
                }
                TransportObservation::LinkObserved {
                    remote_node_id,
                    observation,
                } => {
                    observed_link_count = observed_link_count.saturating_add(1);
                    reachable_remote_nodes.insert(*remote_node_id);
                    last_observed_at_tick = Some(last_observed_at_tick.map_or(
                        observation.observed_at_tick,
                        |current: jacquard_core::Tick| current.max(observation.observed_at_tick),
                    ));
                    let delivery = match &observation.value.state.delivery_confidence_permille {
                        jacquard_core::Belief::Absent => 0,
                        jacquard_core::Belief::Estimated(estimate) => {
                            u32::from(estimate.value.get())
                        }
                    };
                    let symmetry = match &observation.value.state.symmetry_permille {
                        jacquard_core::Belief::Absent => 0,
                        jacquard_core::Belief::Estimated(estimate) => {
                            u32::from(estimate.value.get())
                        }
                    };
                    stability_sum =
                        stability_sum.saturating_add((delivery.saturating_add(symmetry)) / 2);
                    let congestion_penalty_points =
                        PenaltyPoints(u32::from(observation.value.state.loss_permille.get()) / 100);
                    loss_sum = loss_sum.saturating_add(congestion_penalty_points.0);
                    remote_links.insert(
                        *remote_node_id,
                        MeshObservedRemoteLink {
                            last_observed_at_tick: observation.observed_at_tick,
                            stability_score: HealthScore((delivery.saturating_add(symmetry)) / 2),
                            congestion_penalty_points,
                        },
                    );
                }
            }
        }

        last_observed_at_tick.map(|last_observed_at_tick| {
            let reachable_remote_count =
                u16::try_from(reachable_remote_nodes.len()).unwrap_or(u16::MAX);
            let stability_score = if observed_link_count > 0 {
                HealthScore(stability_sum / u32::from(observed_link_count))
            } else if payload_event_count > 0 {
                HealthScore(500)
            } else {
                HealthScore(0)
            };
            let congestion_penalty_points = if observed_link_count > 0 {
                PenaltyPoints(loss_sum / u32::from(observed_link_count))
            } else {
                PenaltyPoints(0)
            };

            MeshTransportObservationSummary {
                last_observed_at_tick: Some(last_observed_at_tick),
                payload_event_count,
                observed_link_count,
                reachable_remote_count,
                stability_score,
                congestion_penalty_points,
                remote_links,
            }
        })
    }

    fn next_control_state(
        &self,
        topology: &Observation<Configuration>,
        transport_summary: Option<&MeshTransportObservationSummary>,
    ) -> MeshControlState {
        let previous = self.control_state.as_ref();
        let neighborhood = self.topology_model.neighborhood_estimate(
            &self.local_node_id,
            topology.observed_at_tick,
            &topology.value,
        );
        let neighborhood_repair_pressure = neighborhood
            .as_ref()
            .and_then(|estimate| estimate.repair_pressure_score)
            .map_or(0, |score| score.0);
        let transport_stability = transport_summary
            .map(|summary| summary.stability_score.0)
            .unwrap_or_else(|| {
                previous.map_or(0, |state| {
                    state.transport_stability_score.0.saturating_sub(100)
                })
            });
        let observed_pressure = transport_summary.map_or(0, |summary| {
            u32::from(summary.payload_event_count == 0 && summary.observed_link_count == 0) * 200
                + summary.congestion_penalty_points.0.saturating_mul(50)
                + 1000_u32.saturating_sub(summary.stability_score.0) / 2
        });
        let previous_anti_entropy_pressure = previous.map_or(0, |state| {
            state.anti_entropy.pressure_score.0.saturating_sub(150)
        });
        let anti_entropy_pressure = previous_anti_entropy_pressure
            .saturating_add(observed_pressure)
            .min(1000);
        let repair_pressure = neighborhood_repair_pressure
            .saturating_add(observed_pressure / 2)
            .min(1000);

        MeshControlState {
            last_updated_at_tick: topology.observed_at_tick,
            transport_stability_score: HealthScore(transport_stability.min(1000)),
            repair_pressure_score: HealthScore(repair_pressure),
            anti_entropy: super::types::MeshAntiEntropyState {
                pressure_score: HealthScore(anti_entropy_pressure),
                last_refreshed_at_tick: previous
                    .and_then(|state| state.anti_entropy.last_refreshed_at_tick),
            },
        }
    }

    fn consume_anti_entropy_pressure(&mut self, now: jacquard_core::Tick) {
        if let Some(control_state) = self.control_state.as_mut() {
            control_state.anti_entropy.pressure_score = HealthScore(
                control_state
                    .anti_entropy
                    .pressure_score
                    .0
                    .saturating_sub(250),
            );
            control_state.anti_entropy.last_refreshed_at_tick = Some(now);
        }
    }

    fn repair_allowed(&self, active_route: &ActiveMeshRoute) -> bool {
        if active_route.repair.steps_remaining == 0 {
            return false;
        }
        self.control_state.as_ref().is_none_or(|state| {
            !(state.repair_pressure_score.0 > 300 && active_route.repair.steps_remaining <= 1)
        })
    }

    fn current_route_health(
        &self,
        active_route: Option<&ActiveMeshRoute>,
        now: jacquard_core::Tick,
    ) -> RouteHealth {
        let Some(active_route) = active_route else {
            return RouteHealth {
                reachability_state: ReachabilityState::Unknown,
                stability_score: HealthScore(0),
                congestion_penalty_points: PenaltyPoints(0),
                last_validated_at_tick: now,
            };
        };

        let Some(topology) = self.latest_topology.as_ref() else {
            return RouteHealth {
                reachability_state: ReachabilityState::Unknown,
                stability_score: HealthScore(0),
                congestion_penalty_points: PenaltyPoints(0),
                last_validated_at_tick: now,
            };
        };

        let remaining_segments =
            &active_route.path.segments[usize::from(active_route.forwarding.next_hop_index)..];
        if remaining_segments.is_empty() {
            return RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: HealthScore(1000),
                congestion_penalty_points: PenaltyPoints(0),
                last_validated_at_tick: topology.observed_at_tick,
            };
        }

        let mut current_node_id = active_route.forwarding.current_owner_node_id;
        let mut stability_score = HealthScore(1000);
        let mut congestion_penalty_points = PenaltyPoints(0);
        let mut reachability_state = ReachabilityState::Reachable;
        let mut last_validated_at_tick = topology.observed_at_tick;

        for (index, segment) in remaining_segments.iter().enumerate() {
            let mut route_link = match crate::topology::adjacent_link_between(
                &current_node_id,
                &segment.node_id,
                &topology.value,
            ) {
                Some(link) => Some(link),
                None => {
                    reachability_state = ReachabilityState::Unreachable;
                    None
                }
            };

            if index == 0 {
                if let Some(summary) = self.last_transport_summary.as_ref() {
                    if let Some(remote) = summary.remote_links.get(&segment.node_id) {
                        stability_score =
                            HealthScore(stability_score.0.min(remote.stability_score.0));
                        congestion_penalty_points = PenaltyPoints(
                            congestion_penalty_points
                                .0
                                .max(remote.congestion_penalty_points.0),
                        );
                        last_validated_at_tick =
                            last_validated_at_tick.max(remote.last_observed_at_tick);
                    }
                }
            }

            if let Some(link) = route_link.take() {
                let delivery = match &link.state.delivery_confidence_permille {
                    jacquard_core::Belief::Absent => None,
                    jacquard_core::Belief::Estimated(estimate) => {
                        Some(u32::from(estimate.value.get()))
                    }
                };
                let symmetry = match &link.state.symmetry_permille {
                    jacquard_core::Belief::Absent => None,
                    jacquard_core::Belief::Estimated(estimate) => {
                        Some(u32::from(estimate.value.get()))
                    }
                };
                let link_stability = match (delivery, symmetry) {
                    (Some(delivery), Some(symmetry)) => Some((delivery + symmetry) / 2),
                    (Some(delivery), None) => Some(delivery),
                    (None, Some(symmetry)) => Some(symmetry),
                    (None, None) => None,
                };
                if let Some(link_stability) = link_stability {
                    stability_score = HealthScore(stability_score.0.min(link_stability));
                }
                congestion_penalty_points = PenaltyPoints(
                    congestion_penalty_points
                        .0
                        .max(u32::from(link.state.loss_permille.get()) / 100),
                );
            } else {
                break;
            }

            current_node_id = segment.node_id;
        }

        if let Some(control_state) = self.control_state.as_ref() {
            stability_score = HealthScore(
                stability_score
                    .0
                    .min(control_state.transport_stability_score.0),
            );
            congestion_penalty_points = PenaltyPoints(
                congestion_penalty_points
                    .0
                    .max(control_state.anti_entropy.pressure_score.0 / 100),
            );
        }

        RouteHealth {
            reachability_state,
            stability_score,
            congestion_penalty_points,
            last_validated_at_tick,
        }
    }

    fn materialization_proof_for(
        &self,
        input: &RouteMaterializationInput,
        now: jacquard_core::Tick,
    ) -> RouteMaterializationProof {
        RouteMaterializationProof {
            route_id: input.handle.route_id,
            topology_epoch: input.handle.topology_epoch,
            materialized_at_tick: now,
            publication_id: input.handle.publication_id,
            witness: Fact {
                value: input.admission.witness.clone(),
                basis: FactBasis::Admitted,
                established_at_tick: now,
            },
        }
    }

    fn installation_for(
        &self,
        input: &RouteMaterializationInput,
        now: jacquard_core::Tick,
        proof: RouteMaterializationProof,
    ) -> RouteInstallation {
        RouteInstallation {
            materialization_proof: proof,
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: self.current_route_health(None, now),
            progress: RouteProgressContract {
                productive_step_count_max: input.admission.admission_check.productive_step_bound,
                total_step_count_max: input.admission.admission_check.total_step_bound,
                last_progress_at_tick: now,
                state: RouteProgressState::Satisfied,
            },
        }
    }

    fn active_route_for_materialization(
        &self,
        input: &RouteMaterializationInput,
        path: super::MeshPath,
        committee: Option<jacquard_core::CommitteeSelection>,
        ordering_key: jacquard_core::DeterministicOrderKey<jacquard_core::RouteId>,
    ) -> ActiveMeshRoute {
        ActiveMeshRoute {
            current_epoch: path.epoch,
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            path,
            committee,
            route_cost: input.admission.admission_check.route_cost.clone(),
            ordering_key,
            forwarding: super::MeshForwardingState {
                current_owner_node_id: input.lease.owner_node_id,
                next_hop_index: 0,
                in_flight_frames: 0,
                last_ack_at_tick: None,
            },
            repair: super::MeshRepairState {
                steps_remaining: limit_u32(input.admission.admission_check.productive_step_bound),
                last_repaired_at_tick: None,
            },
            handoff: super::MeshHandoffState {
                last_receipt_id: None,
                last_handoff_at_tick: None,
            },
            anti_entropy: super::MeshRouteAntiEntropyState {
                partition_mode: false,
                retained_objects: BTreeSet::new(),
                last_refresh_at_tick: None,
            },
        }
    }

    fn materialization_plan(
        &self,
        input: &RouteMaterializationInput,
    ) -> Result<
        (
            super::MeshPath,
            Option<jacquard_core::CommitteeSelection>,
            jacquard_core::DeterministicOrderKey<jacquard_core::RouteId>,
        ),
        RouteError,
    > {
        if input.admission.backend_ref.engine != super::MESH_ENGINE_ID {
            return Err(RouteRuntimeError::Invalidated.into());
        }

        let plan = decode_backend_token(&input.admission.backend_ref.backend_route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        if plan.source != self.local_node_id
            || plan.destination != input.admission.objective.destination
        {
            return Err(RouteRuntimeError::Invalidated.into());
        }

        let derived_route_id =
            self.route_id_for_backend(&input.admission.backend_ref.backend_route_id);
        if derived_route_id != input.admission.route_id || derived_route_id != input.handle.route_id
        {
            return Err(RouteRuntimeError::Invalidated.into());
        }

        let node_path = node_path_from_plan_token(&plan);
        let path_bytes = encode_path_bytes(&node_path, &plan.segments);
        let ordering_key = deterministic_order_key(derived_route_id, &self.hashing, &path_bytes);
        let path = super::MeshPath {
            route_id: derived_route_id,
            epoch: plan.epoch,
            source: plan.source,
            destination: plan.destination,
            segments: plan.segments,
            valid_for: plan.valid_for,
            route_class: plan.route_class,
        };
        Ok((path, plan.committee, ordering_key))
    }

    fn validated_materialization_candidate(
        &self,
        input: &RouteMaterializationInput,
        topology: &Observation<Configuration>,
        now: jacquard_core::Tick,
    ) -> Result<(), RouteError> {
        let plan = decode_backend_token(&input.admission.backend_ref.backend_route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        let claimed_epoch = input.handle.topology_epoch;
        if plan.epoch != claimed_epoch
            || input.admission.witness.topology_epoch != claimed_epoch
            || topology.value.epoch != claimed_epoch
        {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        if !plan.valid_for.contains(now) {
            return Err(RouteRuntimeError::Invalidated.into());
        }

        let derived = self
            .derive_candidate_from_backend_ref(
                &input.admission.objective,
                &input.admission.profile,
                topology,
                &input.admission.backend_ref.backend_route_id,
            )
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        if derived.route_id != input.admission.route_id
            || derived.summary != input.admission.summary
            || derived.witness != input.admission.witness
            || derived.admission_check != input.admission.admission_check
        {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        Ok(())
    }

    fn expired_lease_result(
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

    fn apply_repair(
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        now: jacquard_core::Tick,
    ) -> RouteMaintenanceResult {
        active_route.repair.steps_remaining = active_route.repair.steps_remaining.saturating_sub(1);
        active_route.repair.last_repaired_at_tick = Some(now);
        active_route.last_lifecycle_event = RouteLifecycleEvent::Repaired;
        runtime.last_lifecycle_event = RouteLifecycleEvent::Repaired;
        runtime.progress.last_progress_at_tick = now;
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::Repaired,
            outcome: RouteMaintenanceOutcome::Repaired,
        }
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
        active_route.handoff.last_handoff_at_tick = Some(runtime.progress.last_progress_at_tick);
        active_route.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
        runtime.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::HandedOff,
            outcome: RouteMaintenanceOutcome::HandedOff(handoff),
        })
    }

    fn replacement_required(trigger: RouteMaintenanceTrigger) -> RouteMaintenanceResult {
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
            outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired),
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

    fn final_destination_node_id(active_route: &ActiveMeshRoute) -> Option<jacquard_core::NodeId> {
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
    ) -> Result<u32, RouteError> {
        let Some(next_segment) = active_route
            .path
            .segments
            .get(usize::from(active_route.forwarding.next_hop_index))
            .cloned()
        else {
            return Ok(0);
        };

        let retained_object_ids = active_route
            .anti_entropy
            .retained_objects
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut flushed = 0_u32;

        for object_id in retained_object_ids {
            let Some(payload) = self
                .retention
                .take_retained_payload(&object_id)
                .map_err(|_| RouteError::Runtime(RouteRuntimeError::MaintenanceFailed))?
            else {
                active_route
                    .anti_entropy
                    .retained_objects
                    .remove(&object_id);
                continue;
            };

            if let Err(error) = self
                .transport
                .send_transport(&next_segment.endpoint, &payload)
            {
                let _ = self.retention.retain_payload(object_id, payload);
                return Err(RouteError::from(error));
            }

            active_route
                .anti_entropy
                .retained_objects
                .remove(&object_id);
            active_route.forwarding.in_flight_frames =
                active_route.forwarding.in_flight_frames.saturating_add(1);
            active_route.forwarding.last_ack_at_tick = Some(self.effects.now_tick());
            flushed = flushed.saturating_add(1);
        }

        Ok(flushed)
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

        let _released_count = self.flush_retained_payloads(active_route)?;
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

    // Trigger dispatch. `LinkDegraded` attempts local repair or
    // escalates to replacement when the repair budget is gone.
    // `CapacityExceeded` and `PartitionDetected` enter hold-fallback.
    // `PolicyShift` hands off. `EpochAdvanced` bumps the epoch and
    // treats it as a repair step. `LeaseExpiring` and `RouteExpired`
    // terminate. `AntiEntropyRequired` only refreshes progress
    // tracking. Lease expiry is checked by the caller before this
    // method runs, so none of these branches need to double-check it.
    fn apply_maintenance_trigger(
        &mut self,
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
        context: &MaintenanceContext<'_>,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        Ok(match trigger {
            RouteMaintenanceTrigger::LinkDegraded => {
                if !self.repair_allowed(active_route) {
                    return Ok(Self::replacement_required(trigger));
                }
                let repaired = context.latest_topology.is_some_and(|topology| {
                    self.repair_remaining_suffix(active_route, &topology.value)
                });
                if !repaired {
                    Self::replacement_required(trigger)
                } else {
                    active_route.current_epoch = context
                        .latest_topology
                        .map_or(active_route.current_epoch, |topology| topology.value.epoch);
                    let result = Self::apply_repair(active_route, runtime, context.now);
                    if let Some(recovered) =
                        self.recover_from_partition_if_possible(active_route, runtime, context.now)?
                    {
                        recovered
                    } else {
                        result
                    }
                }
            }
            RouteMaintenanceTrigger::CapacityExceeded
            | RouteMaintenanceTrigger::PartitionDetected => {
                Self::enter_partition_mode(active_route, runtime, trigger)
            }
            RouteMaintenanceTrigger::PolicyShift => {
                let _ = self.flush_retained_payloads(active_route)?;
                return Self::handoff_result(
                    context.identity,
                    active_route,
                    runtime,
                    context.handoff_receipt_id,
                );
            }
            RouteMaintenanceTrigger::EpochAdvanced => {
                if !self.repair_allowed(active_route) {
                    return Ok(Self::replacement_required(trigger));
                }
                let repaired = context.latest_topology.is_some_and(|topology| {
                    self.repair_remaining_suffix(active_route, &topology.value)
                });
                if repaired {
                    if let Some(topology) = context.latest_topology {
                        active_route.current_epoch = topology.value.epoch;
                    }
                    let result = Self::apply_repair(active_route, runtime, context.now);
                    if let Some(recovered) =
                        self.recover_from_partition_if_possible(active_route, runtime, context.now)?
                    {
                        recovered
                    } else {
                        result
                    }
                } else {
                    Self::replacement_required(trigger)
                }
            }
            RouteMaintenanceTrigger::LeaseExpiring => RouteMaintenanceResult {
                event: active_route.last_lifecycle_event,
                outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
            },
            RouteMaintenanceTrigger::RouteExpired => {
                Self::route_expired_result(active_route, runtime)
            }
            RouteMaintenanceTrigger::AntiEntropyRequired => {
                self.consume_anti_entropy_pressure(context.now);
                if let Some(recovered) =
                    self.recover_from_partition_if_possible(active_route, runtime, context.now)?
                {
                    recovered
                } else {
                    Self::continue_result(active_route, runtime, context.now)
                }
            }
        })
    }

    fn route_commitments_inner(&self, route: &MaterializedRoute) -> Vec<RouteCommitment> {
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

    fn engine_tick_inner(
        &mut self,
        topology: &Observation<Configuration>,
    ) -> Result<(), RouteError> {
        // Engine-internal middleware loop: refresh the cached topology
        // observation, evict the stale candidate cache, checkpoint the
        // current epoch, and poll transport ingress. Route activation,
        // maintenance, and teardown still happen through the shared
        // trait methods rather than inside this tick.
        self.latest_topology = Some(topology.clone());
        self.candidate_cache.borrow_mut().clear();
        let epoch_bytes = topology.value.epoch.0.to_le_bytes();
        let epoch_key = topology_epoch_storage_key(&self.local_node_id);
        self.effects
            .store_bytes(&epoch_key, &epoch_bytes)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))?;
        let observations = self.transport.poll_transport()?;
        self.last_transport_summary = Self::summarize_transport_observations(&observations);
        self.control_state =
            Some(self.next_control_state(topology, self.last_transport_summary.as_ref()));
        Ok(())
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Transport: MeshTransportBounds,
    Retention: super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn materialize_route_inner(
        &mut self,
        input: &RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        // Two hard checks before any state change: the active-route
        // budget (replacements for an existing route_id bypass the cap),
        // and the router-owned lease validity. Lease expiry is a typed
        // runtime failure, not a silent fallthrough.
        let route_id = input.handle.route_id;
        let previous_active_route = self.active_routes.get(&route_id).cloned();
        let is_replacement = previous_active_route.is_some();
        if !is_replacement && self.active_routes.len() >= MESH_ACTIVE_ROUTE_COUNT_MAX {
            return Err(RouteError::Policy(RoutePolicyError::BudgetExceeded));
        }
        let now = self.effects.now_tick();
        let latest_topology = self
            .latest_topology
            .as_ref()
            .ok_or(RouteError::Runtime(RouteRuntimeError::Invalidated))?;
        input.lease.ensure_valid_at(now)?;
        self.validated_materialization_candidate(input, latest_topology, now)?;
        let (path, committee, ordering_key) = self.materialization_plan(input)?;

        let proof = self.materialization_proof_for(input, now);
        let installation = self.installation_for(input, now, proof.clone());
        let active_route =
            self.active_route_for_materialization(input, path, committee, ordering_key);
        let installation = RouteInstallation {
            health: self.current_route_health(Some(&active_route), now),
            ..installation
        };
        let route_event = RouteEvent::RouteMaterialized {
            handle: input.handle.clone(),
            proof,
        };
        self.store_checkpoint(&active_route)?;
        if let Err(error) = self.record_event(route_event) {
            if let Some(previous_active_route) = previous_active_route.as_ref() {
                let _ = self.store_checkpoint(previous_active_route);
            } else {
                let _ = self.remove_checkpoint(&route_id);
            }
            return Err(error);
        }
        self.active_routes.insert(route_id, active_route);
        Ok(installation)
    }

    fn maintain_route_inner(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let now = self.effects.now_tick();
        let handoff_receipt_id = self.receipt_id_for_route(&identity.handle.route_id);
        let latest_topology = self.latest_topology.clone();
        if !identity.lease.is_valid_at(now) {
            return self.expired_lease_result(identity, runtime);
        }

        let original_active_route = self
            .active_routes
            .get(&identity.handle.route_id)
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
                latest_topology: latest_topology.as_ref(),
            },
        )?;

        next_runtime.health = self.current_route_health(Some(&next_active_route), now);
        self.store_checkpoint(&next_active_route)?;
        if let Err(error) = self.record_event(RouteEvent::RouteMaintenanceCompleted {
            route_id: identity.handle.route_id,
            result: result.clone(),
        }) {
            let _ = self.store_checkpoint(&original_active_route);
            return Err(error);
        }
        self.active_routes
            .insert(identity.handle.route_id, next_active_route);
        *runtime = next_runtime;
        Ok(result)
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> MeshRoutingEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Transport: MeshTransportBounds,
    Retention: super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    type TopologyModel = Topology;
    type Transport = Transport;
    type Retention = Retention;

    fn topology_model(&self) -> &Self::TopologyModel {
        &self.topology_model
    }

    fn transport(&self) -> &Self::Transport {
        &self.transport
    }

    fn transport_mut(&mut self) -> &mut Self::Transport {
        &mut self.transport
    }

    fn retention_store(&self) -> &Self::Retention {
        &self.retention
    }

    fn retention_store_mut(&mut self) -> &mut Self::Retention {
        &mut self.retention
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> CommitteeCoordinatedEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Selector: MeshSelectorBounds,
{
    type Selector = Selector;

    fn committee_selector(&self) -> Option<&Self::Selector> {
        self.selector.as_ref()
    }
}
