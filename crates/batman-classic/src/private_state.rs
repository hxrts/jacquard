//! Engine-private state maintenance for `BatmanClassicEngine`.
//!
//! The classic engine derives originator observations from TQ values carried in
//! received OGMs rather than from a local Bellman-Ford computation over a
//! gossip-merged topology graph. This matches the spec's distributed implicit
//! computation: each forwarding node encodes its computed path quality in the
//! OGM before re-broadcasting, so the local node can read a neighbor's path
//! quality to any originator directly from the received advertisement.
//!
//! Key behaviours:
//! - **No Bellman-Ford** — `received_ogm_info` provides path TQ and hop count
//!   per `(originator, via_neighbor)`.
//! - **No bootstrap shortcut** — if no receive-window data exists for a path,
//!   that path is not used. The engine produces no candidates on tick 1 if it
//!   has not yet observed OGMs.
//! - **Echo-only bidirectionality** — `bidirectional_neighbor_valid` returns
//!   `true` only when an echo has been received via `bidirectional_receive_windows`.
//!   There is no topology fallback.
//! - **No TQ enrichment** — `scoring::derive_tq` uses the
//!   `ogm_equivalent_tq(state)` baseline only.

use std::collections::BTreeMap;

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteId, BackendRouteRef,
    Belief, ByteCount, Configuration, ConnectivityRegime, FailureModelClass, Limit, LinkEndpoint,
    MessageFlowAssumptionClass, NodeDensityClass, NodeId, ObjectiveVsDelivered, Observation,
    RatioPermille, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost,
    RouteDegradation, RouteError, RouteEstimate, RouteId, RouteSelectionError, RouteSummary,
    RouteWitness, RoutingTickChange, RuntimeEnvelopeClass, SelectedRoutingParameters, Tick,
    TimeWindow,
};

