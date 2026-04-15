//! `RoutingEngine` and `RouterManagedEngine` impls for `ScatterEngine`.

use std::collections::BTreeSet;

use jacquard_core::{
    Configuration, DestinationId, Fact, FactBasis, HoldItemCount, Limit, NodeId,
    PublishedRouteRecord, ReachabilityState, RouteCommitment, RouteError, RouteId,
    RouteInstallation, RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
    RouteMaterializationProof, RouteProgressContract, RouteProgressState, RouteRuntimeError,
    RouteRuntimeState, RouteSelectionError, RoutingTickChange, RoutingTickContext, RoutingTickHint,
    RoutingTickOutcome, Tick, TransportObservation,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, TimeEffects, TransportSenderEffects};

use crate::{
    public_state::{ScatterAction, ScatterRegime, ScatterRouteProgress},
    support::{
        action_for_delta, classify_regime, contact_supports_payload, decode_backend_token,
        decode_packet, direct_neighbors, encode_packet, expiry_for_urgency,
        initial_budget_for_urgency, link_is_usable, local_objective_match, objective_supported,
        peer_score, size_class_for_payload, urgency_from_payload_len, ActiveScatterRoute,
        ScatterMessageId, ScatterWirePacket, StoredScatterMessage,
    },
    ScatterEngine, SCATTER_ENGINE_ID,
};

