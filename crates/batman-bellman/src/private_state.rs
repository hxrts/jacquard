//! Engine-private state maintenance for `BatmanBellmanEngine`.
//!
//! Refreshes per-originator observations from the latest topology, ranks
//! neighbors by transmit quality and hop count, and derives the best next-hop
//! table consumed by the planner and runtime modules.
//!
//! The main entry point is `refresh_private_state`, which runs on each engine
//! tick and performs three passes:
//! 1. `derive_originator_observations` — for each remote node visible in the
//!    topology, combine the local direct-link score with classic
//!    B.A.T.M.A.N.-style receive-window occupancy for OGMs relayed by each
//!    direct neighbor.
//! 2. Bidirectional-link gating — only neighbors that currently satisfy the
//!    bidirectional-link check may contribute routing observations.
//! 3. Ranking and best-next-hop derivation — sort each originator's neighbor
//!    list by descending receive-window quality, then descending combined TQ,
//!    then ascending hop count, then neighbor id, and extract the first entry
//!    as the `BestNextHop`.
//!
//! Also exposes helper methods used by the planner and runtime:
//! `candidate_for`, `admission_for`, `route_id_for`, `backend_route_id_for`,
//! and `is_stale`.

use std::collections::BTreeMap;

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteId, BackendRouteRef,
    Belief, ByteCount, Configuration, ConnectivityRegime, FailureModelClass, Limit, LinkEndpoint,
    LinkRuntimeState, MessageFlowAssumptionClass, NodeDensityClass, NodeId, ObjectiveVsDelivered,
    Observation, RatioPermille, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost,
    RouteDegradation, RouteError, RouteEstimate, RouteId, RouteSelectionError, RouteSummary,
    RouteWitness, RoutingTickChange, RuntimeEnvelopeClass, SelectedRoutingParameters, Tick,
    TimeWindow,
};

use crate::{
    gossip::merge_advertisements,
    public_state::{
        BestNextHop, NeighborRanking, OgmReceiveWindow, OriginatorObservation,
        OriginatorObservationTable,
    },
    scoring, BatmanBellmanEngine, BATMAN_BELLMAN_CAPABILITIES, BATMAN_BELLMAN_ENGINE_ID,
};

impl<Transport, Effects> BatmanBellmanEngine<Transport, Effects> {
    // long-block-exception: one refresh pass derives observations, rankings,
    // and best-next-hop state in a single bounded state update.
    pub(crate) fn refresh_private_state(
        &mut self,
        topology: &Observation<Configuration>,
        now: Tick,
    ) -> RoutingTickChange {
        let stale_after_ticks = self.decay_window.stale_after_ticks;
        let window_span = self.window_span();
        self.learned_advertisements.retain(|_, learned| {
            now.0.saturating_sub(learned.observed_at_tick.0) <= stale_after_ticks
        });
        prune_receive_windows(
            &mut self.originator_receive_windows,
            now,
            stale_after_ticks,
            window_span,
        );
        self.bidirectional_receive_windows.retain(|_, window| {
            window.prune(now, stale_after_ticks, window_span);
            window.is_live()
        });
        let merged_topology = merge_advertisements(
            topology,
            &self.learned_advertisements,
            now,
            stale_after_ticks,
        );
        let next_observations = self.derive_originator_observations(&merged_topology, now);
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
                            backend_route_id: self
                                .backend_route_id_for(*originator, best.via_neighbor),
                            topology_epoch: merged_topology.value.epoch,
                            is_bidirectional: best.is_bidirectional,
                        },
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();

