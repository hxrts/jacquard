//! `RoutingEngine` and `RouterManagedEngine` impls for `MercatorEngine`.

// proc-macro-scope: Mercator runtime behavior stays outside shared model macros.

use jacquard_core::{
    Configuration, MaterializedRoute, NodeId, Observation, OrderStamp, PublishedRouteRecord,
    ReachabilityState, RouteCommitment, RouteError, RouteId, RouteInstallation,
    RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceResult,
    RouteMaintenanceTrigger, RouteMaterializationInput, RouteRuntimeError, RouteRuntimeState,
    RouteSelectionError, RoutingTickChange, RoutingTickContext, RoutingTickHint,
    RoutingTickOutcome, Tick,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine};

use crate::{
    broker_nodes_for_path,
    corridor::{self, MercatorRouteRealization},
    evidence::{
        MercatorEvidenceMeta, MercatorObjectiveKey, MercatorRouteSupport, MercatorSupportState,
    },
    MercatorEngine, MERCATOR_ENGINE_ID,
};

impl RoutingEngine for MercatorEngine {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        let route_id = *input.handle.route_id();
        let backend_route_id = input.admission.backend_ref.backend_route_id.clone();
        let active = corridor::active_route_from_backend(backend_route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        let installation = corridor::materialize_admitted(input)?;
        self.record_fresh_route_support(
            route_id,
            &active,
            installation.health.last_validated_at_tick,
        );
        self.record_route_objective_materialized(route_id, active.destination.clone());
        self.active_routes.insert(route_id, active);
        self.refresh_objective_presence_diagnostics(false);
        self.refresh_broker_diagnostics(
            installation
                .materialization_proof
                .witness
                .value
                .topology_epoch,
            installation.health.last_validated_at_tick,
        );
        Ok(installation)
    }

    fn route_commitments(&self, _route: &MaterializedRoute) -> Vec<RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(&mut self, tick: &RoutingTickContext) -> Result<RoutingTickOutcome, RouteError> {
        self.latest_topology_epoch = Some(tick.topology.value.epoch);
        self.latest_topology = Some(tick.topology.clone());
        self.refresh_objective_presence_diagnostics(true);
        self.refresh_broker_diagnostics(tick.topology.value.epoch, tick.topology.observed_at_tick);
        self.refresh_custody_diagnostics();
        self.refresh_active_stale_diagnostics(&tick.topology);
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: RoutingTickChange::NoChange,
            next_tick_hint: RoutingTickHint::WithinTicks(self.config.bounds.engine_tick_within),
        })
    }

    fn maintain_route(
        &mut self,
        identity: &PublishedRouteRecord,
        runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let route_id = *identity.route_id();
        let topology = self
            .latest_topology
            .clone()
            .ok_or(RouteRuntimeError::Invalidated)?;
        if self.route_is_viable(&route_id, &topology)? {
            runtime.health.reachability_state = ReachabilityState::Reachable;
            runtime.health.last_validated_at_tick = topology.observed_at_tick;
            return Ok(continued_result());
        }
        self.mark_route_stale(route_id, topology.observed_at_tick)?;
        self.evidence.record_repair_attempt();
        if let Some(repair) = self.repair_route(route_id, &topology)? {
            let recovery_rounds = self.finish_repair(route_id, repair, runtime, &topology)?;
            self.evidence.record_repair_success(recovery_rounds);
            self.refresh_active_stale_diagnostics(&topology);
            return Ok(repaired_result());
        }
        self.evidence.withdraw_route_support(
            route_id,
            topology.value.epoch,
            topology.observed_at_tick,
        );
        runtime.last_lifecycle_event = RouteLifecycleEvent::Expired;
        runtime.health.reachability_state = ReachabilityState::Unreachable;
        runtime.health.last_validated_at_tick = topology.observed_at_tick;
        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Expired,
            outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
        })
    }

    fn teardown(&mut self, route_id: &RouteId) {
        self.active_routes.remove(route_id);
        self.remove_route_objective(route_id);
        if let Some(topology) = self.latest_topology.clone() {
            self.refresh_objective_presence_diagnostics(false);
            self.refresh_broker_diagnostics(topology.value.epoch, topology.observed_at_tick);
        }
    }
}

impl RouterManagedEngine for MercatorEngine {
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        _payload: &[u8],
    ) -> Result<(), RouteError> {
        if self.active_routes.contains_key(route_id) {
            Ok(())
        } else {
            Err(RouteSelectionError::NoCandidate.into())
        }
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        Ok(false)
    }

    fn restore_route_runtime_with_record_for_router(
        &mut self,
        route: &MaterializedRoute,
        topology: &Observation<Configuration>,
    ) -> Result<bool, RouteError> {
        if route.identity.admission.backend_ref.engine != MERCATOR_ENGINE_ID {
            return Ok(false);
        }
        let Some(active) = corridor::active_route_from_backend(
            route
                .identity
                .admission
                .backend_ref
                .backend_route_id
                .clone(),
        ) else {
            return Ok(false);
        };
        self.latest_topology_epoch = Some(topology.value.epoch);
        self.latest_topology = Some(topology.clone());
        self.record_route_objective_materialized(
            route.identity.stamp.route_id,
            active.destination.clone(),
        );
        self.active_routes
            .insert(route.identity.stamp.route_id, active);
        self.refresh_objective_presence_diagnostics(false);
        self.refresh_broker_diagnostics(topology.value.epoch, topology.observed_at_tick);
        Ok(true)
    }

    fn analysis_snapshot_for_router(
        &self,
        _active_routes: &[MaterializedRoute],
    ) -> Option<Box<dyn std::any::Any>> {
        Some(Box::new(self.router_analysis_snapshot()))
    }
}

