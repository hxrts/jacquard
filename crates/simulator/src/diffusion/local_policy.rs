//! Simulator-local evidence policy state, scoring, reducers, and artifacts.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::coded_inference::{CodedArrivalClassification, CodedContactTraceEvent};

mod ablation;
mod baseline;
mod reducer;
mod score;

#[allow(unused_imports)]
pub(crate) use ablation::{
    run_local_policy_ablation, LocalPolicyAblationDecisionRecord, LocalPolicyAblationVariant,
};
#[allow(unused_imports)]
pub(crate) use baseline::run_local_evidence_policy_baseline;
#[allow(unused_imports)]
pub(crate) use reducer::{
    reduce_local_policy_forwarding, LocalPolicyDecisionRecord, LocalPolicyFragmentCandidate,
    LocalPolicyPeerCandidate, LocalPolicyReducerBudget, LocalPolicyRejectionReason,
};
#[allow(unused_imports)]
pub(crate) use score::{
    compare_scored_candidates, local_policy_score_from_input, LocalPolicyScoreBreakdown,
    LocalPolicyScoreCandidate, LocalPolicyScoreInput,
};

const LOCAL_POLICY_PEER_LIMIT: usize = 128;
const LOCAL_POLICY_RECENT_WINDOW_MAX: usize = 16;
const LOCAL_POLICY_CLUSTER_DIVERSITY_CAP: u32 = 4;
const PERMILLE_MAX: u32 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LocalPolicyError {
    PeerLimitExceeded,
    InvalidWindowLength,
    InvalidCapacity,
    InvalidPermille,
    ContactDoesNotIncludeLocalNode,
}

/// Bounded integer telemetry for one peer.
///
/// Contact rate, bridge score, and all pressure fields are permille values in
/// `0..=1000`. Rounds are simulator round indexes, not host time.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyPeerState {
    pub peer_node_id: u32,
    pub first_contact_round: u32,
    pub last_contact_round: u32,
    pub contact_event_count: u32,
    pub contact_rate_permille: u32,
    pub bridge_score_permille: u32,
    pub bridge_contact_count: u32,
    pub distinct_peer_cluster_count: u32,
}

/// Optional belief inputs are absent unless a scenario explicitly exposes them.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyBeliefInputs {
    pub receiver_likelihood_permille: Option<u32>,
    pub destination_region_belief_permille: Option<u32>,
    pub anomaly_region_belief_permille: Option<u32>,
}

