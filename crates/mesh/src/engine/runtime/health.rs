//! Engine-private health synthesis from topology, transport, and active-route state.
//!
//! Control flow: `engine_tick` folds raw transport observations into a bounded
//! summary and control state, then runtime operations ask this module to turn
//! the latest topology plus the active route suffix into route-scoped health.
//! The result is the shared `RouteHealth` surface without exposing mesh's
//! private accumulators directly.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    Configuration, HealthScore, Observation, PenaltyPoints, ReachabilityState,
    RouteHealth, TransportObservation,
};
use jacquard_traits::MeshNeighborhoodEstimateAccess;

use super::{
    super::{
        ActiveMeshRoute, MeshControlState, MeshObservedRemoteLink,
        MeshTransportFreshness, MeshTransportObservationSummary,
    },
    MeshEffectsBounds, MeshEngine, MeshHasherBounds, MeshSelectorBounds,
    MeshTransportBounds,
};

struct TransportObservationAccumulator {
    last_observed_at_tick:  Option<jacquard_core::Tick>,
    payload_event_count:    u16,
    observed_link_count:    u16,
    reachable_remote_nodes: BTreeSet<jacquard_core::NodeId>,
    stability_sum:          u32,
    loss_sum:               u32,
    remote_links:           BTreeMap<jacquard_core::NodeId, MeshObservedRemoteLink>,
}

impl TransportObservationAccumulator {
    fn new() -> Self {
        Self {
            last_observed_at_tick:  None,
            payload_event_count:    0,
            observed_link_count:    0,
            reachable_remote_nodes: BTreeSet::new(),
            stability_sum:          0,
            loss_sum:               0,
            remote_links:           BTreeMap::new(),
        }
    }

    fn observe(&mut self, observation: &TransportObservation) {
        match observation {
            | TransportObservation::PayloadReceived {
                from_node_id,
                observed_at_tick,
                ..
            } => self.observe_payload(*from_node_id, *observed_at_tick),
            | TransportObservation::LinkObserved { remote_node_id, observation } => {
                self.observe_link(*remote_node_id, observation)
            },
        }
    }

    fn observe_payload(
        &mut self,
        from_node_id: jacquard_core::NodeId,
        observed_at_tick: jacquard_core::Tick,
    ) {
        self.payload_event_count = self.payload_event_count.saturating_add(1);
        self.reachable_remote_nodes.insert(from_node_id);
        self.last_observed_at_tick = Some(
            self.last_observed_at_tick
                .map_or(observed_at_tick, |current| current.max(observed_at_tick)),
        );
    }

    fn observe_link(
        &mut self,
        remote_node_id: jacquard_core::NodeId,
        observation: &Observation<jacquard_core::Link>,
    ) {
        self.observed_link_count = self.observed_link_count.saturating_add(1);
        self.reachable_remote_nodes.insert(remote_node_id);
        self.last_observed_at_tick = Some(
            self.last_observed_at_tick
                .map_or(observation.observed_at_tick, |current| {
                    current.max(observation.observed_at_tick)
                }),
        );
        let stability_score = Self::observed_link_stability_score(observation);
        let congestion_penalty_points =
            PenaltyPoints(u32::from(observation.value.state.loss_permille.get()) / 100);
        self.stability_sum = self.stability_sum.saturating_add(stability_score.0);
        self.loss_sum = self.loss_sum.saturating_add(congestion_penalty_points.0);
        self.remote_links.insert(
            remote_node_id,
            MeshObservedRemoteLink {
                last_observed_at_tick: observation.observed_at_tick,
                stability_score,
                congestion_penalty_points,
            },
        );
    }

    fn observed_link_stability_score(
        observation: &Observation<jacquard_core::Link>,
    ) -> HealthScore {
        let delivery = match &observation.value.state.delivery_confidence_permille {
            | jacquard_core::Belief::Absent => 0,
            | jacquard_core::Belief::Estimated(estimate) => {
                u32::from(estimate.value.get())
            },
        };
        let symmetry = match &observation.value.state.symmetry_permille {
            | jacquard_core::Belief::Absent => 0,
            | jacquard_core::Belief::Estimated(estimate) => {
                u32::from(estimate.value.get())
            },
        };
        HealthScore((delivery.saturating_add(symmetry)) / 2)
    }

