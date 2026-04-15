//! Engine-private state maintenance for `OlsrV2Engine`.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteRef, Belief, ByteCount,
    Configuration, ConnectivityRegime, FailureModelClass, Limit, Link, LinkEndpoint,
    LinkRuntimeState, MessageFlowAssumptionClass, NodeDensityClass, NodeId, ObjectiveVsDelivered,
    Observation, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost, RouteError,
    RouteEstimate, RouteId, RouteSelectionError, RouteSummary, RouteWitness, RoutingTickChange,
    RuntimeEnvelopeClass, SelectedRoutingParameters, Tick, TimeWindow, TransportKind,
};

use crate::{
    gossip::{HelloMessage, TcMessage},
    mpr::select_mprs,
    public_state::{
        HoldWindow, NeighborLinkState, OlsrBestNextHop, TopologyTuple, TwoHopReachability,
    },
    spf::derive_routes,
    OlsrV2Engine, OLSRV2_CAPABILITIES, OLSRV2_ENGINE_ID,
};

impl<Transport, Effects> OlsrV2Engine<Transport, Effects> {
    fn tc_is_newer(&self, tc: &TcMessage) -> bool {
        self.topology_latest_sequences
            .get(&tc.originator)
            .map(|(known_seq, _)| tc.sequence > *known_seq)
            .unwrap_or(true)
    }

    fn replace_originator_tuples(&mut self, tc: &TcMessage, now: Tick) {
        self.topology_latest_sequences
            .insert(tc.originator, (tc.sequence, now));
        self.topology_tuples
            .retain(|(originator, _), _| *originator != tc.originator);
        for advertised_neighbor in tc.advertised_neighbors.iter().copied() {
            self.topology_tuples.insert(
                (tc.originator, advertised_neighbor),
                TopologyTuple {
                    originator: tc.originator,
                    advertised_neighbor,
                    seqno: tc.sequence,
                    observed_at_tick: now,
                },
            );
        }
    }

    pub(crate) fn refresh_private_state(
        &mut self,
        topology: &Observation<Configuration>,
        now: Tick,
    ) -> RoutingTickChange {
        self.prune_neighbor_table(now);
        self.prune_topology_tuples(now);
        self.two_hop_reachability = self.derive_two_hop_reachability();
        self.local_mpr_selection =
            select_mprs(&self.neighbor_table, &self.two_hop_reachability, now);
        let (next_selected, next_best) = derive_routes(
            self.local_node_id,
            &self.neighbor_table,
            &self.topology_tuples,
            topology.value.epoch,
            now,
        );
        let changed = self.selected_routes != next_selected
            || self.best_next_hops != next_best
            || self.latest_topology.as_ref() != Some(topology)
            || self.last_originated_tc_neighbors != self.local_tc_advertised_neighbors(topology);
        self.latest_topology = Some(topology.clone());
        self.selected_routes = next_selected;
        self.best_next_hops = next_best;
        if changed {
            RoutingTickChange::PrivateStateUpdated
        } else {
            RoutingTickChange::NoChange
        }
    }