/// Node-local state for interpretable evidence policy decisions.
///
/// Storage pressure is retained bytes divided by capacity bytes. Duplicate and
/// innovative-success rates use fixed bounded windows; rollover discards the
/// oldest entry before appending the new one. `r_est_permille` is measured
/// reproduction pressure from innovative successors over active opportunities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyState {
    pub local_node_id: u32,
    pub peers: BTreeMap<u32, LocalPolicyPeerState>,
    pub storage_capacity_bytes: u32,
    pub retained_payload_bytes: u32,
    pub storage_pressure_permille: u32,
    pub recent_duplicate_rate_permille: u32,
    pub recent_innovative_forward_success_rate_permille: u32,
    pub r_est_permille: u32,
    pub belief_inputs: LocalPolicyBeliefInputs,
    duplicate_window: RecentBoolWindow,
    innovative_success_window: RecentBoolWindow,
    peer_clusters: BTreeMap<u32, BTreeSet<u8>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum LocalPolicyArrivalKind {
    Innovative,
    Duplicate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum LocalPolicyStateTraceEvent {
    Contact {
        round_index: u32,
        peer_node_id: u32,
        peer_cluster_id: u8,
        bridge_contact: bool,
    },
    Arrival {
        arrival_kind: LocalPolicyArrivalKind,
    },
    ForwardResult {
        innovative_success: bool,
    },
    Storage {
        retained_payload_bytes: u32,
        storage_capacity_bytes: u32,
    },
    Reproduction {
        active_forwarding_opportunities: u32,
        innovative_successor_opportunities: u32,
    },
    Belief {
        inputs: LocalPolicyBeliefInputs,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RecentBoolWindow {
    capacity: usize,
    entries: Vec<bool>,
}

impl LocalPolicyPeerState {
    fn new(peer_node_id: u32, round_index: u32) -> Self {
        Self {
            peer_node_id,
            first_contact_round: round_index,
            last_contact_round: round_index,
            contact_event_count: 0,
            contact_rate_permille: 0,
            bridge_score_permille: 0,
            bridge_contact_count: 0,
            distinct_peer_cluster_count: 0,
        }
    }

    fn record_contact(
        &mut self,
        round_index: u32,
        bridge_contact: bool,
        distinct_peer_cluster_count: u32,
    ) {
        self.last_contact_round = self.last_contact_round.max(round_index);
        self.contact_event_count = self.contact_event_count.saturating_add(1);
        if bridge_contact {
            self.bridge_contact_count = self.bridge_contact_count.saturating_add(1);
        }
        self.distinct_peer_cluster_count = distinct_peer_cluster_count;
        self.contact_rate_permille = contact_rate_permille(
            self.contact_event_count,
            self.first_contact_round,
            self.last_contact_round,
        );
        self.bridge_score_permille = bridge_score_permille(
            self.bridge_contact_count,
            self.contact_event_count,
            self.distinct_peer_cluster_count,
        );
    }
}

impl LocalPolicyBeliefInputs {
    pub(crate) fn try_new(
        receiver_likelihood_permille: Option<u32>,
        destination_region_belief_permille: Option<u32>,
        anomaly_region_belief_permille: Option<u32>,
    ) -> Result<Self, LocalPolicyError> {
        for value in [
            receiver_likelihood_permille,
            destination_region_belief_permille,
            anomaly_region_belief_permille,
        ] {
            if value.unwrap_or(0) > PERMILLE_MAX {
                return Err(LocalPolicyError::InvalidPermille);
            }
        }
        Ok(Self {
            receiver_likelihood_permille,
            destination_region_belief_permille,
            anomaly_region_belief_permille,
        })
    }
}

impl LocalPolicyState {
    pub(crate) fn try_new(
        local_node_id: u32,
        storage_capacity_bytes: u32,
    ) -> Result<Self, LocalPolicyError> {
        if storage_capacity_bytes == 0 {
            return Err(LocalPolicyError::InvalidCapacity);
        }
        Ok(Self {
            local_node_id,
            peers: BTreeMap::new(),
            storage_capacity_bytes,
            retained_payload_bytes: 0,
            storage_pressure_permille: 0,
            recent_duplicate_rate_permille: 0,
            recent_innovative_forward_success_rate_permille: 0,
            r_est_permille: 0,
            belief_inputs: LocalPolicyBeliefInputs::default(),
            duplicate_window: RecentBoolWindow::try_new(LOCAL_POLICY_RECENT_WINDOW_MAX)?,
            innovative_success_window: RecentBoolWindow::try_new(LOCAL_POLICY_RECENT_WINDOW_MAX)?,
            peer_clusters: BTreeMap::new(),
        })
    }

    pub(crate) fn record_contact(
        &mut self,
        round_index: u32,
        peer_node_id: u32,
        peer_cluster_id: u8,
        bridge_contact: bool,
    ) -> Result<(), LocalPolicyError> {
        self.ensure_peer(peer_node_id, round_index)?;
        let clusters = self.peer_clusters.entry(peer_node_id).or_default();
        clusters.insert(peer_cluster_id);
        let distinct_cluster_count = u32::try_from(clusters.len()).unwrap_or(u32::MAX);
        let peer = self
            .peers
            .get_mut(&peer_node_id)
            .expect("peer was inserted above");
        peer.record_contact(round_index, bridge_contact, distinct_cluster_count);
        Ok(())
    }

    pub(crate) fn record_contact_trace_event(
        &mut self,
        event: &CodedContactTraceEvent,
    ) -> Result<(), LocalPolicyError> {
        let (peer_node_id, peer_cluster_id, bridge_contact) = self.peer_from_trace_event(event)?;
        self.record_contact(
            event.round_index,
            peer_node_id,
            peer_cluster_id,
            bridge_contact,
        )
    }

    pub(crate) fn record_arrival(&mut self, arrival_kind: LocalPolicyArrivalKind) {
        self.duplicate_window
            .push(arrival_kind == LocalPolicyArrivalKind::Duplicate);
        self.recent_duplicate_rate_permille = self.duplicate_window.true_rate_permille();
    }

    pub(crate) fn record_forward_result(&mut self, innovative_success: bool) {
        self.innovative_success_window.push(innovative_success);
        self.recent_innovative_forward_success_rate_permille =
            self.innovative_success_window.true_rate_permille();
    }

    pub(crate) fn update_storage(
        &mut self,
        retained_payload_bytes: u32,
        storage_capacity_bytes: u32,
    ) -> Result<(), LocalPolicyError> {
        if storage_capacity_bytes == 0 {
            return Err(LocalPolicyError::InvalidCapacity);
        }
        self.storage_capacity_bytes = storage_capacity_bytes;
        self.retained_payload_bytes = retained_payload_bytes.min(storage_capacity_bytes);
        self.storage_pressure_permille =
            ratio_permille(self.retained_payload_bytes, self.storage_capacity_bytes);
        Ok(())
    }

    pub(crate) fn update_reproduction_estimate(
        &mut self,
        active_forwarding_opportunities: u32,
        innovative_successor_opportunities: u32,
    ) {
        self.r_est_permille = ratio_permille(
            innovative_successor_opportunities,
            active_forwarding_opportunities,
        );
    }

    pub(crate) fn set_belief_inputs(
        &mut self,
        belief_inputs: LocalPolicyBeliefInputs,
    ) -> Result<(), LocalPolicyError> {
        LocalPolicyBeliefInputs::try_new(
            belief_inputs.receiver_likelihood_permille,
            belief_inputs.destination_region_belief_permille,
            belief_inputs.anomaly_region_belief_permille,
        )?;
        self.belief_inputs = belief_inputs;
        Ok(())
    }

    pub(crate) fn apply_trace_event(
        &mut self,
        event: &LocalPolicyStateTraceEvent,
    ) -> Result<(), LocalPolicyError> {
        match *event {
            LocalPolicyStateTraceEvent::Contact {
                round_index,
                peer_node_id,
                peer_cluster_id,
                bridge_contact,
            } => self.record_contact(round_index, peer_node_id, peer_cluster_id, bridge_contact),
            LocalPolicyStateTraceEvent::Arrival { arrival_kind } => {
                self.record_arrival(arrival_kind);
                Ok(())
            }
            LocalPolicyStateTraceEvent::ForwardResult { innovative_success } => {
                self.record_forward_result(innovative_success);
                Ok(())
            }
            LocalPolicyStateTraceEvent::Storage {
                retained_payload_bytes,
                storage_capacity_bytes,
            } => self.update_storage(retained_payload_bytes, storage_capacity_bytes),
            LocalPolicyStateTraceEvent::Reproduction {
                active_forwarding_opportunities,
                innovative_successor_opportunities,
            } => {
                self.update_reproduction_estimate(
                    active_forwarding_opportunities,
                    innovative_successor_opportunities,
                );
                Ok(())
            }
            LocalPolicyStateTraceEvent::Belief { inputs } => self.set_belief_inputs(inputs),
        }
    }

    fn ensure_peer(&mut self, peer_node_id: u32, round_index: u32) -> Result<(), LocalPolicyError> {
        if self.peers.contains_key(&peer_node_id) {
            return Ok(());
        }
        if self.peers.len() >= LOCAL_POLICY_PEER_LIMIT {
            return Err(LocalPolicyError::PeerLimitExceeded);
        }
        self.peers.insert(
            peer_node_id,
            LocalPolicyPeerState::new(peer_node_id, round_index),
        );
        Ok(())
    }

    fn peer_from_trace_event(
        &self,
        event: &CodedContactTraceEvent,
    ) -> Result<(u32, u8, bool), LocalPolicyError> {
        if event.node_a == self.local_node_id {
            return Ok((
                event.node_b,
                event.cluster_b,
                event.cluster_a != event.cluster_b,
            ));
        }
        if event.node_b == self.local_node_id {
            return Ok((
                event.node_a,
                event.cluster_a,
                event.cluster_a != event.cluster_b,
            ));
        }
        Err(LocalPolicyError::ContactDoesNotIncludeLocalNode)
    }
}

impl RecentBoolWindow {
    fn try_new(capacity: usize) -> Result<Self, LocalPolicyError> {
        if capacity == 0 || capacity > LOCAL_POLICY_RECENT_WINDOW_MAX {
            return Err(LocalPolicyError::InvalidWindowLength);
        }
        Ok(Self {
            capacity,
            entries: Vec::new(),
        })
    }

    fn push(&mut self, value: bool) {
        if self.entries.len() == self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(value);
    }

    fn true_rate_permille(&self) -> u32 {
        let true_count = self.entries.iter().filter(|entry| **entry).count();
        ratio_permille(
            u32::try_from(true_count).unwrap_or(u32::MAX),
            u32::try_from(self.entries.len()).unwrap_or(u32::MAX),
        )
    }
}

pub(crate) fn local_policy_state_from_trace(
    local_node_id: u32,
    storage_capacity_bytes: u32,
    trace: &[LocalPolicyStateTraceEvent],
) -> Result<LocalPolicyState, LocalPolicyError> {
    let mut state = LocalPolicyState::try_new(local_node_id, storage_capacity_bytes)?;
    for event in trace {
        state.apply_trace_event(event)?;
    }
    Ok(state)
}

fn contact_rate_permille(contact_count: u32, first_round: u32, last_round: u32) -> u32 {
    let round_span = last_round.saturating_sub(first_round).saturating_add(1);
    ratio_permille(contact_count.min(round_span), round_span)
}

fn bridge_score_permille(
    bridge_contact_count: u32,
    contact_count: u32,
    distinct_peer_cluster_count: u32,
) -> u32 {
    let bridge_component = ratio_permille(bridge_contact_count, contact_count).saturating_mul(7);
    let diversity_component = ratio_permille(
        distinct_peer_cluster_count.min(LOCAL_POLICY_CLUSTER_DIVERSITY_CAP),
        LOCAL_POLICY_CLUSTER_DIVERSITY_CAP,
    )
    .saturating_mul(3);
    bridge_component
        .saturating_add(diversity_component)
        .saturating_div(10)
        .min(PERMILLE_MAX)
}

fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    numerator
        .saturating_mul(PERMILLE_MAX)
        .saturating_div(denominator)
        .min(PERMILLE_MAX)
}

impl From<CodedArrivalClassification> for LocalPolicyArrivalKind {
    fn from(value: CodedArrivalClassification) -> Self {
        match value {
            CodedArrivalClassification::Innovative => Self::Innovative,
            CodedArrivalClassification::Duplicate => Self::Duplicate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        local_policy_state_from_trace, LocalPolicyArrivalKind, LocalPolicyBeliefInputs,
        LocalPolicyError, LocalPolicyState, LocalPolicyStateTraceEvent,
        LOCAL_POLICY_RECENT_WINDOW_MAX,
    };
    use crate::diffusion::{
        coded_inference::CodedContactTraceEvent, model::DiffusionTransportKind,
    };

    #[test]
    fn local_policy_state_empty_input_starts_absent_optional_beliefs() {
        let state = LocalPolicyState::try_new(7, 512).expect("state");

        assert!(state.peers.is_empty());
        assert_eq!(state.storage_pressure_permille, 0);
        assert_eq!(state.recent_duplicate_rate_permille, 0);
        assert_eq!(state.recent_innovative_forward_success_rate_permille, 0);
        assert_eq!(state.r_est_permille, 0);
        assert_eq!(state.belief_inputs, LocalPolicyBeliefInputs::default());
    }

    #[test]
    fn local_policy_state_updates_contact_rate_and_bridge_score_per_peer() {
        let mut state = LocalPolicyState::try_new(7, 512).expect("state");

        state.record_contact(2, 11, 1, true).expect("contact");
        state.record_contact(3, 11, 2, true).expect("contact");
        state.record_contact(5, 11, 2, false).expect("contact");

        let peer = state.peers.get(&11).expect("peer");
        assert_eq!(peer.contact_event_count, 3);
        assert_eq!(peer.contact_rate_permille, 750);
        assert_eq!(peer.distinct_peer_cluster_count, 2);
        assert!(peer.bridge_score_permille > 0);
    }

    #[test]
    fn local_policy_state_repeated_peer_input_is_canonical() {
        let mut state = LocalPolicyState::try_new(7, 512).expect("state");

        for round_index in 0..4 {
            state
                .record_contact(round_index, 11, 1, false)
                .expect("contact");
        }

        assert_eq!(state.peers.len(), 1);
        assert_eq!(state.peers.get(&11).expect("peer").contact_event_count, 4);
        assert_eq!(
            state.peers.get(&11).expect("peer").contact_rate_permille,
            1000
        );
    }

    #[test]
    fn local_policy_state_recent_windows_roll_over_deterministically() {
        let mut state = LocalPolicyState::try_new(7, 512).expect("state");

        for _ in 0..LOCAL_POLICY_RECENT_WINDOW_MAX {
            state.record_arrival(LocalPolicyArrivalKind::Duplicate);
        }
        state.record_arrival(LocalPolicyArrivalKind::Innovative);
        state.record_forward_result(false);
        state.record_forward_result(true);

        assert_eq!(state.recent_duplicate_rate_permille, 937);
        assert_eq!(state.recent_innovative_forward_success_rate_permille, 500);
    }

    #[test]
    fn local_policy_state_storage_and_reproduction_are_bounded() {
        let mut state = LocalPolicyState::try_new(7, 512).expect("state");

        state.update_storage(900, 512).expect("storage");
        state.update_reproduction_estimate(2, 9);

        assert_eq!(state.retained_payload_bytes, 512);
        assert_eq!(state.storage_pressure_permille, 1000);
        assert_eq!(state.r_est_permille, 1000);
        assert_eq!(
            state.update_storage(1, 0),
            Err(LocalPolicyError::InvalidCapacity)
        );
    }

    #[test]
    fn local_policy_state_optional_beliefs_require_explicit_scenario_input() {
        let mut state = LocalPolicyState::try_new(7, 512).expect("state");
        let beliefs =
            LocalPolicyBeliefInputs::try_new(Some(800), None, Some(600)).expect("beliefs");

        state.set_belief_inputs(beliefs).expect("set beliefs");

        assert_eq!(state.belief_inputs, beliefs);
        assert_eq!(
            LocalPolicyBeliefInputs::try_new(Some(1001), None, None),
            Err(LocalPolicyError::InvalidPermille)
        );
    }

    #[test]
    fn local_policy_state_replays_event_trace_deterministically() {
        let trace = vec![
            LocalPolicyStateTraceEvent::Contact {
                round_index: 0,
                peer_node_id: 11,
                peer_cluster_id: 1,
                bridge_contact: true,
            },
            LocalPolicyStateTraceEvent::Arrival {
                arrival_kind: LocalPolicyArrivalKind::Duplicate,
            },
            LocalPolicyStateTraceEvent::ForwardResult {
                innovative_success: true,
            },
            LocalPolicyStateTraceEvent::Storage {
                retained_payload_bytes: 128,
                storage_capacity_bytes: 512,
            },
            LocalPolicyStateTraceEvent::Reproduction {
                active_forwarding_opportunities: 4,
                innovative_successor_opportunities: 3,
            },
        ];

        let first = local_policy_state_from_trace(7, 512, &trace).expect("first");
        let second = local_policy_state_from_trace(7, 512, &trace).expect("second");

        assert_eq!(first, second);
        assert_eq!(first.storage_pressure_permille, 250);
        assert_eq!(first.r_est_permille, 750);
    }

    #[test]
    fn local_policy_state_contact_trace_requires_local_node() {
        let mut state = LocalPolicyState::try_new(7, 512).expect("state");
        let event = CodedContactTraceEvent {
            round_index: 0,
            contact_id: 1,
            node_a: 1,
            node_b: 2,
            cluster_a: 0,
            cluster_b: 1,
            transport_kind: DiffusionTransportKind::Ble,
            bandwidth_bytes: 64,
            connection_delay: 0,
            energy_cost_per_byte: 1,
            contact_window: 1,
        };

        assert_eq!(
            state.record_contact_trace_event(&event),
            Err(LocalPolicyError::ContactDoesNotIncludeLocalNode)
        );
    }
}
