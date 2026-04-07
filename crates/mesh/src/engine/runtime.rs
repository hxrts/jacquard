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
        node_path_from_plan_token, topology_epoch_storage_key,
    },
    ActiveMeshRoute, MeshEffectsBounds, MeshEngine, MeshHasherBounds, MeshSelectorBounds,
    MeshTransportBounds, MeshTransportObservationSummary, MESH_ACTIVE_ROUTE_COUNT_MAX,
    MESH_COMMITMENT_ATTEMPT_COUNT_MAX, MESH_COMMITMENT_BACKOFF_MS_MAX,
    MESH_COMMITMENT_INITIAL_BACKOFF_MS, MESH_COMMITMENT_OVERALL_TIMEOUT_MS,
};
use crate::committee::mesh_health_score;

impl<Topology, Transport, Retention, Effects, Hasher, Selector> RoutingEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Transport: MeshTransportBounds,
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
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
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
                    loss_sum = loss_sum
                        .saturating_add(u32::from(observation.value.state.loss_permille.get()));
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
                PenaltyPoints((loss_sum / u32::from(observed_link_count)) / 100)
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
            }
        })
    }

    fn current_route_health(&self, now: jacquard_core::Tick) -> RouteHealth {
        let reachability_state =
            if self.latest_topology.is_some() || self.last_transport_summary.is_some() {
                ReachabilityState::Reachable
            } else {
                ReachabilityState::Unreachable
            };
        let stability_score = self
            .latest_topology
            .as_ref()
            .map(|topology| mesh_health_score(&topology.value))
            .or_else(|| {
                self.last_transport_summary
                    .as_ref()
                    .map(|summary| summary.stability_score)
            })
            .unwrap_or(HealthScore(0));
        let congestion_penalty_points = self
            .last_transport_summary
            .as_ref()
            .map_or(PenaltyPoints(0), |summary| {
                summary.congestion_penalty_points
            });
        let last_validated_at_tick = self
            .last_transport_summary
            .as_ref()
            .and_then(|summary| summary.last_observed_at_tick)
            .unwrap_or(now);

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
            health: self.current_route_health(now),
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
            current_owner_node_id: input.lease.owner_node_id,
            next_hop_index: 0,
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            path,
            committee,
            in_flight_frames: 0,
            last_ack_at_tick: None,
            repair_steps_remaining: limit_u32(
                input.admission.admission_check.productive_step_bound,
            ),
            route_cost: input.admission.admission_check.route_cost.clone(),
            partition_mode: false,
            retained_objects: BTreeSet::new(),
            ordering_key,
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
        active_route.repair_steps_remaining = active_route.repair_steps_remaining.saturating_sub(1);
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
        active_route.partition_mode = true;
        active_route.last_lifecycle_event = RouteLifecycleEvent::EnteredPartitionMode;
        runtime.last_lifecycle_event = RouteLifecycleEvent::EnteredPartitionMode;
        runtime.progress.state = RouteProgressState::Blocked;
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::EnteredPartitionMode,
            outcome: RouteMaintenanceOutcome::HoldFallback { trigger },
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
            from_node_id: active_route.current_owner_node_id,
            to_node_id: next_owner,
            handoff_epoch: active_route.current_epoch,
            receipt_id: handoff_receipt_id,
        };
        active_route.current_owner_node_id = next_owner;
        active_route.next_hop_index = active_route.next_hop_index.saturating_add(1);
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

    // Trigger dispatch. `LinkDegraded` attempts local repair or
    // escalates to replacement when the repair budget is gone.
    // `CapacityExceeded` and `PartitionDetected` enter hold-fallback.
    // `PolicyShift` hands off. `EpochAdvanced` bumps the epoch and
    // treats it as a repair step. `LeaseExpiring` and `RouteExpired`
    // terminate. `AntiEntropyRequired` only refreshes progress
    // tracking. Lease expiry is checked by the caller before this
    // method runs, so none of these branches need to double-check it.
    fn apply_maintenance_trigger(
        identity: &MaterializedRouteIdentity,
        active_route: &mut ActiveMeshRoute,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
        now: jacquard_core::Tick,
        handoff_receipt_id: jacquard_core::ReceiptId,
        latest_topology_epoch: Option<jacquard_core::RouteEpoch>,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        Ok(match trigger {
            RouteMaintenanceTrigger::LinkDegraded => {
                if active_route.repair_steps_remaining == 0 {
                    Self::replacement_required(trigger)
                } else {
                    Self::apply_repair(active_route, runtime, now)
                }
            }
            RouteMaintenanceTrigger::CapacityExceeded
            | RouteMaintenanceTrigger::PartitionDetected => {
                Self::enter_partition_mode(active_route, runtime, trigger)
            }
            RouteMaintenanceTrigger::PolicyShift => {
                return Self::handoff_result(identity, active_route, runtime, handoff_receipt_id);
            }
            RouteMaintenanceTrigger::EpochAdvanced => {
                if let Some(epoch) = latest_topology_epoch {
                    active_route.current_epoch = epoch;
                }
                if active_route.repair_steps_remaining > 0 {
                    Self::apply_repair(active_route, runtime, now)
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
                Self::continue_result(active_route, runtime, now)
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
        Ok(())
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Transport: MeshTransportBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
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
        if self.latest_topology.is_none() {
            return Err(RouteError::Runtime(RouteRuntimeError::Invalidated));
        }
        let (path, committee, ordering_key) = self.materialization_plan(input)?;
        let now = self.effects.now_tick();
        input.lease.ensure_valid_at(now)?;

        let proof = self.materialization_proof_for(input, now);
        let installation = self.installation_for(input, now, proof.clone());
        let active_route =
            self.active_route_for_materialization(input, path, committee, ordering_key);
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
        let latest_topology_epoch = self
            .latest_topology
            .as_ref()
            .map(|topology| topology.value.epoch);
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
        let result = Self::apply_maintenance_trigger(
            identity,
            &mut next_active_route,
            &mut next_runtime,
            trigger,
            now,
            handoff_receipt_id,
            latest_topology_epoch,
        )?;

        next_runtime.health = self.current_route_health(now);
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
