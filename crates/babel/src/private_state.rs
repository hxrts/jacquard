//! Engine-private state maintenance for `BabelEngine`.
//!
//! ## Feasibility distance table (RFC 8966 Section 3.5.1)
//!
//! Every node maintains a per-destination feasibility distance `FD[D]`, stored
//! as a `(seqno, metric)` pair. A route entry `(seqno, metric)` for destination
//! D is **feasible** if and only if:
//!
//! ```text
//! seqno_is_newer(entry.seqno, FD[D].seqno)
//!   OR (entry.seqno == FD[D].seqno AND entry.metric < FD[D].metric)
//! ```
//!
//! where `seqno_is_newer` uses modular arithmetic as defined in RFC 8966
//! Section 3.5.1. When FD is absent for a destination (never selected, or all
//! routes expired), any finite-metric route is feasible.
//!
//! **Route admission vs route selection**: updates are always admitted to the
//! route table (no FC check on ingestion). Feasibility is only enforced when
//! choosing the best route per destination during each refresh pass. This
//! matches the RFC: the FC gates selection, not admission.
//!
//! **Feasible selection**: among unexpired finite-metric routes for D, the
//! engine prefers the feasible candidate with the lowest metric. FD[D] is
//! updated to `(seqno, metric)` of the selected route only when the selection
//! is feasible — infeasible fallback selections do not update FD, leaving it
//! in place for the next seqno epoch to resolve.
//!
//! **Infeasible fallback**: when no feasible route exists (all candidates have
//! metric >= FD and same seqno), the engine selects the best infeasible route
//! as a last-resort path to preserve connectivity. FD is not updated during
//! infeasible fallback. The periodic seqno increment (every
//! `SEQNO_REFRESH_INTERVAL_TICKS`) propagates a fresh seqno downstream; when
//! that update arrives, it satisfies the FC (newer seqno) and allows FD to be
//! updated, ending the fallback period. This replaces the explicit SEQREQ
//! mechanism from RFC 8966: rather than requesting an immediate seqno bump, the
//! engine waits for the originator's next periodic increment. The resulting
//! infeasible-fallback window is bounded by `SEQNO_REFRESH_INTERVAL_TICKS`.
//!
//! **FD expiry**: when all routes to D expire from the route table, FD[D] is
//! removed. The next route learned for D will be treated as if FD = ∞.
//!
//! ## Bidirectionality via link cost
//!
//! Unlike batman-classic which uses an echo-window gate to confirm bidirectional
//! links, Babel encodes bidirectionality in the link cost itself via the ETX
//! formula: `256 * 1_000_000 / (fwd_delivery * rev_delivery)`. If the reverse
//! link is absent, `rev_delivery = 0` and `link_cost = BABEL_INFINITY`, so
//! `compound_metric = BABEL_INFINITY` and the route is never stored. No
//! separate echo-window check is needed.

use std::collections::BTreeMap;

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteId, BackendRouteRef,
    Belief, ByteCount, Configuration, ConnectivityRegime, FailureModelClass, Limit, LinkEndpoint,
    MessageFlowAssumptionClass, NodeDensityClass, NodeId, ObjectiveVsDelivered, Observation,
    RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost, RouteError, RouteEstimate,
    RouteId, RouteSelectionError, RouteSummary, RouteWitness, RoutingTickChange,
    RuntimeEnvelopeClass, SelectedRoutingParameters, Tick, TimeWindow,
};

use crate::{
    gossip::{BabelUpdate, BABEL_INFINITY},
    public_state::{BabelBestNextHop, FeasibilityEntry, RouteEntry, SelectedBabelRoute},
    scoring::{self, link_cost, metric_degradation, metric_to_ratio, seqno_is_newer},
    BabelEngine, BABEL_CAPABILITIES, BABEL_ENGINE_ID,
};

