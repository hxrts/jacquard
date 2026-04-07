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

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    Configuration, Fact, FactBasis, MaterializedRoute, MaterializedRouteIdentity, Observation,
    PenaltyPoints, ReachabilityState, RouteBinding, RouteCommitment, RouteCommitmentResolution,
    RouteError, RouteEvent, RouteHealth, RouteId, RouteInstallation, RouteInvalidationReason,
    RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceResult,
    RouteMaintenanceTrigger, RouteMaterializationInput, RouteMaterializationProof,
    RouteOperationId, RoutePolicyError, RouteProgressContract, RouteProgressState,
    RouteRuntimeError, RouteSelectionError, RouteSemanticHandoff, TimeoutPolicy,
};
use jacquard_traits::{CommitteeCoordinatedEngine, MeshRoutingEngine, RoutingEngine};

use super::{
    support::limit_u32, ActiveMeshRoute, MeshEffectsBounds, MeshEngine, MeshHasherBounds,
    MeshSelectorBounds, MeshTransportBounds, MESH_ACTIVE_ROUTE_COUNT_MAX,
    MESH_COMMITMENT_ATTEMPT_COUNT_MAX, MESH_COMMITMENT_BACKOFF_MS_MAX,
    MESH_COMMITMENT_INITIAL_BACKOFF_MS, MESH_COMMITMENT_OVERALL_TIMEOUT_MS,
};
use crate::committee::mesh_health_score;

impl<Topology, Transport, Retention, Effects, Hasher, Selector> RoutingEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: MeshTransportBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        self.materialize_route_inner(input)
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
    Transport: MeshTransportBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
{
    // Bootstrap fallback used only when an engine is asked to materialize
    // a route before the first `engine_tick` has populated
    // `latest_topology`. `mesh_health_score` of this empty configuration
    // is HealthScore(0), which correctly represents "no observed
    // stability yet" until the first tick arrives.
    fn fallback_health_configuration(&self, cached: &super::CachedCandidate) -> Configuration {
        Configuration {
            epoch: cached.path.epoch,
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
            environment: jacquard_core::Environment {
                reachable_neighbor_count: 0,
                churn_permille: jacquard_core::RatioPermille(0),
                contention_permille: jacquard_core::RatioPermille(0),
            },
        }
    }

    fn route_health_for_materialization(
        &self,
        cached: &super::CachedCandidate,
        now: jacquard_core::Tick,
    ) -> RouteHealth {
        RouteHealth {
            reachability_state: ReachabilityState::Reachable,
            stability_score: mesh_health_score(&self.latest_topology.as_ref().map_or_else(
                || self.fallback_health_configuration(cached),
                |topology| topology.value.clone(),
            )),
            congestion_penalty_points: PenaltyPoints(0),
            last_validated_at_tick: now,
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
        _input: &RouteMaterializationInput,
        cached: &super::CachedCandidate,
        now: jacquard_core::Tick,
        proof: RouteMaterializationProof,
    ) -> RouteInstallation {
        RouteInstallation {
            materialization_proof: proof,
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: self.route_health_for_materialization(cached, now),
            progress: RouteProgressContract {
                productive_step_count_max: cached.admission_check.productive_step_bound,
                total_step_count_max: cached.admission_check.total_step_bound,
                last_progress_at_tick: now,
                state: RouteProgressState::Satisfied,
            },
        }
    }

    fn active_route_for_materialization(&self, cached: &super::CachedCandidate) -> ActiveMeshRoute {
        ActiveMeshRoute {
            path: cached.path.clone(),
            committee: cached.committee.clone(),
            current_epoch: cached.path.epoch,
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            in_flight_frames: 0,
            last_ack_at_tick: None,
            repair_steps_remaining: limit_u32(cached.admission_check.productive_step_bound),
            route_cost: cached.route_cost.clone(),
            partition_mode: false,
            retained_objects: BTreeSet::new(),
            ordering_key: cached.ordering_key.clone(),
        }
    }

    fn expired_lease_result(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut jacquard_core::RouteRuntimeState,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        runtime.last_lifecycle_event = RouteLifecycleEvent::Expired;
        runtime.progress.state = RouteProgressState::Failed;
        let result = RouteMaintenanceResult {
            event: RouteLifecycleEvent::Expired,
            outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired),
        };
        self.record_event(RouteEvent::RouteMaintenanceCompleted {
            route_id: identity.handle.route_id,
            result: result.clone(),
        })?;
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
    ) -> RouteMaintenanceResult {
        let handoff = RouteSemanticHandoff {
            route_id: identity.handle.route_id,
            from_node_id: identity.lease.owner_node_id,
            to_node_id: Self::handoff_target(active_route, identity.lease.owner_node_id),
            handoff_epoch: active_route.current_epoch,
            receipt_id: handoff_receipt_id,
        };
        active_route.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
        runtime.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::HandedOff,
            outcome: RouteMaintenanceOutcome::HandedOff(handoff),
        }
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
    ) -> RouteMaintenanceResult {
        match trigger {
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
                Self::handoff_result(identity, active_route, runtime, handoff_receipt_id)
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
        }
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
        self.effects
            .store_bytes(b"mesh/topology-epoch", &epoch_bytes)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))?;
        let _observations = self.transport.poll_transport()?;
        Ok(())
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: MeshTransportBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
{
    fn materialize_route_inner(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        // Two hard checks before any state change: the active-route
        // budget (replacements for an existing route_id bypass the cap),
        // and the router-owned lease validity. Lease expiry is a typed
        // runtime failure, not a silent fallthrough.
        let is_replacement = self.active_routes.contains_key(&input.handle.route_id);
        if !is_replacement && self.active_routes.len() >= MESH_ACTIVE_ROUTE_COUNT_MAX {
            return Err(RouteError::Policy(RoutePolicyError::BudgetExceeded));
        }
        let cached = self
            .find_cached_candidate_by_route_id(&input.admission.route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let now = self.effects.now_tick();
        input.lease.ensure_valid_at(now)?;

        let proof = self.materialization_proof_for(&input, now);
        let installation = self.installation_for(&input, &cached, now, proof.clone());
        let active_route = self.active_route_for_materialization(&cached);
        self.store_checkpoint(&active_route)?;
        self.active_routes
            .insert(input.handle.route_id, active_route);
        self.record_event(RouteEvent::RouteMaterialized {
            handle: input.handle,
            proof,
        })?;
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

        let active_route_snapshot;
        let result = {
            let active_route = self
                .active_routes
                .get_mut(&identity.handle.route_id)
                .ok_or(RouteSelectionError::NoCandidate)?;
            let result = Self::apply_maintenance_trigger(
                identity,
                active_route,
                runtime,
                trigger,
                now,
                handoff_receipt_id,
                latest_topology_epoch,
            );
            active_route_snapshot = active_route.clone();
            result
        };

        runtime.health.last_validated_at_tick = now;
        self.store_checkpoint(&active_route_snapshot)?;
        self.record_event(RouteEvent::RouteMaintenanceCompleted {
            route_id: identity.handle.route_id,
            result: result.clone(),
        })?;
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