    fn finish(self) -> Option<MeshTransportObservationSummary> {
        let last_observed_at_tick = self.last_observed_at_tick?;
        let reachable_remote_count =
            u16::try_from(self.reachable_remote_nodes.len()).unwrap_or(u16::MAX);
        // Payload events confirm reachability but carry no link-quality
        // data. Use 500 (mid-scale) as "reachable, unknown quality"
        // rather than collapsing to zero.
        let stability_score = if self.observed_link_count > 0 {
            HealthScore(self.stability_sum / u32::from(self.observed_link_count))
        } else if self.payload_event_count > 0 {
            HealthScore(500)
        } else {
            HealthScore(0)
        };
        let congestion_penalty_points = if self.observed_link_count > 0 {
            PenaltyPoints(self.loss_sum / u32::from(self.observed_link_count))
        } else {
            PenaltyPoints(0)
        };
        Some(MeshTransportObservationSummary {
            last_observed_at_tick: Some(last_observed_at_tick),
            payload_event_count: self.payload_event_count,
            observed_link_count: self.observed_link_count,
            reachable_remote_count,
            freshness: MeshTransportFreshness::Fresh,
            stability_score,
            congestion_penalty_points,
            remote_links: self.remote_links,
        })
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::MeshTopologyBounds,
    Topology::PeerEstimate: jacquard_traits::MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: jacquard_traits::MeshNeighborhoodEstimateAccess,
    Transport: MeshTransportBounds,
    Retention: super::super::MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    pub(super) fn summarize_transport_observations(
        observations: &[TransportObservation],
    ) -> Option<MeshTransportObservationSummary> {
        let mut accumulator = TransportObservationAccumulator::new();
        for observation in observations {
            accumulator.observe(observation);
        }
        accumulator.finish()
    }

    pub(super) fn next_transport_summary(
        previous: Option<&MeshTransportObservationSummary>,
        observed: Option<MeshTransportObservationSummary>,
        now: jacquard_core::Tick,
    ) -> Option<MeshTransportObservationSummary> {
        const QUIET_STALE_TICKS: u64 = 3;
        const QUIET_DECAY_STEP: u32 = 100;

        if let Some(observed) = observed {
            return Some(observed);
        }

        let previous = previous?.clone();
        let last_observed_at_tick = previous.last_observed_at_tick?;
        let quiet_ticks = now.0.saturating_sub(last_observed_at_tick.0);
        let freshness =
            Self::transport_freshness_for_quiet_ticks(quiet_ticks, QUIET_STALE_TICKS);
        let decay = u32::try_from(quiet_ticks)
            .unwrap_or(u32::MAX)
            .saturating_mul(QUIET_DECAY_STEP);

        // Preserve reachable_remote_count so the topology model retains
        // neighbor knowledge during quiet periods. Zero the per-tick
        // counters to avoid double-counting across ticks.
        Some(MeshTransportObservationSummary {
            last_observed_at_tick: Some(last_observed_at_tick),
            payload_event_count: 0,
            observed_link_count: 0,
            reachable_remote_count: previous.reachable_remote_count,
            freshness,
            stability_score: HealthScore(
                previous.stability_score.0.saturating_sub(decay),
            ),
            congestion_penalty_points: previous.congestion_penalty_points,
            remote_links: Self::decayed_remote_links(previous.remote_links, decay),
        })
    }

    fn transport_freshness_for_quiet_ticks(
        quiet_ticks: u64,
        quiet_stale_ticks: u64,
    ) -> MeshTransportFreshness {
        if quiet_ticks == 0 {
            MeshTransportFreshness::Fresh
        } else if quiet_ticks >= quiet_stale_ticks {
            MeshTransportFreshness::Stale
        } else {
            MeshTransportFreshness::Quiet
        }
    }

    fn decayed_remote_links(
        remote_links: BTreeMap<jacquard_core::NodeId, MeshObservedRemoteLink>,
        decay: u32,
    ) -> BTreeMap<jacquard_core::NodeId, MeshObservedRemoteLink> {
        remote_links
            .into_iter()
            .map(|(node_id, mut remote)| {
                remote.stability_score =
                    HealthScore(remote.stability_score.0.saturating_sub(decay));
                (node_id, remote)
            })
            .collect()
    }

    pub(super) fn next_control_state(
        &self,
        topology: &Observation<Configuration>,
        transport_summary: Option<&MeshTransportObservationSummary>,
    ) -> MeshControlState {
        let previous = self.control_state.as_ref();
        let neighborhood_repair_pressure = self
            .topology_model
            .neighborhood_estimate(
                &self.local_node_id,
                topology.observed_at_tick,
                &topology.value,
            )
            .as_ref()
            .and_then(|estimate| estimate.repair_pressure_score())
            .map_or(0, |score| score.0);
        let transport_stability =
            self.transport_stability_score(previous, transport_summary);
        let observed_pressure = Self::observed_pressure_score(transport_summary);
        let anti_entropy_pressure =
            self.anti_entropy_pressure(previous, observed_pressure);
        // Halve observed pressure before adding to the neighborhood signal.
        // This keeps transient congestion spikes from overwhelming a stable
        // topology reading. Combined score is capped at the 0..=1000 scale.
        let repair_pressure = neighborhood_repair_pressure
            .saturating_add(observed_pressure / 2)
            .min(1000);

        MeshControlState {
            last_updated_at_tick:      topology.observed_at_tick,
            transport_stability_score: HealthScore(transport_stability.min(1000)),
            repair_pressure_score:     HealthScore(repair_pressure),
            anti_entropy:              super::super::types::MeshAntiEntropyState {
                pressure_score:         HealthScore(anti_entropy_pressure),
                last_refreshed_at_tick: previous
                    .and_then(|state| state.anti_entropy.last_refreshed_at_tick),
            },
        }
    }

    fn transport_stability_score(
        &self,
        previous: Option<&MeshControlState>,
        transport_summary: Option<&MeshTransportObservationSummary>,
    ) -> u32 {
        transport_summary
            .map(|summary| summary.stability_score.0)
            .unwrap_or_else(|| {
                previous.map_or(0, |state| {
                    state.transport_stability_score.0.saturating_sub(100)
                })
            })
    }

    fn observed_pressure_score(
        transport_summary: Option<&MeshTransportObservationSummary>,
    ) -> u32 {
        transport_summary.map_or(0, |summary| {
            let quiet_pressure: u32 = match summary.freshness {
                | MeshTransportFreshness::Fresh => 0,
                | MeshTransportFreshness::Quiet => 100,
                | MeshTransportFreshness::Stale => 250,
            };
            quiet_pressure
                .saturating_add(summary.congestion_penalty_points.0.saturating_mul(50))
                .saturating_add(1000_u32.saturating_sub(summary.stability_score.0) / 2)
        })
    }

    fn anti_entropy_pressure(
        &self,
        previous: Option<&MeshControlState>,
        observed_pressure: u32,
    ) -> u32 {
        let previous_pressure = previous.map_or(0, |state| {
            state.anti_entropy.pressure_score.0.saturating_sub(150)
        });
        previous_pressure
            .saturating_add(observed_pressure)
            .min(1000)
    }

    pub(super) fn consume_anti_entropy_pressure(&mut self, now: jacquard_core::Tick) {
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

    // Suppress the last repair step under moderate mesh pressure (>300) to
    // avoid burning the final budget during a transient congestion burst.
    // Repairs are unrestricted when multiple steps remain.
    pub(super) fn repair_allowed(&self, active_route: &ActiveMeshRoute) -> bool {
        if active_route.repair.steps_remaining == 0 {
            return false;
        }
        self.control_state.as_ref().is_none_or(|state| {
            !(state.repair_pressure_score.0 > 300
                && active_route.repair.steps_remaining <= 1)
        })
    }

    pub(super) fn current_route_health(
        &self,
        active_route: Option<&ActiveMeshRoute>,
        now: jacquard_core::Tick,
    ) -> RouteHealth {
        let Some(active_route) = active_route else {
            return Self::unknown_route_health(now);
        };
        let Some(topology) = self.latest_topology.as_ref() else {
            return Self::unknown_route_health(now);
        };

        let remaining_segments = &active_route.path.segments
            [usize::from(active_route.forwarding.next_hop_index)..];
        if remaining_segments.is_empty() {
            return Self::terminal_route_health(topology.observed_at_tick);
        }

        let mut health = RouteHealth {
            reachability_state:        ReachabilityState::Reachable,
            stability_score:           HealthScore(1000),
            congestion_penalty_points: PenaltyPoints(0),
            last_validated_at_tick:    topology.observed_at_tick,
        };
        Self::apply_first_hop_transport_summary(
            &mut health,
            active_route,
            self.last_transport_summary.as_ref(),
        );
        Self::apply_path_health(&mut health, active_route, topology);
        self.apply_control_state_health(&mut health);
        health
    }

    fn unknown_route_health(now: jacquard_core::Tick) -> RouteHealth {
        RouteHealth {
            reachability_state:        ReachabilityState::Unknown,
            stability_score:           HealthScore(0),
            congestion_penalty_points: PenaltyPoints(0),
            last_validated_at_tick:    now,
        }
    }

    fn terminal_route_health(validated_at_tick: jacquard_core::Tick) -> RouteHealth {
        RouteHealth {
            reachability_state:        ReachabilityState::Reachable,
            stability_score:           HealthScore(1000),
            congestion_penalty_points: PenaltyPoints(0),
            last_validated_at_tick:    validated_at_tick,
        }
    }

    fn apply_first_hop_transport_summary(
        health: &mut RouteHealth,
        active_route: &ActiveMeshRoute,
        summary: Option<&MeshTransportObservationSummary>,
    ) {
        let Some(next_segment) = active_route
            .path
            .segments
            .get(usize::from(active_route.forwarding.next_hop_index))
        else {
            return;
        };
        let Some(summary) = summary else {
            return;
        };
        let Some(remote) = summary.remote_links.get(&next_segment.node_id) else {
            return;
        };
        health.stability_score =
            HealthScore(health.stability_score.0.min(remote.stability_score.0));
        health.congestion_penalty_points = PenaltyPoints(
            health
                .congestion_penalty_points
                .0
                .max(remote.congestion_penalty_points.0),
        );
        health.last_validated_at_tick = health
            .last_validated_at_tick
            .max(remote.last_observed_at_tick);
    }

    fn apply_path_health(
        health: &mut RouteHealth,
        active_route: &ActiveMeshRoute,
        topology: &Observation<Configuration>,
    ) {
        let remaining_segments = &active_route.path.segments
            [usize::from(active_route.forwarding.next_hop_index)..];
        let mut current_node_id = active_route.forwarding.current_owner_node_id;
        for segment in remaining_segments {
            let Some(link) = crate::topology::adjacent_link_between(
                &current_node_id,
                &segment.node_id,
                &topology.value,
            ) else {
                health.reachability_state = ReachabilityState::Unreachable;
                break;
            };
            Self::apply_link_health(health, link);
            current_node_id = segment.node_id;
        }
    }

    fn apply_link_health(health: &mut RouteHealth, link: &jacquard_core::Link) {
        let delivery = match &link.state.delivery_confidence_permille {
            | jacquard_core::Belief::Absent => None,
            | jacquard_core::Belief::Estimated(estimate) => {
                Some(u32::from(estimate.value.get()))
            },
        };
        let symmetry = match &link.state.symmetry_permille {
            | jacquard_core::Belief::Absent => None,
            | jacquard_core::Belief::Estimated(estimate) => {
                Some(u32::from(estimate.value.get()))
            },
        };
        let link_stability = match (delivery, symmetry) {
            | (Some(delivery), Some(symmetry)) => Some((delivery + symmetry) / 2),
            | (Some(delivery), None) => Some(delivery),
            | (None, Some(symmetry)) => Some(symmetry),
            | (None, None) => None,
        };
        if let Some(link_stability) = link_stability {
            health.stability_score =
                HealthScore(health.stability_score.0.min(link_stability));
        }
        health.congestion_penalty_points = PenaltyPoints(
            health
                .congestion_penalty_points
                .0
                .max(u32::from(link.state.loss_permille.get()) / 100),
        );
    }

    fn apply_control_state_health(&self, health: &mut RouteHealth) {
        if let Some(control_state) = self.control_state.as_ref() {
            health.stability_score = HealthScore(
                health
                    .stability_score
                    .0
                    .min(control_state.transport_stability_score.0),
            );
            health.congestion_penalty_points = PenaltyPoints(
                health
                    .congestion_penalty_points
                    .0
                    .max(control_state.anti_entropy.pressure_score.0 / 100),
            );
        }
    }
}