    pub(crate) fn ingest_hello(
        &mut self,
        from_neighbor: NodeId,
        hello: HelloMessage,
        topology: &Observation<Configuration>,
        now: Tick,
    ) {
        if hello.originator != from_neighbor || hello.originator == self.local_node_id {
            return;
        }
        let direct_symmetric = self.direct_symmetric_neighbors(topology);
        let link_cost = direct_symmetric.get(&from_neighbor).map(|(_, cost)| *cost);
        let transport_kind = topology
            .value
            .links
            .get(&(self.local_node_id, from_neighbor))
            .map(|link| link.endpoint.transport_kind.clone())
            .unwrap_or_else(|| TransportKind::Custom("unknown".into()));
        let state = NeighborLinkState {
            neighbor: from_neighbor,
            latest_sequence: hello.sequence,
            hold_window: HoldWindow {
                last_observed_at_tick: now,
                stale_after_ticks: self.decay_window.stale_after_ticks,
            },
            is_symmetric: link_cost.is_some()
                && hello
                    .symmetric_neighbors
                    .binary_search(&self.local_node_id)
                    .is_ok(),
            is_mpr_selector: hello.mprs.binary_search(&self.local_node_id).is_ok(),
            advertised_symmetric_neighbors: hello.symmetric_neighbors.into_iter().collect(),
            advertised_mprs: hello.mprs.into_iter().collect(),
            link_cost: link_cost.unwrap_or(u32::MAX / 4),
            transport_kind,
        };
        let replace = self
            .neighbor_table
            .get(&from_neighbor)
            .map(|known| hello.sequence >= known.latest_sequence)
            .unwrap_or(true);
        if replace {
            self.neighbor_table.insert(from_neighbor, state);
        }
    }

    pub(crate) fn ingest_tc(
        &mut self,
        from_neighbor: NodeId,
        tc: TcMessage,
        topology: &Observation<Configuration>,
        now: Tick,
    ) {
        if tc.originator == self.local_node_id {
            return;
        }
        if !self.tc_is_newer(&tc) {
            return;
        }
        self.replace_originator_tuples(&tc, now);

        let should_forward = self
            .neighbor_table
            .get(&from_neighbor)
            .map(|neighbor| neighbor.is_mpr_selector && neighbor.is_symmetric)
            .unwrap_or(false);
        let already_forwarded = self
            .last_forwarded_tc_sequences
            .get(&tc.originator)
            .map(|known| *known >= tc.sequence)
            .unwrap_or(false);
        if should_forward && !already_forwarded {
            self.pending_tc_forwards.insert(
                (tc.originator, tc.sequence),
                crate::PendingTcForward {
                    tc,
                    received_from: from_neighbor,
                },
            );
        }

        // Keep local direct-neighbor knowledge aligned with the latest topology
        // snapshot so route derivation never uses a first hop the topology no
        // longer exposes as usable.
        for (neighbor, (_, link_cost)) in self.direct_symmetric_neighbors(topology) {
            if let Some(state) = self.neighbor_table.get_mut(&neighbor) {
                state.link_cost = link_cost;
                state.is_symmetric = true;
            }
        }
    }