        let changed = self.originator_observations != next_observations
            || self.neighbor_rankings != next_rankings
            || self.best_next_hops != next_best;
        self.latest_topology = Some(merged_topology);
        self.originator_observations = next_observations;
        self.neighbor_rankings = next_rankings;
        self.best_next_hops = next_best;
        if changed {
            RoutingTickChange::PrivateStateUpdated
        } else {
            RoutingTickChange::NoChange
        }
    }

    // long-block-exception: one pass derives classic BATMAN originator
    // observations by combining direct-neighbor transport quality, downstream
    // path quality, and receive-window occupancy in a single auditable loop.
    fn derive_originator_observations(
        &self,
        topology: &Observation<Configuration>,
        now: Tick,
    ) -> OriginatorObservationTable {
        let direct_neighbors = topology
            .value
            .links
            .iter()
            .filter(|((from, _), link)| {
                *from == self.local_node_id && link_is_usable(link.state.state)
            })
            .map(|((_, neighbor), link)| {
                let (tq, degradation, protocol) = scoring::derive_tq(link);
                (*neighbor, tq, degradation, protocol)
            })
            .collect::<Vec<_>>();
        let any_window_state = !self.originator_receive_windows.is_empty();

        topology
            .value
            .nodes
            .keys()
            .copied()
            .filter(|originator| *originator != self.local_node_id)
            .filter_map(|originator| {
                let mut per_neighbor = BTreeMap::new();
                for (neighbor, local_tq, local_degradation, local_protocol) in &direct_neighbors {
                    let Some((path_tq, remote_hops)) =
                        self.best_path_from(*neighbor, originator, topology)
                    else {
                        continue;
                    };
                    let is_bidirectional =
                        self.bidirectional_neighbor_valid(topology, *neighbor, now);
                    if !is_bidirectional {
                        continue;
                    }
                    let receive_quality = self
                        .window_quality_for(originator, *neighbor)
                        .or_else(|| (!any_window_state).then_some(path_tq))
                        .unwrap_or(RatioPermille(0));
                    if receive_quality.0 == 0 {
                        continue;
                    }
                    let tq = scoring::tq_product(
                        scoring::tq_product(*local_tq, path_tq),
                        receive_quality,
                    );
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
                            originator,
                            via_neighbor: *neighbor,
                            tq,
                            receive_quality,
                            hop_count: remote_hops.saturating_add(1),
                            observed_at_tick: now,
                            transport_kind: local_protocol.clone(),
                            degradation,
                            is_bidirectional,
                        },
                    );
                }
                (!per_neighbor.is_empty()).then_some((originator, per_neighbor))
            })
            .collect()
    }

    pub(crate) fn observe_originator_ogm(
        &mut self,
        originator: NodeId,
        via_neighbor: NodeId,
        sequence: u64,
        observed_at_tick: Tick,
    ) {
        let window_span = self.window_span();
        self.originator_receive_windows
            .entry(originator)
            .or_default()
            .entry(via_neighbor)
            .or_default()
            .observe(sequence, observed_at_tick, window_span);
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
            .observe(sequence, observed_at_tick, window_span);
    }

    pub(crate) fn bidirectional_neighbor_valid(
        &self,
        topology: &Observation<Configuration>,
        neighbor: NodeId,
        now: Tick,
    ) -> bool {
        if self
            .bidirectional_receive_windows
            .get(&neighbor)
            .is_some_and(|window| self.window_is_live(window, now))
        {
            return true;
        }
        topology
            .value
            .links
            .get(&(neighbor, self.local_node_id))
            .is_some_and(|link| link_is_usable(link.state.state))
    }

    fn window_quality_for(
        &self,
        originator: NodeId,
        via_neighbor: NodeId,
    ) -> Option<RatioPermille> {
        self.originator_receive_windows
            .get(&originator)
            .and_then(|by_neighbor| by_neighbor.get(&via_neighbor))
            .filter(|window| window.is_live())
            .map(|window| window.occupancy_permille(self.window_span()))
    }

    fn window_is_live(&self, window: &OgmReceiveWindow, now: Tick) -> bool {
        let mut clone = window.clone();
        clone.prune(now, self.decay_window.stale_after_ticks, self.window_span());
        clone.is_live()
    }

    fn window_span(&self) -> u64 {
        self.decay_window.stale_after_ticks.max(1)
    }

    fn best_path_from(
        &self,
        start: NodeId,
        destination: NodeId,
        topology: &Observation<Configuration>,
    ) -> Option<(jacquard_core::RatioPermille, u8)> {
        if start == destination {
            return Some((jacquard_core::RatioPermille(1000), 0));
        }

        let node_count = topology.value.nodes.len();
        let mut best = BTreeMap::from([(start, (jacquard_core::RatioPermille(1000), 0_u8))]);
        for _ in 0..node_count.saturating_sub(1) {
            let mut changed = false;
            for ((from, to), link) in &topology.value.links {
                if *to == self.local_node_id || !link_is_usable(link.state.state) {
                    continue;
                }
                let Some((score_so_far, hops_so_far)) = best.get(from).copied() else {
                    continue;
                };
                let (edge_tq, _, _) = scoring::derive_tq(link);
                let candidate_tq = scoring::tq_product(score_so_far, edge_tq);
                let candidate_hops = hops_so_far.saturating_add(1);
                let current = best.get(to).copied();
                if better_path(current, candidate_tq, candidate_hops) {
                    best.insert(*to, (candidate_tq, candidate_hops));
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        best.get(&destination).copied()
    }

    pub(crate) fn candidate_for(
        &self,
        objective: &jacquard_core::RoutingObjective,
        best: &BestNextHop,
    ) -> RouteCandidate {
        RouteCandidate {
            route_id: self.route_id_for(best.originator),
            summary: RouteSummary {
                engine: BATMAN_BELLMAN_ENGINE_ID,
                protection: objective.target_protection,
                connectivity: BATMAN_BELLMAN_CAPABILITIES.max_connectivity,
                protocol_mix: vec![best.transport_kind.clone()],
                hop_count_hint: Belief::certain(best.hop_count, best.updated_at_tick),
                valid_for: TimeWindow::new(
                    best.updated_at_tick,
                    Tick(
                        best.updated_at_tick
                            .0
                            .saturating_add(self.decay_window.stale_after_ticks),
                    ),
                )
                .expect("valid BATMAN candidate window"),
            },
            estimate: jacquard_core::Estimate::certain(
                RouteEstimate {
                    estimated_protection: objective.target_protection,
                    estimated_connectivity: BATMAN_BELLMAN_CAPABILITIES.max_connectivity,
                    topology_epoch: best.topology_epoch,
                    degradation: best.degradation,
                },
                best.updated_at_tick,
            ),
            backend_ref: BackendRouteRef {
                engine: BATMAN_BELLMAN_ENGINE_ID,
                backend_route_id: best.backend_route_id.clone(),
            },
        }
    }

    pub(crate) fn admission_for(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
    ) -> RouteAdmission {
        let decision = if profile.selected_connectivity.partition
            > BATMAN_BELLMAN_CAPABILITIES.max_connectivity.partition
            || profile.selected_connectivity.repair
                > BATMAN_BELLMAN_CAPABILITIES.max_connectivity.repair
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
                profile: batman_assumptions(),
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
                    delivered: BATMAN_BELLMAN_CAPABILITIES.max_connectivity,
                },
                admission_profile: batman_assumptions(),
                topology_epoch: candidate.estimate.value.topology_epoch,
                degradation: candidate.estimate.value.degradation,
            },
        }
    }

    pub(crate) fn route_id_for(&self, destination: NodeId) -> RouteId {
        let mut bytes = [0_u8; 16];
        bytes[..8].copy_from_slice(&self.local_node_id.0[..8]);
        bytes[8..].copy_from_slice(&destination.0[..8]);
        RouteId(bytes)
    }

    pub(crate) fn backend_route_id_for(
        &self,
        destination: NodeId,
        next_hop: NodeId,
    ) -> BackendRouteId {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(&destination.0);
        bytes.extend_from_slice(&next_hop.0);
        BackendRouteId(bytes)
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

fn prune_receive_windows(
    windows: &mut BTreeMap<NodeId, BTreeMap<NodeId, OgmReceiveWindow>>,
    now: Tick,
    stale_after_ticks: u64,
    window_span: u64,
) {
    windows.retain(|_, by_neighbor| {
        by_neighbor.retain(|_, window| {
            window.prune(now, stale_after_ticks, window_span);
            window.is_live()
        });
        !by_neighbor.is_empty()
    });
}

pub(crate) fn batman_assumptions() -> AdmissionAssumptions {
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

pub(crate) fn link_is_usable(state: LinkRuntimeState) -> bool {
    matches!(state, LinkRuntimeState::Active | LinkRuntimeState::Degraded)
}

pub(crate) fn better_path(
    current: Option<(jacquard_core::RatioPermille, u8)>,
    candidate_tq: jacquard_core::RatioPermille,
    candidate_hops: u8,
) -> bool {
    match current {
        None => true,
        Some((current_tq, current_hops)) => {
            candidate_tq > current_tq
                || (candidate_tq == current_tq && candidate_hops < current_hops)
        }
    }
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
        ControllerId, DurationMs, EndpointLocator, Environment, Link, LinkProfile, LinkState, Node,
        NodeProfile, NodeState, RatioPermille, RepairCapability, RouteEpoch, RoutingTickContext,
        RoutingTickHint, Tick, TransportKind,
    };
    use jacquard_mem_link_profile::{InMemoryRuntimeEffects, InMemoryTransport};
    use jacquard_traits::RoutingEngine;

    use super::*;
    use crate::public_state::DecayWindow;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn empty_node(byte: u8) -> Node {
        Node {
            controller_id: ControllerId([byte; 32]),
            profile: NodeProfile {
                services: Vec::new(),
                endpoints: vec![LinkEndpoint::new(
                    TransportKind::Custom("reference-hop".into()),
                    EndpointLocator::Opaque(vec![byte]),
                    ByteCount(64),
                )],
                connection_count_max: 4,
                neighbor_state_count_max: 4,
                simultaneous_transfer_count_max: 1,
                active_route_count_max: 4,
                relay_work_budget_max: jacquard_core::RelayWorkBudget(1),
                maintenance_work_budget_max: jacquard_core::MaintenanceWorkBudget(1),
                hold_item_count_max: jacquard_core::HoldItemCount(0),
                hold_capacity_bytes_max: ByteCount(0),
            },
            state: NodeState {
                relay_budget: Belief::Absent,
                available_connection_count: Belief::Absent,
                hold_capacity_available_bytes: Belief::Absent,
                information_summary: Belief::Absent,
            },
        }
    }

    fn link(remote: u8, delivery: u16, symmetry: u16, loss: u16) -> Link {
        Link {
            endpoint: LinkEndpoint::new(
                TransportKind::Custom("reference-hop".into()),
                EndpointLocator::Opaque(vec![remote]),
                ByteCount(64),
            ),
            profile: LinkProfile {
                latency_floor_ms: DurationMs(5),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: jacquard_core::PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::certain(128_000, Tick(1)),
                stability_horizon_ms: Belief::certain(DurationMs(4_000), Tick(1)),
                loss_permille: RatioPermille(loss),
                delivery_confidence_permille: Belief::certain(RatioPermille(delivery), Tick(1)),
                symmetry_permille: Belief::certain(RatioPermille(symmetry), Tick(1)),
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
                    (node(4), empty_node(4)),
                ]),
                links: BTreeMap::from([
                    ((node(1), node(2)), link(2, 960, 950, 5)),
                    ((node(2), node(1)), link(1, 960, 950, 5)),
                    ((node(2), node(4)), link(4, 940, 930, 10)),
                    ((node(4), node(2)), link(2, 940, 930, 10)),
                    ((node(1), node(3)), link(3, 910, 900, 20)),
                    ((node(3), node(1)), link(1, 910, 900, 20)),
                    ((node(3), node(4)), link(4, 800, 790, 80)),
                    ((node(4), node(3)), link(3, 800, 790, 80)),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 2,
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

    #[test]
    fn ranking_table_prefers_higher_tq_neighbor() {
        let mut engine = BatmanBellmanEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let topology = sample_topology();

        let outcome = engine
            .engine_tick(&RoutingTickContext::new(topology))
            .expect("first tick");

        assert_eq!(outcome.change, RoutingTickChange::PrivateStateUpdated);
        assert_eq!(engine.best_next_hops[&node(4)].next_hop, node(2));
    }

    #[test]
    fn stale_observations_decay_out_of_private_tables() {
        let mut engine = BatmanBellmanEngine::with_decay_window(
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
        engine
            .engine_tick(&RoutingTickContext::new(topology))
            .expect("populate table");

        let stale_topology = Observation {
            observed_at_tick: Tick(4),
            ..sample_topology()
        };
        engine.refresh_private_state(&stale_topology, Tick(4));

        assert!(engine.originator_observations.values().all(|entries| {
            entries
                .values()
                .all(|observation| observation.observed_at_tick == Tick(4))
        }));
    }

    #[test]
    fn engine_tick_reports_immediate_then_bounded_hint() {
        let mut engine = BatmanBellmanEngine::new(
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

        assert_eq!(first.next_tick_hint, RoutingTickHint::Immediate);
        assert_eq!(second.change, RoutingTickChange::NoChange);
        assert_eq!(second.next_tick_hint, RoutingTickHint::WithinTicks(Tick(4)));
    }
}