impl<Transport, Effects> ScatterEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn local_node<'a>(
        &self,
        topology: &'a jacquard_core::Observation<Configuration>,
    ) -> Option<&'a jacquard_core::Node> {
        topology.value.nodes.get(&self.local_node_id)
    }

    fn progress_for_route(
        &self,
        destination: &DestinationId,
        service_kind: jacquard_core::RouteServiceKind,
    ) -> crate::public_state::ScatterRouteProgress {
        let mut retained_message_count = 0_u32;
        let mut delivered_message_count = 0_u32;
        let mut last_action = ScatterAction::KeepCarrying;
        let mut last_progress_at_tick = None;
        for message in self.stored_messages.values() {
            if &message.packet.destination != destination
                || message.packet.service_kind != service_kind
            {
                continue;
            }
            retained_message_count = retained_message_count.saturating_add(1);
            if message.delivered_locally {
                delivered_message_count = delivered_message_count.saturating_add(1);
            }
            last_action = message.last_action;
            last_progress_at_tick = Some(message.last_progress_at_tick);
        }
        crate::public_state::ScatterRouteProgress {
            retained_message_count,
            delivered_message_count,
            last_regime: self.current_regime,
            last_action,
            last_progress_at_tick,
        }
    }

    fn update_peer_observation(&mut self, peer_node_id: NodeId, observed_at_tick: Tick) {
        let entry = self.peer_observations.entry(peer_node_id).or_default();
        let is_novel = entry.last_seen_tick.is_none_or(|last_seen| {
            observed_at_tick.0.saturating_sub(last_seen.0) > self.config.regime.history_window_ticks
        });
        entry.encounter_count = entry.encounter_count.saturating_add(1);
        entry.last_seen_tick = Some(observed_at_tick);
        if is_novel {
            entry.recent_novelty_count = entry.recent_novelty_count.saturating_add(1);
        }
    }

    fn next_message_id(&mut self) -> ScatterMessageId {
        self.next_message_sequence = self.next_message_sequence.saturating_add(1);
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&self.local_node_id.0[..8]);
        bytes[8..].copy_from_slice(&self.next_message_sequence.to_be_bytes());
        ScatterMessageId(bytes)
    }

    fn store_packet(
        &mut self,
        packet: ScatterWirePacket,
        injected_by_route_id: Option<RouteId>,
        observed_at_tick: Tick,
    ) {
        let message_id = packet.message_id.clone();
        let delivered_locally = self.latest_topology.as_ref().is_some_and(|topology| {
            local_objective_match(
                topology,
                self.local_node_id,
                &packet.destination,
                packet.service_kind,
                observed_at_tick,
            )
        });
        self.seen_messages.insert(message_id.clone());
        self.stored_messages.insert(
            message_id,
            StoredScatterMessage {
                packet,
                known_holder_nodes: BTreeSet::new(),
                injected_by_route_id,
                last_action: ScatterAction::KeepCarrying,
                last_progress_at_tick: observed_at_tick,
                preferential_handoff_target: None,
                delivered_locally,
            },
        );
    }

    fn process_incoming_packet(
        &mut self,
        from_node_id: NodeId,
        packet: ScatterWirePacket,
        observed_at_tick: Tick,
    ) {
        self.update_peer_observation(from_node_id, observed_at_tick);
        let message_id = packet.message_id.clone();
        if self.seen_messages.contains(&message_id) {
            return;
        }
        self.store_packet(packet, None, observed_at_tick);
        if let Some(stored) = self.stored_messages.get_mut(&message_id) {
            stored.known_holder_nodes.insert(from_node_id);
        }
    }

    fn forward_packet_to_neighbor(
        &mut self,
        neighbor: NodeId,
        link: &jacquard_core::Link,
        packet: &ScatterWirePacket,
    ) -> Result<(), RouteError> {
        let payload = encode_packet(packet);
        self.transport.send_transport(&link.endpoint, &payload)?;
        if let Some(stored) = self.stored_messages.get_mut(&packet.message_id) {
            stored.known_holder_nodes.insert(neighbor);
            stored.last_progress_at_tick = self.effects.now_tick();
        }
        Ok(())
    }

    fn refresh_local_regime(
        &mut self,
        topology: &jacquard_core::Observation<Configuration>,
    ) -> Option<ScatterRegime> {
        let local_node = self.local_node(topology)?;
        let (regime, local_summary) = classify_regime(
            topology,
            self.local_node_id,
            local_node,
            &self.peer_observations,
            &self.config,
        );
        self.current_regime = regime;
        self.last_local_summary = local_summary;
        Some(regime)
    }

    fn process_diffusion_message(
        &mut self,
        topology: &jacquard_core::Observation<Configuration>,
        regime: ScatterRegime,
        message_id: &ScatterMessageId,
        now: Tick,
    ) -> Result<bool, RouteError> {
        let Some(stored) = self.stored_messages.get(message_id).cloned() else {
            return Ok(false);
        };
        if crate::support::packet_expired(&stored.packet, now) {
            self.stored_messages.remove(message_id);
            return Ok(true);
        }
        if stored.delivered_locally {
            return Ok(false);
        }

        let local_score = peer_score(
            topology,
            self.local_node_id,
            self.local_node_id,
            &stored.packet.destination,
            stored.packet.service_kind,
        );
        let mut progressed = false;
        for (neighbor, link) in direct_neighbors(topology, self.local_node_id) {
            if self.skip_diffusion_neighbor(topology, regime, &stored, neighbor, link) {
                continue;
            }
            if self.forward_scatter_action(
                topology,
                regime,
                &stored,
                local_score,
                neighbor,
                link,
            )? {
                progressed = true;
            }
        }
        Ok(progressed)
    }

    fn skip_diffusion_neighbor(
        &self,
        topology: &jacquard_core::Observation<Configuration>,
        regime: ScatterRegime,
        stored: &StoredScatterMessage,
        neighbor: NodeId,
        link: &jacquard_core::Link,
    ) -> bool {
        !link_is_usable(link)
            || stored.known_holder_nodes.contains(&neighbor)
            || !contact_supports_payload(link, stored.packet.payload.len(), &self.config)
            || (regime == ScatterRegime::Dense
                && !crate::support::diversity_gate(topology, self.local_node_id, neighbor))
    }

    fn forward_scatter_action(
        &mut self,
        topology: &jacquard_core::Observation<Configuration>,
        regime: ScatterRegime,
        stored: &StoredScatterMessage,
        local_score: i32,
        neighbor: NodeId,
        link: &jacquard_core::Link,
    ) -> Result<bool, RouteError> {
        let delta = peer_score(
            topology,
            self.local_node_id,
            neighbor,
            &stored.packet.destination,
            stored.packet.service_kind,
        ) - local_score;
        match action_for_delta(regime, delta, &stored.packet, &self.config) {
            ScatterAction::KeepCarrying => Ok(false),
            ScatterAction::Replicate => self.replicate_packet(stored, neighbor, link),
            ScatterAction::PreferentialHandoff => self.preferential_handoff(stored, neighbor, link),
        }
    }

    fn replicate_packet(
        &mut self,
        stored: &StoredScatterMessage,
        neighbor: NodeId,
        link: &jacquard_core::Link,
    ) -> Result<bool, RouteError> {
        let peer_budget = stored.packet.copy_budget / 2;
        if peer_budget == 0 {
            return Ok(false);
        }

        let mut packet = stored.packet.clone();
        packet.copy_budget = peer_budget;
        self.forward_packet_to_neighbor(neighbor, link, &packet)?;
        let now = self.effects.now_tick();
        if let Some(local) = self.stored_messages.get_mut(&stored.packet.message_id) {
            local.packet.copy_budget = local.packet.copy_budget.saturating_sub(peer_budget);
            local.last_action = ScatterAction::Replicate;
            local.last_progress_at_tick = now;
        }
        Ok(true)
    }

    fn preferential_handoff(
        &mut self,
        stored: &StoredScatterMessage,
        neighbor: NodeId,
        link: &jacquard_core::Link,
    ) -> Result<bool, RouteError> {
        self.forward_packet_to_neighbor(neighbor, link, &stored.packet)?;
        let now = self.effects.now_tick();
        if let Some(local) = self.stored_messages.get_mut(&stored.packet.message_id) {
            local.preferential_handoff_target = Some(neighbor);
            local.last_action = ScatterAction::PreferentialHandoff;
            local.last_progress_at_tick = now;
        }
        Ok(true)
    }

    fn run_diffusion_round(
        &mut self,
        topology: &jacquard_core::Observation<Configuration>,
    ) -> Result<bool, RouteError> {
        let Some(regime) = self.refresh_local_regime(topology) else {
            return Ok(false);
        };
        let now = self.effects.now_tick();
        let message_ids = self.stored_messages.keys().cloned().collect::<Vec<_>>();
        let mut progressed = false;
        for message_id in message_ids {
            if self.process_diffusion_message(topology, regime, &message_id, now)? {
                progressed = true;
            }
        }
        Ok(progressed)
    }

    fn route_objective(
        identity: &PublishedRouteRecord,
        destination: DestinationId,
        service_kind: jacquard_core::RouteServiceKind,
    ) -> jacquard_core::RoutingObjective {
        jacquard_core::RoutingObjective {
            destination,
            service_kind,
            target_protection: identity.admission.objective.target_protection,
            protection_floor: identity.admission.objective.protection_floor,
            target_connectivity: identity.admission.objective.target_connectivity,
            hold_fallback_policy: identity.admission.objective.hold_fallback_policy,
            latency_budget_ms: identity.admission.objective.latency_budget_ms,
            protection_priority: identity.admission.objective.protection_priority,
            connectivity_priority: identity.admission.objective.connectivity_priority,
        }
    }

    fn refresh_route_progress(
        &mut self,
        route_id: &RouteId,
        destination: &DestinationId,
        service_kind: jacquard_core::RouteServiceKind,
        runtime: &mut RouteRuntimeState,
        has_direct: bool,
    ) -> ScatterRouteProgress {
        let progress = self.progress_for_route(destination, service_kind);
        if let Some(active) = self.active_routes.get_mut(route_id) {
            active.progress = progress;
        }
        runtime.progress.last_progress_at_tick = self.effects.now_tick();
        runtime.health.last_validated_at_tick = self.effects.now_tick();
        runtime.health.reachability_state = if has_direct {
            ReachabilityState::Reachable
        } else {
            ReachabilityState::Unreachable
        };
        progress
    }

    fn maintenance_result(
        has_direct: bool,
        progress: ScatterRouteProgress,
    ) -> RouteMaintenanceResult {
        if !has_direct && progress.retained_message_count > 0 {
            return RouteMaintenanceResult {
                event: RouteLifecycleEvent::EnteredPartitionMode,
                outcome: RouteMaintenanceOutcome::HoldFallback {
                    trigger: RouteMaintenanceTrigger::PartitionDetected,
                    retained_object_count: HoldItemCount(progress.retained_message_count),
                },
            };
        }
        if has_direct && progress.last_action == ScatterAction::PreferentialHandoff {
            return RouteMaintenanceResult {
                event: RouteLifecycleEvent::RecoveredFromPartition,
                outcome: RouteMaintenanceOutcome::Repaired,
            };
        }
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::Activated,
            outcome: RouteMaintenanceOutcome::Continued,
        }
    }
}