    pub(crate) fn candidate_for(
        &self,
        objective: &jacquard_core::RoutingObjective,
        best: &OlsrBestNextHop,
    ) -> RouteCandidate {
        RouteCandidate {
            route_id: self.route_id_for(best.destination),
            summary: RouteSummary {
                engine: OLSRV2_ENGINE_ID,
                protection: objective.target_protection,
                connectivity: OLSRV2_CAPABILITIES.max_connectivity,
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
                .expect("valid OLSRv2 candidate window"),
            },
            estimate: jacquard_core::Estimate::certain(
                RouteEstimate {
                    estimated_protection: objective.target_protection,
                    estimated_connectivity: OLSRV2_CAPABILITIES.max_connectivity,
                    topology_epoch: best.topology_epoch,
                    degradation: best.degradation,
                },
                best.updated_at_tick,
            ),
            backend_ref: BackendRouteRef {
                engine: OLSRV2_ENGINE_ID,
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
            > OLSRV2_CAPABILITIES.max_connectivity.partition
            || profile.selected_connectivity.repair > OLSRV2_CAPABILITIES.max_connectivity.repair
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
                profile: olsrv2_assumptions(),
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
                    delivered: OLSRV2_CAPABILITIES.max_connectivity,
                },
                admission_profile: olsrv2_assumptions(),
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

    pub(crate) fn local_tc_advertised_neighbors(
        &self,
        topology: &Observation<Configuration>,
    ) -> BTreeSet<NodeId> {
        self.direct_symmetric_neighbors(topology)
            .into_keys()
            .collect()
    }

    pub(crate) fn local_mprs(&self) -> BTreeSet<NodeId> {
        self.local_mpr_selection.selected_relays.clone()
    }

    pub(crate) fn direct_neighbor_endpoints(
        &self,
        topology: &Observation<Configuration>,
    ) -> Vec<(NodeId, LinkEndpoint)> {
        topology
            .value
            .links
            .iter()
            .filter(|((from_node_id, _), link)| {
                *from_node_id == self.local_node_id && link_is_usable(link.state.state)
            })
            .map(|((_, neighbor), link)| (*neighbor, link.endpoint.clone()))
            .collect()
    }

    fn direct_symmetric_neighbors(
        &self,
        topology: &Observation<Configuration>,
    ) -> BTreeMap<NodeId, (LinkEndpoint, u32)> {
        topology
            .value
            .links
            .iter()
            .filter_map(|((from_node_id, neighbor), link)| {
                (*from_node_id == self.local_node_id
                    && link_is_usable(link.state.state)
                    && topology
                        .value
                        .links
                        .get(&(*neighbor, self.local_node_id))
                        .is_some_and(|reverse| link_is_usable(reverse.state.state)))
                .then_some((*neighbor, (link.endpoint.clone(), link_metric(link))))
            })
            .collect()
    }

    fn derive_two_hop_reachability(&self) -> BTreeMap<NodeId, TwoHopReachability> {
        let direct_neighbors: BTreeSet<NodeId> = self
            .neighbor_table
            .iter()
            .filter_map(|(neighbor, state)| state.is_symmetric.then_some(*neighbor))
            .collect();
        let mut reachability = BTreeMap::new();
        for (neighbor, state) in &self.neighbor_table {
            if !state.is_symmetric {
                continue;
            }
            for two_hop in &state.advertised_symmetric_neighbors {
                if *two_hop == self.local_node_id || direct_neighbors.contains(two_hop) {
                    continue;
                }
                reachability
                    .entry(*two_hop)
                    .or_insert_with(|| TwoHopReachability {
                        two_hop: *two_hop,
                        via_neighbors: BTreeSet::new(),
                    })
                    .via_neighbors
                    .insert(*neighbor);
            }
        }
        reachability
    }

    fn prune_neighbor_table(&mut self, now: Tick) {
        self.neighbor_table
            .retain(|_, state| state.hold_window.is_live(now));
    }

    fn prune_topology_tuples(&mut self, now: Tick) {
        self.topology_latest_sequences
            .retain(|originator, (_, observed_at_tick)| {
                let live =
                    now.0.saturating_sub(observed_at_tick.0) <= self.decay_window.stale_after_ticks;
                if !live {
                    self.topology_tuples
                        .retain(|(tuple_originator, _), _| tuple_originator != originator);
                }
                live
            });
    }
}

pub(crate) fn olsrv2_assumptions() -> AdmissionAssumptions {
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

pub(crate) fn link_metric(link: &Link) -> u32 {
    let confidence = u32::from(
        link.state
            .delivery_confidence_permille
            .value_or(jacquard_core::RatioPermille(900))
            .0,
    );
    let loss = u32::from(link.state.loss_permille.0);
    let state_penalty = match link.state.state {
        LinkRuntimeState::Active => 0,
        LinkRuntimeState::Degraded => 4,
        LinkRuntimeState::Suspended | LinkRuntimeState::Faulted => 1000,
    };
    1 + ((1000_u32.saturating_sub(confidence)).saturating_add(loss) / 200) + state_penalty
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_adapter::opaque_endpoint;
    use jacquard_core::{
        ByteCount, Configuration, ControllerId, Environment, FactSourceClass, LinkEndpoint,
        Observation, OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutingEvidenceClass,
        Tick, TransportKind,
    };
    use jacquard_mem_link_profile::{
        InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
    };
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};

    use super::*;
    use crate::gossip::{HelloMessage, TcMessage};

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
    }

    fn fixture_node(byte: u8) -> jacquard_core::Node {
        NodePreset::route_capable(
            NodePresetOptions::new(
                NodeIdentity::new(node(byte), ControllerId([byte; 32])),
                endpoint(byte),
                Tick(1),
            ),
            &OLSRV2_ENGINE_ID,
        )
        .build()
    }

    fn fixture_link(byte: u8) -> jacquard_core::Link {
        LinkPreset::active(LinkPresetOptions::new(endpoint(byte), Tick(1))).build()
    }

    fn sample_topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(2),
                nodes: BTreeMap::from([
                    (node(1), fixture_node(1)),
                    (node(2), fixture_node(2)),
                    (node(3), fixture_node(3)),
                ]),
                links: BTreeMap::from([
                    ((node(1), node(2)), fixture_link(2)),
                    ((node(2), node(1)), fixture_link(1)),
                    ((node(2), node(3)), fixture_link(3)),
                    ((node(3), node(2)), fixture_link(2)),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 2,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    #[test]
    fn hello_ingestion_promotes_symmetric_neighbor() {
        let topology = sample_topology();
        let mut engine = OlsrV2Engine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );

        engine.ingest_hello(
            node(2),
            HelloMessage {
                originator: node(2),
                sequence: 1,
                symmetric_neighbors: vec![node(1), node(3)],
                mprs: vec![node(1)],
            },
            &topology,
            Tick(2),
        );

        let neighbor = engine.neighbor_table.get(&node(2)).expect("neighbor state");
        assert!(neighbor.is_symmetric);
        assert!(neighbor.is_mpr_selector);
    }

    #[test]
    fn tc_ingestion_replaces_older_topology_for_originator() {
        let topology = sample_topology();
        let mut engine = OlsrV2Engine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        engine.ingest_tc(
            node(2),
            TcMessage {
                originator: node(2),
                sequence: 1,
                advertised_neighbors: vec![node(3)],
            },
            &topology,
            Tick(2),
        );
        engine.ingest_tc(
            node(2),
            TcMessage {
                originator: node(2),
                sequence: 2,
                advertised_neighbors: vec![node(1)],
            },
            &topology,
            Tick(3),
        );

        assert_eq!(engine.topology_tuples.len(), 1);
        assert!(engine.topology_tuples.contains_key(&(node(2), node(1))));
    }

    #[test]
    fn refresh_prunes_stale_hello_state() {
        let topology = sample_topology();
        let mut engine = OlsrV2Engine::with_decay_window(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
            crate::DecayWindow::new(2, 1),
        );
        engine.ingest_hello(
            node(2),
            HelloMessage {
                originator: node(2),
                sequence: 1,
                symmetric_neighbors: vec![node(1), node(3)],
                mprs: vec![],
            },
            &topology,
            Tick(1),
        );

        let change = engine.refresh_private_state(&topology, Tick(5));

        assert_eq!(
            change,
            jacquard_core::RoutingTickChange::PrivateStateUpdated
        );
        assert!(!engine.neighbor_table.contains_key(&node(2)));
    }

    #[test]
    fn refresh_updates_best_next_hop_from_topology_tuples() {
        let topology = sample_topology();
        let mut engine = OlsrV2Engine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        engine.ingest_hello(
            node(2),
            HelloMessage {
                originator: node(2),
                sequence: 1,
                symmetric_neighbors: vec![node(1), node(3)],
                mprs: vec![node(1)],
            },
            &topology,
            Tick(2),
        );
        engine.ingest_tc(
            node(2),
            TcMessage {
                originator: node(2),
                sequence: 1,
                advertised_neighbors: vec![node(3)],
            },
            &topology,
            Tick(2),
        );

        engine.refresh_private_state(&topology, Tick(2));

        let best = engine.best_next_hops.get(&node(3)).expect("best next hop");
        assert_eq!(best.next_hop, node(2));
        assert_eq!(best.hop_count, 2);
    }
}