impl<Transport, Effects> BabelEngine<Transport, Effects> {
    // long-block-exception: one refresh pass prunes stale entries, enforces
    // the RFC 8966 feasibility condition, rebuilds selected_routes, and derives
    // best_next_hops in a single bounded update.
    pub(crate) fn refresh_private_state(
        &mut self,
        topology: &Observation<Configuration>,
        now: Tick,
    ) -> RoutingTickChange {
        let stale_after_ticks = self.decay_window.stale_after_ticks;

        // Prune stale route entries from the route table.
        self.route_table.retain(|_, by_neighbor| {
            by_neighbor.retain(|_, entry| {
                now.0.saturating_sub(entry.observed_at_tick.0) <= stale_after_ticks
            });
            !by_neighbor.is_empty()
        });

        // Clear FD for destinations whose route table entries all expired.
        // Absent FD = ∞: any finite-metric route for that destination is
        // treated as feasible when new updates arrive.
        {
            let route_table = &self.route_table;
            self.feasibility_distances
                .retain(|dest, _| route_table.contains_key(dest));
        }

        // Rebuild selected_routes using the RFC 8966 feasibility condition.
        // For each destination: prefer the feasible candidate with the lowest
        // metric. If no feasible route exists, fall back to the best infeasible
        // route (without updating FD) to preserve connectivity until the next
        // seqno epoch from the originator restores feasibility.
        let mut next_feasibility = self.feasibility_distances.clone();
        let mut next_selected: BTreeMap<NodeId, SelectedBabelRoute> = BTreeMap::new();

        for (dest, by_neighbor) in &self.route_table {
            if *dest == self.local_node_id {
                continue;
            }
            let candidates: Vec<(NodeId, RouteEntry)> = by_neighbor
                .iter()
                .filter(|(_, entry)| entry.metric < BABEL_INFINITY)
                .map(|(n, e)| (*n, *e))
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let fd = self.feasibility_distances.get(dest);
            let feasible_best = candidates
                .iter()
                .filter(|(_, entry)| route_is_feasible(entry, fd))
                .min_by_key(|(_, entry)| entry.metric);

            let (via_neighbor, entry, is_feasible) = if let Some((n, e)) = feasible_best {
                (*n, *e, true)
            } else {
                // Infeasible fallback: use best-metric route without updating FD.
                let (n, e) = candidates
                    .iter()
                    .min_by_key(|(_, entry)| entry.metric)
                    .expect("candidates non-empty");
                (*n, *e, false)
            };

            // Update FD only on feasible selections. Infeasible fallback
            // selections leave FD at its prior value so that the periodic
            // seqno increment can restore feasibility on the next epoch.
            if is_feasible {
                next_feasibility.insert(
                    *dest,
                    FeasibilityEntry {
                        seqno: entry.seqno,
                        metric: entry.metric,
                    },
                );
            }

            let transport_kind = topology
                .value
                .links
                .get(&(self.local_node_id, via_neighbor))
                .map(|link| link.endpoint.transport_kind.clone())
                .unwrap_or_else(|| jacquard_core::TransportKind::Custom("unknown".into()));

            next_selected.insert(
                *dest,
                SelectedBabelRoute {
                    destination: *dest,
                    via_neighbor,
                    metric: entry.metric,
                    seqno: entry.seqno,
                    router_id: entry.router_id,
                    tq: metric_to_ratio(entry.metric),
                    degradation: metric_degradation(entry.metric),
                    transport_kind,
                    observed_at_tick: entry.observed_at_tick,
                },
            );
        }

        // Rebuild best_next_hops from selected_routes.
        let next_best: BTreeMap<NodeId, BabelBestNextHop> = next_selected
            .iter()
            .map(|(dest, selected)| {
                (
                    *dest,
                    BabelBestNextHop {
                        destination: *dest,
                        next_hop: selected.via_neighbor,
                        metric: selected.metric,
                        tq: selected.tq,
                        degradation: selected.degradation,
                        transport_kind: selected.transport_kind.clone(),
                        updated_at_tick: selected.observed_at_tick,
                        topology_epoch: topology.value.epoch,
                        backend_route_id: self.backend_route_id_for(*dest, selected.via_neighbor),
                    },
                )
            })
            .collect();

        let changed = self.selected_routes != next_selected || self.best_next_hops != next_best;

        self.latest_topology = Some(topology.clone());
        self.selected_routes = next_selected;
        self.best_next_hops = next_best;
        self.feasibility_distances = next_feasibility;

        if changed {
            RoutingTickChange::PrivateStateUpdated
        } else {
            RoutingTickChange::NoChange
        }
    }

    /// Ingest a received Babel update from `from_neighbor`.
    ///
    /// Computes bidirectional link cost and compound metric. Ignores the update
    /// if the route is unusable (metric >= BABEL_INFINITY) or if the stored
    /// entry has a fresher seqno.
    pub(crate) fn ingest_update(
        &mut self,
        from_neighbor: NodeId,
        update: &BabelUpdate,
        topology: &Observation<Configuration>,
        now: Tick,
    ) {
        if update.destination == self.local_node_id {
            return;
        }
        let local = self.local_node_id;
        let cost = link_cost(
            topology.value.links.get(&(local, from_neighbor)),
            topology.value.links.get(&(from_neighbor, local)),
        );
        let metric = scoring::compound_metric(cost, update.metric);
        if metric == BABEL_INFINITY {
            return;
        }
        // Only update if seqno is fresher or equal (accept same seqno with
        // potentially better metric, matching the RFC's feasibility update rule
        // as simplified here).
        let existing = self
            .route_table
            .get(&update.destination)
            .and_then(|by_nbr| by_nbr.get(&from_neighbor));
        if let Some(existing) = existing {
            if update.seqno < existing.seqno {
                return;
            }
        }
        self.route_table
            .entry(update.destination)
            .or_default()
            .insert(
                from_neighbor,
                RouteEntry {
                    router_id: update.router_id,
                    seqno: update.seqno,
                    metric,
                    observed_at_tick: now,
                },
            );
    }