impl MercatorEngine {
    fn record_fresh_route_support(
        &mut self,
        route_id: RouteId,
        active: &corridor::ActiveMercatorRoute,
        now: Tick,
    ) {
        self.evidence.record_route_support(MercatorRouteSupport {
            route_id,
            objective: MercatorObjectiveKey::destination(active.destination.clone()),
            state: MercatorSupportState::Fresh,
            support_score: active.support_score,
            last_loss_epoch: None,
            stale_started_at: None,
            meta: MercatorEvidenceMeta::new(
                active.topology_epoch,
                now,
                self.config.bounds.evidence_validity,
                OrderStamp(u64::try_from(self.active_routes.len()).unwrap_or(u64::MAX)),
            ),
        });
    }

    fn refresh_active_stale_diagnostics(&mut self, topology: &Observation<Configuration>) {
        let now = topology.observed_at_tick;
        let mut count = 0_u32;
        let mut rounds = 0_u32;
        let route_ids = self.active_routes.keys().copied().collect::<Vec<_>>();
        for route_id in route_ids {
            if self.route_is_viable(&route_id, topology).unwrap_or(false) {
                if let Some(active) = self.active_routes.get_mut(&route_id) {
                    active.stale_started_at = None;
                }
                continue;
            }
            if let Some(active) = self.active_routes.get_mut(&route_id) {
                let started_at = *active.stale_started_at.get_or_insert(now);
                count = count.saturating_add(1);
                rounds = rounds.saturating_add(stale_rounds_since(started_at, now));
            }
        }
        self.evidence.record_active_stale_routes(count, rounds);
    }

    fn route_is_viable(
        &self,
        route_id: &RouteId,
        topology: &Observation<Configuration>,
    ) -> Result<bool, RouteError> {
        let active = self
            .active_routes
            .get(route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        Ok(corridor::path_is_viable(
            &active.primary_path,
            topology,
            &self.evidence,
        ))
    }

    fn mark_route_stale(&mut self, route_id: RouteId, now: Tick) -> Result<Tick, RouteError> {
        let active = self
            .active_routes
            .get_mut(&route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        Ok(*active.stale_started_at.get_or_insert(now))
    }

    fn repair_route(
        &self,
        route_id: RouteId,
        topology: &Observation<Configuration>,
    ) -> Result<Option<MercatorRouteRealization>, RouteError> {
        let active = self
            .active_routes
            .get(&route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        Ok(corridor::repair_realization_from_alternates(
            self.local_node_id,
            active,
            topology,
            &self.config,
            &self.evidence,
        ))
    }

    fn finish_repair(
        &mut self,
        route_id: RouteId,
        repair: MercatorRouteRealization,
        runtime: &mut RouteRuntimeState,
        topology: &Observation<Configuration>,
    ) -> Result<u32, RouteError> {
        let (previous_brokers, destination) = {
            let active = self
                .active_routes
                .get(&route_id)
                .ok_or(RouteRuntimeError::Invalidated)?;
            (
                broker_nodes_for_path(&active.primary_path),
                active.destination.clone(),
            )
        };
        let refreshed_alternates = corridor::alternate_paths_for_repair(
            self.local_node_id,
            &destination,
            &repair.path,
            topology,
            &self.config,
            &self.evidence,
        );
        let refreshed_next_hops = refreshed_alternates
            .iter()
            .filter_map(|path| path.get(1).copied())
            .collect::<Vec<_>>();
        let active = self
            .active_routes
            .get_mut(&route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        let next_brokers = broker_nodes_for_path(&repair.path);
        if previous_brokers != next_brokers {
            self.evidence.record_broker_switch();
        }
        let started_at = active
            .stale_started_at
            .take()
            .unwrap_or(topology.observed_at_tick);
        active.primary_path = repair.path;
        active.support_score = repair.support_score;
        active.topology_epoch = topology.value.epoch;
        active.alternate_paths = refreshed_alternates;
        active.alternate_next_hops = refreshed_next_hops;
        runtime.last_lifecycle_event = RouteLifecycleEvent::Repaired;
        runtime.health.reachability_state = ReachabilityState::Reachable;
        runtime.health.stability_score =
            jacquard_core::HealthScore(u32::from(repair.support_score));
        runtime.health.last_validated_at_tick = topology.observed_at_tick;
        self.evidence.mark_route_support_fresh(
            route_id,
            repair.support_score,
            MercatorEvidenceMeta::new(
                topology.value.epoch,
                topology.observed_at_tick,
                self.config.bounds.evidence_validity,
                OrderStamp(u64::try_from(self.active_routes.len()).unwrap_or(u64::MAX)),
            ),
        );
        self.refresh_broker_diagnostics(topology.value.epoch, topology.observed_at_tick);
        Ok(stale_rounds_since(started_at, topology.observed_at_tick))
    }
}

fn continued_result() -> RouteMaintenanceResult {
    RouteMaintenanceResult {
        event: RouteLifecycleEvent::Activated,
        outcome: RouteMaintenanceOutcome::Continued,
    }
}

fn repaired_result() -> RouteMaintenanceResult {
    RouteMaintenanceResult {
        event: RouteLifecycleEvent::Repaired,
        outcome: RouteMaintenanceOutcome::Repaired,
    }
}

fn stale_rounds_since(started_at: Tick, now: Tick) -> u32 {
    u32::try_from(now.0.saturating_sub(started_at.0)).unwrap_or(u32::MAX)
}
