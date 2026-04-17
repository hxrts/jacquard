//! `RoutingEngine` and `RouterManagedEngine` impls for `ScatterEngine`.

use std::collections::BTreeSet;

use jacquard_core::{
    Configuration, DestinationId, Fact, FactBasis, HoldItemCount, Limit, MaterializedRoute, NodeId,
    Observation, PublishedRouteRecord, ReachabilityState, RouteCommitment, RouteError, RouteId,
    RouteInstallation, RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
    RouteMaterializationProof, RouteProgressContract, RouteProgressState, RouteRuntimeError,
    RouteRuntimeState, RouteSelectionError, RoutingTickChange, RoutingTickContext, RoutingTickHint,
    RoutingTickOutcome, Tick, TransportObservation,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, TimeEffects, TransportSenderEffects};

use crate::{
    public_state::{ScatterAction, ScatterLocalSummary, ScatterRegime, ScatterRouteProgress},
    support::{
        action_for_delta, classify_regime, contact_supports_payload, decode_backend_token,
        decode_packet, direct_neighbors, encode_packet, expiry_for_urgency,
        initial_budget_for_urgency, link_is_usable, local_objective_match, objective_supported,
        peer_score, size_class_for_payload, urgency_from_payload_len, ActiveScatterRoute,
        ScatterMessageId, ScatterWirePacket, StoredScatterMessage,
    },
    ScatterEngine, SCATTER_ENGINE_ID,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScatterForwardIntent {
    message_id: ScatterMessageId,
    neighbor: NodeId,
    action: ScatterAction,
    packet: ScatterWirePacket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScatterRoundState {
    stored_messages: std::collections::BTreeMap<ScatterMessageId, StoredScatterMessage>,
    current_regime: ScatterRegime,
    last_local_summary: ScatterLocalSummary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScatterRoundInput {
    topology: Observation<Configuration>,
    local_node_id: NodeId,
    peer_observations: std::collections::BTreeMap<NodeId, crate::support::PeerObservationState>,
    config: crate::ScatterEngineConfig,
    now_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScatterRoundTransition {
    next_state: ScatterRoundState,
    intents: Vec<ScatterForwardIntent>,
    progressed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScatterMaintenanceInput {
    runtime: RouteRuntimeState,
    has_direct: bool,
    progress: ScatterRouteProgress,
    now_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScatterMaintenanceTransition {
    next_runtime: RouteRuntimeState,
    result: RouteMaintenanceResult,
}

// long-block-exception: diffusion planning keeps the bounded custody,
// carrier-selection, and transfer-intent ladder together in one reducer.
fn plan_diffusion_round(
    input: &ScatterRoundInput,
    state: ScatterRoundState,
) -> ScatterRoundTransition {
    let Some(local_node) = input.topology.value.nodes.get(&input.local_node_id) else {
        return ScatterRoundTransition {
            next_state: state,
            intents: Vec::new(),
            progressed: false,
        };
    };
    let (regime, local_summary) = classify_regime(
        &input.topology,
        input.local_node_id,
        local_node,
        &input.peer_observations,
        &input.config,
    );
    let mut next_messages = state.stored_messages;
    let mut intents = Vec::new();
    let mut progressed = false;
    let message_ids = next_messages.keys().cloned().collect::<Vec<_>>();
    for message_id in message_ids {
        let Some(stored) = next_messages.get(&message_id).cloned() else {
            continue;
        };
        if crate::support::packet_expired(&stored.packet, input.now_tick) {
            next_messages.remove(&message_id);
            progressed = true;
            continue;
        }
        if stored.delivered_locally {
            continue;
        }
        let local_score = peer_score(
            &input.topology,
            input.local_node_id,
            input.local_node_id,
            &stored.packet.destination,
            stored.packet.service_kind,
        );
        for (neighbor, link) in direct_neighbors(&input.topology, input.local_node_id) {
            if !link_is_usable(link)
                || stored.known_holder_nodes.contains(&neighbor)
                || !contact_supports_payload(link, stored.packet.payload.len(), &input.config)
                || (regime == ScatterRegime::Dense
                    && !crate::support::diversity_gate(
                        &input.topology,
                        input.local_node_id,
                        neighbor,
                    ))
            {
                continue;
            }
            let delta = peer_score(
                &input.topology,
                input.local_node_id,
                neighbor,
                &stored.packet.destination,
                stored.packet.service_kind,
            ) - local_score;
            match action_for_delta(regime, delta, &stored.packet, &input.config) {
                ScatterAction::KeepCarrying => {}
                ScatterAction::Replicate => {
                    let peer_budget = stored.packet.copy_budget / 2;
                    if peer_budget == 0 {
                        continue;
                    }
                    let mut packet = stored.packet.clone();
                    packet.copy_budget = peer_budget;
                    intents.push(ScatterForwardIntent {
                        message_id: stored.packet.message_id.clone(),
                        neighbor,
                        action: ScatterAction::Replicate,
                        packet,
                    });
                    progressed = true;
                }
                ScatterAction::PreferentialHandoff => {
                    intents.push(ScatterForwardIntent {
                        message_id: stored.packet.message_id.clone(),
                        neighbor,
                        action: ScatterAction::PreferentialHandoff,
                        packet: stored.packet.clone(),
                    });
                    progressed = true;
                }
            }
        }
    }
    ScatterRoundTransition {
        next_state: ScatterRoundState {
            stored_messages: next_messages,
            current_regime: regime,
            last_local_summary: local_summary,
        },
        intents,
        progressed,
    }
}

fn reduce_maintenance(input: ScatterMaintenanceInput) -> ScatterMaintenanceTransition {
    let mut next_runtime = input.runtime;
    next_runtime.progress.last_progress_at_tick = input.now_tick;
    next_runtime.health.last_validated_at_tick = input.now_tick;
    next_runtime.health.reachability_state = if input.has_direct {
        ReachabilityState::Reachable
    } else {
        ReachabilityState::Unreachable
    };
    let result = if !input.has_direct && input.progress.retained_message_count > 0 {
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::EnteredPartitionMode,
            outcome: RouteMaintenanceOutcome::HoldFallback {
                trigger: RouteMaintenanceTrigger::PartitionDetected,
                retained_object_count: HoldItemCount(input.progress.retained_message_count),
            },
        }
    } else if input.has_direct && input.progress.last_action == ScatterAction::PreferentialHandoff {
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::RecoveredFromPartition,
            outcome: RouteMaintenanceOutcome::Repaired,
        }
    } else {
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::Activated,
            outcome: RouteMaintenanceOutcome::Continued,
        }
    };
    ScatterMaintenanceTransition {
        next_runtime,
        result,
    }
}

fn restored_active_route(route: &MaterializedRoute) -> Option<ActiveScatterRoute> {
    let token = decode_backend_token(&route.identity.admission.backend_ref.backend_route_id)?;
    Some(ActiveScatterRoute {
        destination: token.destination,
        service_kind: token.service_kind,
        backend_route_id: route
            .identity
            .admission
            .backend_ref
            .backend_route_id
            .clone(),
        installed_at_tick: route.identity.stamp.materialized_at_tick,
        progress: ScatterRouteProgress::default(),
    })
}

impl<Transport, Effects> ScatterEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
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

    fn send_packet_to_neighbor(
        &mut self,
        link: &jacquard_core::Link,
        packet: &ScatterWirePacket,
    ) -> Result<(), RouteError> {
        let payload = encode_packet(packet);
        self.transport.send_transport(&link.endpoint, &payload)?;
        Ok(())
    }

    fn apply_forward_intent_success(&mut self, intent: &ScatterForwardIntent, now: Tick) {
        if let Some(stored) = self.stored_messages.get_mut(&intent.message_id) {
            stored.known_holder_nodes.insert(intent.neighbor);
            match intent.action {
                ScatterAction::KeepCarrying => {}
                ScatterAction::Replicate => {
                    stored.packet.copy_budget = stored
                        .packet
                        .copy_budget
                        .saturating_sub(intent.packet.copy_budget);
                    stored.last_action = ScatterAction::Replicate;
                }
                ScatterAction::PreferentialHandoff => {
                    stored.preferential_handoff_target = Some(intent.neighbor);
                    stored.last_action = ScatterAction::PreferentialHandoff;
                }
            }
            stored.last_progress_at_tick = now;
        }
    }

    fn run_diffusion_round(
        &mut self,
        topology: &jacquard_core::Observation<Configuration>,
    ) -> Result<bool, RouteError> {
        let now = self.effects.now_tick();
        let transition = plan_diffusion_round(
            &ScatterRoundInput {
                topology: topology.clone(),
                local_node_id: self.local_node_id,
                peer_observations: self.peer_observations.clone(),
                config: self.config,
                now_tick: now,
            },
            ScatterRoundState {
                stored_messages: std::mem::take(&mut self.stored_messages),
                current_regime: self.current_regime,
                last_local_summary: self.last_local_summary,
            },
        );
        self.stored_messages = transition.next_state.stored_messages;
        self.current_regime = transition.next_state.current_regime;
        self.last_local_summary = transition.next_state.last_local_summary;
        let neighbors = direct_neighbors(topology, self.local_node_id)
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        for intent in &transition.intents {
            let Some(link) = neighbors.get(&intent.neighbor) else {
                continue;
            };
            self.send_packet_to_neighbor(link, &intent.packet)?;
            self.apply_forward_intent_success(intent, now);
        }
        Ok(transition.progressed)
    }

    fn route_progress(
        stored_messages: &std::collections::BTreeMap<ScatterMessageId, StoredScatterMessage>,
        current_regime: ScatterRegime,
        destination: &DestinationId,
        service_kind: jacquard_core::RouteServiceKind,
    ) -> ScatterRouteProgress {
        let mut retained_message_count = 0_u32;
        let mut delivered_message_count = 0_u32;
        let mut last_action = ScatterAction::KeepCarrying;
        let mut last_progress_at_tick = None;
        for message in stored_messages.values() {
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
        ScatterRouteProgress {
            retained_message_count,
            delivered_message_count,
            last_regime: current_regime,
            last_action,
            last_progress_at_tick,
        }
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
        let progress = Self::route_progress(
            &self.stored_messages,
            self.current_regime,
            destination,
            service_kind,
        );
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
        let progress = Self::route_progress(
            &self.stored_messages,
            self.current_regime,
            &token.destination,
            token.service_kind,
        );
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
        let transition = reduce_maintenance(ScatterMaintenanceInput {
            runtime: runtime.clone(),
            has_direct,
            progress,
            now_tick: self.effects.now_tick(),
        });
        *runtime = transition.next_runtime;
        Ok(transition.result)
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

    fn restore_route_runtime_with_record_for_router(
        &mut self,
        route: &MaterializedRoute,
        topology: &Observation<Configuration>,
    ) -> Result<bool, RouteError> {
        let Some(mut active_route) = restored_active_route(route) else {
            return Ok(false);
        };
        self.latest_topology = Some(topology.clone());
        active_route.progress = Self::route_progress(
            &self.stored_messages,
            self.current_regime,
            &active_route.destination,
            active_route.service_kind,
        );
        self.active_routes
            .insert(route.identity.stamp.route_id, active_route);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_adapter::opaque_endpoint;
    use jacquard_core::{
        ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
        Environment, FactSourceClass, LinkEndpoint, MaterializedRoute, OriginAuthenticationClass,
        PublicationId, RatioPermille, RouteEpoch, RouteHandle, RouteLease, RoutePartitionClass,
        RouteProtectionClass, RouteRepairClass, RoutingEvidenceClass, SelectedRoutingParameters,
        TimeWindow, TransportKind,
    };
    use jacquard_mem_link_profile::{
        InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
    };
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::{RouterManagedEngine, RoutingEngine, RoutingEnginePlanner};

    use super::*;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
    }

    // long-block-exception: the test topology fixture keeps one complete
    // deterministic scatter contact sample in one place for runtime tests.
    fn topology(with_direct_link: bool) -> Observation<Configuration> {
        let mut links = BTreeMap::new();
        if with_direct_link {
            links.insert(
                (node(1), node(2)),
                LinkPreset::active(LinkPresetOptions::new(endpoint(2), Tick(1))).build(),
            );
            links.insert(
                (node(2), node(1)),
                LinkPreset::active(LinkPresetOptions::new(endpoint(1), Tick(1))).build(),
            );
        }
        Observation {
            value: Configuration {
                epoch: RouteEpoch(2),
                nodes: BTreeMap::from([
                    (
                        node(1),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(1), ControllerId([1; 32])),
                                endpoint(1),
                                Tick(1),
                            ),
                            &SCATTER_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(2),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(2), ControllerId([2; 32])),
                                endpoint(2),
                                Tick(1),
                            ),
                            &SCATTER_ENGINE_ID,
                        )
                        .build(),
                    ),
                ]),
                links,
                environment: Environment {
                    reachable_neighbor_count: if with_direct_link { 1 } else { 0 },
                    churn_permille: RatioPermille(50),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    fn profile(partition: RoutePartitionClass) -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition,
            },
            deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn objective(partition: RoutePartitionClass) -> jacquard_core::RoutingObjective {
        jacquard_core::RoutingObjective {
            destination: DestinationId::Node(node(2)),
            service_kind: jacquard_core::RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
            latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(100)),
            protection_priority: jacquard_core::PriorityPoints(10),
            connectivity_priority: jacquard_core::PriorityPoints(10),
        }
    }

    fn materialization_input(
        admission: jacquard_core::RouteAdmission,
        route_id: RouteId,
    ) -> jacquard_core::RouteMaterializationInput {
        let lease = RouteLease {
            owner_node_id: node(1),
            lease_epoch: RouteEpoch(2),
            valid_for: TimeWindow::new(Tick(1), Tick(9)).expect("lease window"),
        };
        jacquard_core::RouteMaterializationInput {
            handle: RouteHandle {
                stamp: jacquard_core::RouteIdentityStamp {
                    route_id,
                    topology_epoch: lease.lease_epoch,
                    materialized_at_tick: lease.valid_for.start_tick(),
                    publication_id: PublicationId([9; 16]),
                },
            },
            admission,
            lease,
        }
    }

    #[test]
    fn planner_snapshot_captures_local_policy_surface() {
        let engine = ScatterEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let snapshot = engine.planner_snapshot();

        assert_eq!(snapshot.local_node_id, node(1));
        assert_eq!(snapshot.config, engine.config());
        assert_eq!(snapshot.current_regime, engine.current_regime());
        assert_eq!(snapshot.last_local_summary, engine.last_local_summary());
    }

    #[test]
    fn round_reducer_plans_forward_intent_for_retained_message() {
        let mut topology = topology(true);
        topology.value.links.remove(&(node(2), node(1)));
        let mut engine = ScatterEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        engine.latest_topology = Some(topology.clone());
        let packet = ScatterWirePacket {
            message_id: ScatterMessageId([7; 16]),
            destination: DestinationId::Node(node(2)),
            service_kind: jacquard_core::RouteServiceKind::Move,
            created_tick: Tick(1),
            expiry_after_ms: DurationMs(50),
            copy_budget: 2,
            urgency_class: crate::ScatterUrgencyClass::Normal,
            size_class: crate::ScatterSizeClass::Small,
            payload: b"msg".to_vec(),
        };
        engine.store_packet(packet, None, Tick(1));

        let transition = plan_diffusion_round(
            &ScatterRoundInput {
                topology: topology.clone(),
                local_node_id: node(1),
                peer_observations: engine.peer_observations.clone(),
                config: engine.config(),
                now_tick: Tick(1),
            },
            ScatterRoundState {
                stored_messages: engine.stored_messages.clone(),
                current_regime: engine.current_regime,
                last_local_summary: engine.last_local_summary,
            },
        );

        assert!(transition.progressed);
        assert_eq!(transition.intents.len(), 1);
        assert_eq!(transition.intents[0].neighbor, node(2));
        assert_eq!(
            transition.intents[0].action,
            ScatterAction::PreferentialHandoff
        );
    }

    #[test]
    // long-block-exception: the parity test keeps the full diffusion
    // projection-vs-wrapper setup together for route-maintenance auditing.
    fn maintenance_reducer_matches_wrapper_projection() {
        let topology = topology(false);
        let mut engine = ScatterEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        engine
            .engine_tick(&RoutingTickContext::new(topology.clone()))
            .expect("seed topology");
        let objective = objective(RoutePartitionClass::PartitionTolerant);
        let profile = profile(RoutePartitionClass::PartitionTolerant);
        let candidate = engine
            .candidate_routes(&objective, &profile, &topology)
            .pop()
            .expect("candidate");
        let admission = engine
            .admit_route(&objective, &profile, candidate.clone(), &topology)
            .expect("admission");
        let input = materialization_input(admission, candidate.route_id);
        let installation = engine
            .materialize_route(input.clone())
            .expect("materialize");
        engine
            .forward_payload_for_router(&candidate.route_id, b"scatter-payload")
            .expect("forward");
        let mut route = MaterializedRoute::from_installation(input, installation);
        let progress = ScatterEngine::<InMemoryTransport, InMemoryRuntimeEffects>::route_progress(
            &engine.stored_messages,
            engine.current_regime,
            &DestinationId::Node(node(2)),
            jacquard_core::RouteServiceKind::Move,
        );
        let reduced = reduce_maintenance(ScatterMaintenanceInput {
            runtime: route.runtime.clone(),
            has_direct: false,
            progress,
            now_tick: Tick(1),
        });

        let wrapper_result = engine
            .maintain_route(
                &route.identity,
                &mut route.runtime,
                jacquard_core::RouteMaintenanceTrigger::PartitionDetected,
            )
            .expect("maintenance");

        assert_eq!(route.runtime, reduced.next_runtime);
        assert_eq!(wrapper_result, reduced.result);
    }

    #[test]
    // long-block-exception: the restore test keeps route reconstruction,
    // router-led restore, and forward-after-restore checks together.
    fn restore_reconstructs_active_route_from_materialized_route() {
        let topology = topology(true);
        let mut engine = ScatterEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        engine
            .engine_tick(&RoutingTickContext::new(topology.clone()))
            .expect("seed topology");
        let objective = objective(RoutePartitionClass::PartitionTolerant);
        let profile = profile(RoutePartitionClass::PartitionTolerant);
        let candidate = engine
            .candidate_routes(&objective, &profile, &topology)
            .pop()
            .expect("candidate");
        let admission = engine
            .admit_route(&objective, &profile, candidate.clone(), &topology)
            .expect("admission");
        let input = materialization_input(admission, candidate.route_id);
        let installation = engine
            .materialize_route(input.clone())
            .expect("materialize");
        let route = MaterializedRoute::from_installation(input, installation);

        engine.active_routes.clear();
        let restored = engine
            .restore_route_runtime_with_record_for_router(&route, &topology)
            .expect("restore");

        assert!(restored);
        assert_eq!(
            engine.active_routes.get(&route.identity.stamp.route_id),
            Some(&ActiveScatterRoute {
                destination: DestinationId::Node(node(2)),
                service_kind: jacquard_core::RouteServiceKind::Move,
                backend_route_id: route
                    .identity
                    .admission
                    .backend_ref
                    .backend_route_id
                    .clone(),
                installed_at_tick: Tick(1),
                progress: ScatterRouteProgress::default(),
            })
        );
    }
}