impl<Transport, Effects> RoutingEngine for ScatterEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        if input.admission.backend_ref.engine != SCATTER_ENGINE_ID {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        let token = decode_backend_token(&input.admission.backend_ref.backend_route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        let now = self.effects.now_tick();
        let progress = self.progress_for_route(&token.destination, token.service_kind);
        self.active_routes.insert(
            *input.handle.route_id(),
            ActiveScatterRoute {
                destination: token.destination,
                service_kind: token.service_kind,
                backend_route_id: input.admission.backend_ref.backend_route_id.clone(),
                installed_at_tick: now,
                progress,
            },
        );
        Ok(RouteInstallation {
            materialization_proof: RouteMaterializationProof {
                stamp: input.handle.stamp.clone(),
                witness: Fact {
                    basis: FactBasis::Admitted,
                    value: input.admission.witness,
                    established_at_tick: now,
                },
            },
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: jacquard_core::RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: jacquard_core::HealthScore(700),
                congestion_penalty_points: jacquard_core::PenaltyPoints(250),
                last_validated_at_tick: now,
            },
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(1),
                total_step_count_max: Limit::Bounded(self.config.bounds.work_step_count_max),
                last_progress_at_tick: now,
                state: RouteProgressState::Pending,
            },
        })
    }

    fn route_commitments(&self, _route: &jacquard_core::MaterializedRoute) -> Vec<RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(&mut self, tick: &RoutingTickContext) -> Result<RoutingTickOutcome, RouteError> {
        self.latest_topology = Some(tick.topology.clone());
        let progressed = self.run_diffusion_round(&tick.topology)?;
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: if progressed {
                RoutingTickChange::PrivateStateUpdated
            } else {
                RoutingTickChange::NoChange
            },
            next_tick_hint: if progressed {
                RoutingTickHint::Immediate
            } else {
                RoutingTickHint::WithinTicks(Tick(self.config.bounds.engine_tick_within_ticks))
            },
        })
    }

    fn maintain_route(
        &mut self,
        identity: &PublishedRouteRecord,
        runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let Some((destination, service_kind)) = self
            .active_routes
            .get(identity.route_id())
            .map(|active| (active.destination.clone(), active.service_kind))
        else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let Some(topology) = self.latest_topology.as_ref() else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let objective = Self::route_objective(identity, destination.clone(), service_kind);
        if !objective_supported(topology, &objective, topology.observed_at_tick) {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        }
        let has_direct = crate::support::has_direct_match(topology, self.local_node_id, &objective);
        let progress = self.refresh_route_progress(
            identity.route_id(),
            &destination,
            service_kind,
            runtime,
            has_direct,
        );
        Ok(Self::maintenance_result(has_direct, progress))
    }

    fn teardown(&mut self, route_id: &RouteId) {
        self.active_routes.remove(route_id);
    }
}