use crate::{
    public_state::{
        BatmanClassicPlannerSnapshot, BestNextHop, NeighborRanking, OgmReceiveWindow,
        OriginatorObservation, OriginatorObservationTable, ReceivedOgmInfo,
    },
    scoring, BatmanClassicEngine, BATMAN_CLASSIC_CAPABILITIES, BATMAN_CLASSIC_ENGINE_ID,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BatmanClassicRoundState {
    pub learned_advertisements: BTreeMap<NodeId, crate::gossip::LearnedAdvertisement>,
    pub originator_receive_windows: BTreeMap<NodeId, BTreeMap<NodeId, OgmReceiveWindow>>,
    pub received_ogm_info: BTreeMap<NodeId, BTreeMap<NodeId, ReceivedOgmInfo>>,
    pub bidirectional_receive_windows: BTreeMap<NodeId, OgmReceiveWindow>,
    pub originator_observations: OriginatorObservationTable,
    pub neighbor_rankings: BTreeMap<NodeId, NeighborRanking>,
    pub best_next_hops: BTreeMap<NodeId, BestNextHop>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BatmanClassicRoundInput {
    pub local_node_id: NodeId,
    pub stale_after_ticks: u64,
    pub topology: Observation<Configuration>,
    pub now: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BatmanClassicRoundTransition {
    pub next_state: BatmanClassicRoundState,
    pub planner_snapshot: BatmanClassicPlannerSnapshot,
    pub changed: bool,
}

impl<Transport, Effects> BatmanClassicEngine<Transport, Effects> {
    // long-block-exception: one refresh pass derives observations, rankings,
    // and best-next-hop state in a single bounded state update.
    pub(crate) fn refresh_private_state(
        &mut self,
        topology: &Observation<Configuration>,
        now: Tick,
    ) -> RoutingTickChange {
        let round_state = BatmanClassicRoundState {
            learned_advertisements: std::mem::take(&mut self.learned_advertisements),
            originator_receive_windows: std::mem::take(&mut self.originator_receive_windows),
            received_ogm_info: std::mem::take(&mut self.received_ogm_info),
            bidirectional_receive_windows: std::mem::take(&mut self.bidirectional_receive_windows),
            originator_observations: std::mem::take(&mut self.originator_observations),
            neighbor_rankings: std::mem::take(&mut self.neighbor_rankings),
            best_next_hops: std::mem::take(&mut self.best_next_hops),
        };
        let transition = reduce_round_state(
            round_state,
            &BatmanClassicRoundInput {
                local_node_id: self.local_node_id,
                stale_after_ticks: self.decay_window.stale_after_ticks,
                topology: topology.clone(),
                now,
            },
        );

        self.latest_topology = Some(topology.clone());
        self.learned_advertisements = transition.next_state.learned_advertisements;
        self.originator_receive_windows = transition.next_state.originator_receive_windows;
        self.received_ogm_info = transition.next_state.received_ogm_info;
        self.bidirectional_receive_windows = transition.next_state.bidirectional_receive_windows;
        self.originator_observations = transition.next_state.originator_observations;
        self.neighbor_rankings = transition.next_state.neighbor_rankings;
        self.best_next_hops = transition.next_state.best_next_hops;

        if transition.changed {
            RoutingTickChange::PrivateStateUpdated
        } else {
            RoutingTickChange::NoChange
        }
    }

    pub(crate) fn observe_originator_ogm(
        &mut self,
        originator: NodeId,
        via_neighbor: NodeId,
        sequence: u64,
        tq: RatioPermille,
        hop_count: u8,
        observed_at_tick: Tick,
    ) {
        let window_span = self.window_span();
        let window = self
            .originator_receive_windows
            .entry(originator)
            .or_default()
            .entry(via_neighbor)
            .or_default();
        let is_fresher = window.latest_sequence.is_none_or(|seq| sequence > seq);
        window.observe(sequence, observed_at_tick.0, window_span);
        // Update TQ info only when the sequence is strictly newer so that the
        // stored TQ always corresponds to the most recent OGM from this neighbor.
        if is_fresher {
            self.received_ogm_info
                .entry(originator)
                .or_default()
                .insert(
                    via_neighbor,
                    crate::public_state::ReceivedOgmInfo { tq, hop_count },
                );
        }
    }

    pub(crate) fn observe_bidirectional_ogm(
        &mut self,
        neighbor: NodeId,
        sequence: u64,
        observed_at_tick: Tick,
    ) {
        let window_span = self.window_span();
        self.bidirectional_receive_windows
            .entry(neighbor)
            .or_default()
            .observe(sequence, observed_at_tick.0, window_span);
    }

    fn window_span(&self) -> u64 {
        self.decay_window.stale_after_ticks.max(1)
    }

    pub(crate) fn endpoint_for_next_hop(
        &self,
        next_hop: NodeId,
    ) -> Result<LinkEndpoint, RouteError> {
        self.latest_topology
            .as_ref()
            .and_then(|topology| topology.value.nodes.get(&next_hop))
            .and_then(|node| node.profile.endpoints.first().cloned())
            .ok_or(RouteSelectionError::NoCandidate.into())
    }
}

// long-block-exception: originator-observation derivation preserves the
// branch-for-branch receive-window and echo evidence mapping in one pass.
fn derive_originator_observations(
    local_node_id: NodeId,
    stale_after_ticks: u64,
    originator_receive_windows: &BTreeMap<NodeId, BTreeMap<NodeId, OgmReceiveWindow>>,
    received_ogm_info: &BTreeMap<NodeId, BTreeMap<NodeId, ReceivedOgmInfo>>,
    bidirectional_receive_windows: &BTreeMap<NodeId, OgmReceiveWindow>,
    topology: &Observation<Configuration>,
    now: Tick,
) -> OriginatorObservationTable {
    let direct_neighbors = topology
        .value
        .links
        .iter()
        .filter(|((from, _), link)| *from == local_node_id && link_is_usable(link.state.state))
        .map(|((_, neighbor), link)| {
            let (tq, degradation, protocol) = scoring::derive_tq(link);
            (*neighbor, tq, degradation, protocol)
        })
        .collect::<Vec<_>>();
    let window_span = stale_after_ticks.max(1);

    received_ogm_info
        .iter()
        .filter(|(originator, _)| **originator != local_node_id)
        .filter_map(|(originator, ogm_by_neighbor)| {
            let mut per_neighbor = BTreeMap::new();
            for (neighbor, local_tq, local_degradation, local_protocol) in &direct_neighbors {
                let Some(received) = ogm_by_neighbor.get(neighbor) else {
                    continue;
                };
                let is_bidirectional =
                    bidirectional_receive_windows
                        .get(neighbor)
                        .is_some_and(|window| {
                            window.would_be_live_after_prune(now.0, stale_after_ticks, window_span)
                        });
                if !is_bidirectional {
                    continue;
                }
                let receive_quality = originator_receive_windows
                    .get(originator)
                    .and_then(|by_neighbor| by_neighbor.get(neighbor))
                    .filter(|window| window.is_live())
                    .map(|window| window.occupancy_permille(window_span))
                    .unwrap_or(RatioPermille(0));
                if receive_quality.0 == 0 {
                    continue;
                }
                let path_tq = scoring::tq_product(*local_tq, received.tq);
                let tq = scoring::tq_product(path_tq, receive_quality);
                let degradation = max_degradation(
                    *local_degradation,
                    if tq.0 < scoring::TQ_DEGRADED_BELOW {
                        RouteDegradation::Degraded(
                            jacquard_core::DegradationReason::LinkInstability,
                        )
                    } else {
                        RouteDegradation::None
                    },
                );
                per_neighbor.insert(
                    *neighbor,
                    OriginatorObservation {
                        originator: *originator,
                        via_neighbor: *neighbor,
                        tq,
                        receive_quality,
                        hop_count: received.hop_count,
                        observed_at_tick: now,
                        transport_kind: local_protocol.clone(),
                        degradation,
                        is_bidirectional,
                    },
                );
            }
            (!per_neighbor.is_empty()).then_some((*originator, per_neighbor))
        })
        .collect()
}

// long-block-exception: BATMAN Classic round reduction mirrors the protocol's
// receive-window, echo, and ranking transitions in one auditable pass.
pub(crate) fn reduce_round_state(
    mut state: BatmanClassicRoundState,
    input: &BatmanClassicRoundInput,
) -> BatmanClassicRoundTransition {
    let window_span = input.stale_after_ticks.max(1);
    state.learned_advertisements.retain(|_, learned| {
        input.now.0.saturating_sub(learned.observed_at_tick.0) <= input.stale_after_ticks
    });
    prune_receive_windows(
        &mut state.originator_receive_windows,
        input.now,
        input.stale_after_ticks,
        window_span,
    );
    state.bidirectional_receive_windows.retain(|_, window| {
        window.prune(input.now.0, input.stale_after_ticks, window_span);
        window.is_live()
    });
    state.received_ogm_info.retain(|originator, by_neighbor| {
        by_neighbor.retain(|neighbor, _| {
            state
                .originator_receive_windows
                .get(originator)
                .and_then(|by_nbr| by_nbr.get(neighbor))
                .is_some()
        });
        !by_neighbor.is_empty()
    });

    let next_observations = derive_originator_observations(
        input.local_node_id,
        input.stale_after_ticks,
        &state.originator_receive_windows,
        &state.received_ogm_info,
        &state.bidirectional_receive_windows,
        &input.topology,
        input.now,
    );
    let next_rankings = next_observations
        .iter()
        .map(|(originator, observations)| {
            let mut ranked = observations.values().cloned().collect::<Vec<_>>();
            ranked.sort_by(|left, right| {
                right
                    .receive_quality
                    .cmp(&left.receive_quality)
                    .then_with(|| right.tq.cmp(&left.tq))
                    .then_with(|| right.is_bidirectional.cmp(&left.is_bidirectional))
                    .then_with(|| right.observed_at_tick.cmp(&left.observed_at_tick))
                    .then_with(|| left.hop_count.cmp(&right.hop_count))
                    .then_with(|| left.via_neighbor.cmp(&right.via_neighbor))
            });
            (
                *originator,
                NeighborRanking {
                    originator: *originator,
                    ranked_neighbors: ranked,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let next_best = next_rankings
        .iter()
        .filter_map(|(originator, ranking)| {
            ranking.ranked_neighbors.first().map(|best| {
                (
                    *originator,
                    BestNextHop {
                        originator: *originator,
                        next_hop: best.via_neighbor,
                        tq: best.tq,
                        receive_quality: best.receive_quality,
                        hop_count: best.hop_count,
                        updated_at_tick: best.observed_at_tick,
                        transport_kind: best.transport_kind.clone(),
                        degradation: best.degradation,
                        backend_route_id: backend_route_id_for(*originator, best.via_neighbor),
                        topology_epoch: input.topology.value.epoch,
                        is_bidirectional: best.is_bidirectional,
                    },
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let changed = state.originator_observations != next_observations
        || state.neighbor_rankings != next_rankings
        || state.best_next_hops != next_best;
    let planner_snapshot = BatmanClassicPlannerSnapshot {
        local_node_id: input.local_node_id,
        stale_after_ticks: input.stale_after_ticks,
        best_next_hops: next_best.clone(),
    };
    BatmanClassicRoundTransition {
        next_state: BatmanClassicRoundState {
            learned_advertisements: state.learned_advertisements,
            originator_receive_windows: state.originator_receive_windows,
            received_ogm_info: state.received_ogm_info,
            bidirectional_receive_windows: state.bidirectional_receive_windows,
            originator_observations: next_observations,
            neighbor_rankings: next_rankings,
            best_next_hops: next_best,
        },
        planner_snapshot,
        changed,
    }
}

pub(crate) fn candidate_for_snapshot(
    snapshot: &BatmanClassicPlannerSnapshot,
    objective: &jacquard_core::RoutingObjective,
    best: &BestNextHop,
) -> RouteCandidate {
    RouteCandidate {
        route_id: route_id_for(snapshot.local_node_id, best.originator),
        summary: RouteSummary {
            engine: BATMAN_CLASSIC_ENGINE_ID,
            protection: objective.target_protection,
            connectivity: BATMAN_CLASSIC_CAPABILITIES.max_connectivity,
            protocol_mix: vec![best.transport_kind.clone()],
            hop_count_hint: Belief::certain(best.hop_count, best.updated_at_tick),
            valid_for: TimeWindow::new(
                best.updated_at_tick,
                Tick(
                    best.updated_at_tick
                        .0
                        .saturating_add(snapshot.stale_after_ticks),
                ),
            )
            .expect("valid BATMAN classic candidate window"),
        },
        estimate: jacquard_core::Estimate::certain(
            RouteEstimate {
                estimated_protection: objective.target_protection,
                estimated_connectivity: BATMAN_CLASSIC_CAPABILITIES.max_connectivity,
                topology_epoch: best.topology_epoch,
                degradation: best.degradation,
            },
            best.updated_at_tick,
        ),
        backend_ref: BackendRouteRef {
            engine: BATMAN_CLASSIC_ENGINE_ID,
            backend_route_id: best.backend_route_id.clone(),
        },
    }
}

pub(crate) fn admission_for_candidate(
    objective: &jacquard_core::RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: &RouteCandidate,
) -> RouteAdmission {
    let decision = if profile.selected_connectivity.partition
        > BATMAN_CLASSIC_CAPABILITIES.max_connectivity.partition
        || profile.selected_connectivity.repair
            > BATMAN_CLASSIC_CAPABILITIES.max_connectivity.repair
    {
        AdmissionDecision::Rejected(jacquard_core::RouteAdmissionRejection::BackendUnavailable)
    } else {
        AdmissionDecision::Admissible
    };
    RouteAdmission {
        backend_ref: candidate.backend_ref.clone(),
        objective: objective.clone(),
        profile: profile.clone(),
        admission_check: RouteAdmissionCheck {
            decision,
            profile: batman_classic_assumptions(),
            productive_step_bound: Limit::Bounded(1),
            total_step_bound: Limit::Bounded(1),
            route_cost: RouteCost {
                message_count_max: Limit::Bounded(1),
                byte_count_max: Limit::Bounded(ByteCount(256)),
                hop_count: candidate.summary.hop_count_hint.value_or(1),
                repair_attempt_count_max: Limit::Bounded(0),
                hold_bytes_reserved: Limit::Bounded(ByteCount(0)),
                work_step_count_max: Limit::Bounded(1),
            },
        },
        summary: candidate.summary.clone(),
        witness: RouteWitness {
            protection: ObjectiveVsDelivered {
                objective: objective.target_protection,
                delivered: objective.target_protection,
            },
            connectivity: ObjectiveVsDelivered {
                objective: objective.target_connectivity,
                delivered: BATMAN_CLASSIC_CAPABILITIES.max_connectivity,
            },
            admission_profile: batman_classic_assumptions(),
            topology_epoch: candidate.estimate.value.topology_epoch,
            degradation: candidate.estimate.value.degradation,
        },
    }
}

pub(crate) fn route_id_for(local_node_id: NodeId, destination: NodeId) -> RouteId {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&local_node_id.0[..8]);
    bytes[8..].copy_from_slice(&destination.0[..8]);
    RouteId(bytes)
}

pub(crate) fn backend_route_id_for(destination: NodeId, next_hop: NodeId) -> BackendRouteId {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(&destination.0);
    bytes.extend_from_slice(&next_hop.0);
    BackendRouteId(bytes)
}

fn prune_receive_windows(
    windows: &mut BTreeMap<NodeId, BTreeMap<NodeId, OgmReceiveWindow>>,
    now: Tick,
    stale_after_ticks: u64,
    window_span: u64,
) {
    windows.retain(|_, by_neighbor| {
        by_neighbor.retain(|_, window| {
            window.prune(now.0, stale_after_ticks, window_span);
            window.is_live()
        });
        !by_neighbor.is_empty()
    });
}

pub(crate) fn batman_classic_assumptions() -> AdmissionAssumptions {
    AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
        failure_model: FailureModelClass::Benign,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: NodeDensityClass::Moderate,
        connectivity_regime: ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: jacquard_core::ClaimStrength::ConservativeUnderProfile,
    }
}

pub(crate) fn link_is_usable(state: jacquard_core::LinkRuntimeState) -> bool {
    matches!(
        state,
        jacquard_core::LinkRuntimeState::Active | jacquard_core::LinkRuntimeState::Degraded
    )
}

pub(crate) fn max_degradation(left: RouteDegradation, right: RouteDegradation) -> RouteDegradation {
    match (left, right) {
        (RouteDegradation::Degraded(reason), _) | (_, RouteDegradation::Degraded(reason)) => {
            RouteDegradation::Degraded(reason)
        }
        (RouteDegradation::None, RouteDegradation::None) => RouteDegradation::None,
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use jacquard_core::{
        ControllerId, DurationMs, EndpointLocator, Link, LinkProfile, LinkState, Node, NodeProfile,
        NodeState, RatioPermille, RepairCapability, RouteEpoch, RoutingTickChange,
        RoutingTickContext, Tick, TransportKind,
    };
    use jacquard_mem_link_profile::{InMemoryRuntimeEffects, InMemoryTransport};
    use jacquard_traits::RoutingEngine;

    use super::*;
    use crate::public_state::DecayWindow;
    use crate::BatmanClassicEngine;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn empty_node(byte: u8) -> Node {
        Node {
            controller_id: ControllerId([byte; 32]),
            profile: NodeProfile {
                services: Vec::new(),
                endpoints: vec![jacquard_core::LinkEndpoint::new(
                    TransportKind::Custom("reference-hop".into()),
                    EndpointLocator::Opaque(vec![byte]),
                    jacquard_core::ByteCount(64),
                )],
                connection_count_max: 4,
                neighbor_state_count_max: 4,
                simultaneous_transfer_count_max: 1,
                active_route_count_max: 4,
                relay_work_budget_max: jacquard_core::RelayWorkBudget(1),
                maintenance_work_budget_max: jacquard_core::MaintenanceWorkBudget(1),
                hold_item_count_max: jacquard_core::HoldItemCount(0),
                hold_capacity_bytes_max: jacquard_core::ByteCount(0),
            },
            state: NodeState {
                relay_budget: Belief::Absent,
                available_connection_count: Belief::Absent,
                hold_capacity_available_bytes: Belief::Absent,
                information_summary: Belief::Absent,
            },
        }
    }

    fn link(remote: u8) -> Link {
        Link {
            endpoint: jacquard_core::LinkEndpoint::new(
                TransportKind::Custom("reference-hop".into()),
                EndpointLocator::Opaque(vec![remote]),
                jacquard_core::ByteCount(64),
            ),
            profile: LinkProfile {
                latency_floor_ms: DurationMs(5),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: jacquard_core::PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: jacquard_core::LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::Absent,
                symmetry_permille: Belief::Absent,
            },
        }
    }

    fn sample_topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(1),
                nodes: BTreeMap::from([
                    (node(1), empty_node(1)),
                    (node(2), empty_node(2)),
                    (node(3), empty_node(3)),
                ]),
                links: BTreeMap::from([
                    ((node(1), node(2)), link(2)),
                    ((node(2), node(1)), link(1)),
                    ((node(2), node(3)), link(3)),
                    ((node(3), node(2)), link(2)),
                ]),
                environment: jacquard_core::Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: jacquard_core::FactSourceClass::Local,
            evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
            origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    fn engine_round_state(
        engine: &BatmanClassicEngine<InMemoryTransport, InMemoryRuntimeEffects>,
    ) -> BatmanClassicRoundState {
        BatmanClassicRoundState {
            learned_advertisements: engine.learned_advertisements.clone(),
            originator_receive_windows: engine.originator_receive_windows.clone(),
            received_ogm_info: engine.received_ogm_info.clone(),
            bidirectional_receive_windows: engine.bidirectional_receive_windows.clone(),
            originator_observations: engine.originator_observations.clone(),
            neighbor_rankings: engine.neighbor_rankings.clone(),
            best_next_hops: engine.best_next_hops.clone(),
        }
    }

    #[test]
    fn no_candidates_before_ogms_received() {
        // Without OGM receive-window data, the classic engine emits no
        // candidates (no bootstrap shortcut).
        let mut engine = BatmanClassicEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let topology = sample_topology();
        engine
            .engine_tick(&RoutingTickContext::new(topology))
            .expect("first tick");

        assert!(engine.best_next_hops.is_empty());
    }

    #[test]
    fn stale_observations_decay_out_of_private_tables() {
        let mut engine = BatmanClassicEngine::with_decay_window(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
            DecayWindow {
                stale_after_ticks: 1,
                next_refresh_within_ticks: 2,
            },
        );
        let topology = sample_topology();
        // Inject artificial OGM state so there is something to decay.
        engine.observe_originator_ogm(node(3), node(2), 1, RatioPermille(800), 2, Tick(1));
        engine.observe_bidirectional_ogm(node(2), 1, Tick(1));
        engine.refresh_private_state(&topology, Tick(1));

        // Advance time past the stale threshold.
        let stale_topology = Observation {
            observed_at_tick: Tick(10),
            ..sample_topology()
        };
        engine.refresh_private_state(&stale_topology, Tick(10));

        assert!(engine.originator_observations.is_empty());
        assert!(engine.best_next_hops.is_empty());
    }

    #[test]
    fn engine_tick_reports_no_change_after_stable_state() {
        let mut engine = BatmanClassicEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let topology = sample_topology();
        let first = engine
            .engine_tick(&RoutingTickContext::new(topology.clone()))
            .expect("first tick");
        let second = engine
            .engine_tick(&RoutingTickContext::new(topology))
            .expect("second tick");

        assert_eq!(first.change, RoutingTickChange::NoChange);
        assert_eq!(second.change, RoutingTickChange::NoChange);
    }

    #[test]
    fn planner_snapshot_tracks_route_choice_projection() {
        let mut engine = BatmanClassicEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let topology = sample_topology();
        engine.observe_originator_ogm(node(3), node(2), 1, RatioPermille(1000), 2, Tick(1));
        engine.observe_bidirectional_ogm(node(2), 1, Tick(1));

        engine.refresh_private_state(&topology, Tick(1));
        let snapshot = engine.planner_snapshot();

        assert_eq!(snapshot.local_node_id, node(1));
        assert_eq!(
            snapshot.stale_after_ticks,
            engine.decay_window.stale_after_ticks
        );
        assert_eq!(
            snapshot
                .best_next_hops
                .get(&node(3))
                .map(|best| best.next_hop),
            Some(node(2))
        );
    }

    #[test]
    fn round_reducer_matches_wrapper_refresh_projection() {
        let mut engine = BatmanClassicEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let topology = sample_topology();
        engine.observe_originator_ogm(node(3), node(2), 1, RatioPermille(1000), 2, Tick(1));
        engine.observe_bidirectional_ogm(node(2), 1, Tick(1));

        let reduced = reduce_round_state(
            engine_round_state(&engine),
            &BatmanClassicRoundInput {
                local_node_id: node(1),
                stale_after_ticks: engine.decay_window.stale_after_ticks,
                topology: topology.clone(),
                now: Tick(1),
            },
        );

        let wrapper_change = engine.refresh_private_state(&topology, Tick(1));

        assert_eq!(wrapper_change, RoutingTickChange::PrivateStateUpdated);
        assert!(reduced.changed);
        assert_eq!(engine_round_state(&engine), reduced.next_state);
        assert_eq!(engine.planner_snapshot(), reduced.planner_snapshot);
    }
}