    pub(crate) fn candidate_for(
        &self,
        objective: &jacquard_core::RoutingObjective,
        best: &BabelBestNextHop,
    ) -> RouteCandidate {
        // hop_count_hint: estimate hops from metric. Each hop adds ~256 (one
        // perfect ETX hop), so metric/256 approximates hop count. Clamp to 1..=255.
        let hop_estimate = u8::try_from(
            (u32::from(best.metric) / 256)
                .max(1)
                .min(u32::from(u8::MAX)),
        )
        .unwrap_or(1);
        RouteCandidate {
            route_id: self.route_id_for(best.destination),
            summary: RouteSummary {
                engine: BABEL_ENGINE_ID,
                protection: objective.target_protection,
                connectivity: BABEL_CAPABILITIES.max_connectivity,
                protocol_mix: vec![best.transport_kind.clone()],
                hop_count_hint: Belief::certain(hop_estimate, best.updated_at_tick),
                valid_for: TimeWindow::new(
                    best.updated_at_tick,
                    Tick(
                        best.updated_at_tick
                            .0
                            .saturating_add(self.decay_window.stale_after_ticks),
                    ),
                )
                .expect("valid Babel candidate window"),
            },
            estimate: jacquard_core::Estimate::certain(
                RouteEstimate {
                    estimated_protection: objective.target_protection,
                    estimated_connectivity: BABEL_CAPABILITIES.max_connectivity,
                    topology_epoch: best.topology_epoch,
                    degradation: best.degradation,
                },
                best.updated_at_tick,
            ),
            backend_ref: BackendRouteRef {
                engine: BABEL_ENGINE_ID,
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
            > BABEL_CAPABILITIES.max_connectivity.partition
            || profile.selected_connectivity.repair > BABEL_CAPABILITIES.max_connectivity.repair
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
                profile: babel_assumptions(),
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
                    delivered: BABEL_CAPABILITIES.max_connectivity,
                },
                admission_profile: babel_assumptions(),
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

/// RFC 8966 feasibility condition for a single route entry.
///
/// Returns `true` if the entry passes the feasibility condition against `fd`:
/// - FD absent (= ∞): always feasible.
/// - FD present: feasible if `entry.seqno` is strictly newer (modular), or if
///   `seqno` is equal and `entry.metric` is strictly less than `fd.metric`.
pub(crate) fn route_is_feasible(entry: &RouteEntry, fd: Option<&FeasibilityEntry>) -> bool {
    match fd {
        None => true,
        Some(fd) => {
            seqno_is_newer(entry.seqno, fd.seqno)
                || (entry.seqno == fd.seqno && entry.metric < fd.metric)
        }
    }
}

pub(crate) fn babel_assumptions() -> AdmissionAssumptions {
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

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        Belief, ByteCount, Configuration, ControllerId, DurationMs, EndpointLocator, Link,
        LinkEndpoint, LinkProfile, LinkRuntimeState, LinkState, Node, NodeProfile, NodeState,
        RatioPermille, RepairCapability, RouteEpoch, RoutingTickChange, RoutingTickContext, Tick,
        TransportKind,
    };
    use jacquard_mem_link_profile::{InMemoryRuntimeEffects, InMemoryTransport};
    use jacquard_traits::RoutingEngine;

    use super::*;
    use crate::{public_state::DecayWindow, BabelEngine};

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

    fn fixture_link(remote: u8) -> Link {
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
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::certain(RatioPermille(900), Tick(1)),
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
                    ((node(1), node(2)), fixture_link(2)),
                    ((node(2), node(1)), fixture_link(1)),
                    ((node(2), node(3)), fixture_link(3)),
                    ((node(3), node(2)), fixture_link(2)),
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

    #[test]
    fn no_candidates_before_updates_received() {
        let mut engine = BabelEngine::new(
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
    fn stale_route_entries_decay_from_route_table() {
        let mut engine = BabelEngine::with_decay_window(
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
        // Inject an entry directly so there is something to decay.
        let update = BabelUpdate {
            destination: node(3),
            router_id: node(3),
            seqno: 1,
            metric: 0,
        };
        engine.ingest_update(node(2), &update, &topology, Tick(1));
        engine.refresh_private_state(&topology, Tick(1));
        assert!(!engine.route_table.is_empty());

        // Advance time past the stale threshold.
        let stale_topology = Observation {
            observed_at_tick: Tick(10),
            ..sample_topology()
        };
        engine.refresh_private_state(&stale_topology, Tick(10));

        assert!(engine.route_table.is_empty());
        assert!(engine.best_next_hops.is_empty());
    }

    #[test]
    fn engine_tick_reports_no_change_after_stable_state() {
        let mut engine = BabelEngine::new(
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
    fn feasibility_distance_set_on_first_selection() {
        let mut engine = BabelEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let topology = sample_topology();
        let update = BabelUpdate {
            destination: node(3),
            router_id: node(3),
            seqno: 5,
            metric: 0,
        };
        engine.ingest_update(node(2), &update, &topology, Tick(1));
        engine.refresh_private_state(&topology, Tick(1));

        let fd = engine.feasibility_distances.get(&node(3)).copied();
        assert!(
            fd.is_some(),
            "FD should be set after first feasible selection"
        );
        let fd = fd.unwrap();
        assert_eq!(fd.seqno, 5);
        // metric is the compound metric (link_cost + update.metric), not just 0
        assert!(
            fd.metric < crate::gossip::BABEL_INFINITY,
            "FD metric should be finite"
        );
    }

    #[test]
    fn route_with_same_seqno_and_worse_metric_is_infeasible() {
        let mut engine = BabelEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let topology = sample_topology();

        // Inject a good route for node(3) via node(2): seqno=1, metric=0 (node2's metric)
        let good = BabelUpdate {
            destination: node(3),
            router_id: node(3),
            seqno: 1,
            metric: 0,
        };
        engine.ingest_update(node(2), &good, &topology, Tick(1));
        engine.refresh_private_state(&topology, Tick(1));

        let good_fd = engine
            .feasibility_distances
            .get(&node(3))
            .copied()
            .expect("FD set");
        let good_metric = good_fd.metric;

        // Inject a worse route same seqno: metric after compounding will be >= good_fd.metric
        // Use a higher neighbor metric to ensure compound_metric >= good_fd.metric.
        // We'll directly verify infeasibility using the free function.
        let worse_entry = crate::public_state::RouteEntry {
            router_id: node(3),
            seqno: 1,
            metric: good_metric.saturating_add(100),
            observed_at_tick: Tick(1),
        };
        assert!(
            !route_is_feasible(&worse_entry, Some(&good_fd)),
            "same seqno, worse metric — should be infeasible"
        );
    }

    #[test]
    fn route_with_newer_seqno_is_always_feasible() {
        let fd = FeasibilityEntry {
            seqno: 3,
            metric: 500,
        };
        // newer seqno, even with worse metric
        let entry = crate::public_state::RouteEntry {
            router_id: node(2),
            seqno: 4,
            metric: 800,
            observed_at_tick: Tick(1),
        };
        assert!(
            route_is_feasible(&entry, Some(&fd)),
            "newer seqno should pass FC regardless of metric"
        );
    }

    #[test]
    fn feasibility_distance_cleared_on_route_expiry() {
        let mut engine = BabelEngine::with_decay_window(
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
        let update = BabelUpdate {
            destination: node(3),
            router_id: node(3),
            seqno: 1,
            metric: 0,
        };
        engine.ingest_update(node(2), &update, &topology, Tick(1));
        engine.refresh_private_state(&topology, Tick(1));
        assert!(
            engine.feasibility_distances.contains_key(&node(3)),
            "FD set after selection"
        );

        // Advance time past the stale threshold so the route expires.
        engine.refresh_private_state(&topology, Tick(10));
        assert!(
            !engine.feasibility_distances.contains_key(&node(3)),
            "FD should be cleared after route expires"
        );
    }

    #[test]
    fn seqno_is_newer_handles_wraparound() {
        use crate::scoring::seqno_is_newer;
        // Normal case
        assert!(seqno_is_newer(5, 3));
        assert!(!seqno_is_newer(3, 5));
        assert!(!seqno_is_newer(5, 5));
        // Wraparound: 0 is newer than 0xFFFF
        assert!(seqno_is_newer(0, 0xFFFF));
        // 0x8000 apart: not newer (ambiguous half — returns false)
        assert!(!seqno_is_newer(0x8000, 0));
        assert!(!seqno_is_newer(0, 0x8000));
    }
}