impl<Transport, Effects> RouterManagedEngine for ScatterEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn ingest_transport_observation_for_router(
        &mut self,
        observation: &TransportObservation,
    ) -> Result<(), RouteError> {
        if let TransportObservation::PayloadReceived {
            from_node_id,
            payload,
            observed_at_tick,
            ..
        } = observation
        {
            if let Some(packet) = decode_packet(payload) {
                self.process_incoming_packet(*from_node_id, packet, *observed_at_tick);
            }
        }
        Ok(())
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        let Some((destination, service_kind)) = self
            .active_routes
            .get(route_id)
            .map(|active| (active.destination.clone(), active.service_kind))
        else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let urgency_class = urgency_from_payload_len(payload.len());
        let packet = ScatterWirePacket {
            message_id: self.next_message_id(),
            destination,
            service_kind,
            created_tick: self.effects.now_tick(),
            expiry_after_ms: expiry_for_urgency(self.config.expiry, urgency_class),
            copy_budget: initial_budget_for_urgency(self.config.budget, urgency_class),
            urgency_class,
            size_class: size_class_for_payload(payload),
            payload: payload.to_vec(),
        };
        self.store_packet(packet, Some(*route_id), self.effects.now_tick());
        if let Some(topology) = self.latest_topology.clone() {
            // allow-ignored-result: submit needs diffusion-side errors only; the progress flag is local bookkeeping.
            let _ = self.run_diffusion_round(&topology)?;
        }
        Ok(())
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        Ok(false)
    }
}
