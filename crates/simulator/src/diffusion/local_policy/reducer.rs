//! Deterministic local evidence forwarding reducer.

use serde::{Deserialize, Serialize};

use super::{
    compare_scored_candidates, local_policy_score_from_input, LocalPolicyScoreBreakdown,
    LocalPolicyScoreCandidate, LocalPolicyScoreInput, LocalPolicyState,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyPeerCandidate {
    pub peer_node_id: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyFragmentCandidate {
    pub fragment_id: u32,
    pub payload_bytes: u32,
    pub expected_innovation_gain: u32,
    pub landscape_value: u32,
    pub demand_value: u32,
    pub duplicate_risk_hint: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyReducerBudget {
    pub payload_byte_budget_remaining: u32,
    pub storage_payload_units_remaining: u32,
    pub reproduction_target_max_permille: u32,
    pub max_forwarding_decisions: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum LocalPolicyRejectionReason {
    PayloadByteBudget,
    StorageBudget,
    ReproductionBudget,
    ForwardingDecisionLimit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyDecisionRecord {
    pub policy_id: String,
    pub peer_node_id: u32,
    pub fragment_id: u32,
    pub selected: bool,
    pub rejection_reason: Option<LocalPolicyRejectionReason>,
    pub total_score: i32,
    pub score: LocalPolicyScoreBreakdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ScoredPair {
    peer: LocalPolicyPeerCandidate,
    fragment: LocalPolicyFragmentCandidate,
    score: LocalPolicyScoreBreakdown,
}

pub(crate) fn reduce_local_policy_forwarding(
    state: &LocalPolicyState,
    peer_candidates: &[LocalPolicyPeerCandidate],
    fragment_candidates: &[LocalPolicyFragmentCandidate],
    budget: LocalPolicyReducerBudget,
) -> Vec<LocalPolicyDecisionRecord> {
    let mut scored = score_pairs(state, peer_candidates, fragment_candidates);
    scored.sort_by(compare_pairs);
    let mut payload_remaining = budget.payload_byte_budget_remaining;
    let mut storage_units_remaining = budget.storage_payload_units_remaining;
    let mut selected_count = 0_u32;
    let mut records = Vec::with_capacity(scored.len());

    for pair in scored {
        let rejection_reason = rejection_reason(
            pair.fragment,
            budget,
            payload_remaining,
            storage_units_remaining,
            selected_count,
            state.r_est_permille,
        );
        if rejection_reason.is_none() {
            payload_remaining = payload_remaining.saturating_sub(pair.fragment.payload_bytes);
            storage_units_remaining = storage_units_remaining.saturating_sub(1);
            selected_count = selected_count.saturating_add(1);
        }
        records.push(decision_record(pair, rejection_reason));
    }
    records
}

fn score_pairs(
    state: &LocalPolicyState,
    peer_candidates: &[LocalPolicyPeerCandidate],
    fragment_candidates: &[LocalPolicyFragmentCandidate],
) -> Vec<ScoredPair> {
    let mut scored = Vec::with_capacity(
        peer_candidates
            .len()
            .saturating_mul(fragment_candidates.len()),
    );
    for peer in peer_candidates {
        for fragment in fragment_candidates {
            scored.push(ScoredPair {
                peer: *peer,
                fragment: *fragment,
                score: score_pair(state, *peer, *fragment),
            });
        }
    }
    scored
}

fn score_pair(
    state: &LocalPolicyState,
    peer: LocalPolicyPeerCandidate,
    fragment: LocalPolicyFragmentCandidate,
) -> LocalPolicyScoreBreakdown {
    let bridge_value = state
        .peers
        .get(&peer.peer_node_id)
        .map(|peer_state| peer_state.bridge_score_permille)
        .unwrap_or(0);
    local_policy_score_from_input(LocalPolicyScoreInput {
        expected_innovation_gain: fragment.expected_innovation_gain,
        bridge_value,
        landscape_value: fragment.landscape_value,
        demand_value: fragment.demand_value,
        duplicate_risk: state
            .recent_duplicate_rate_permille
            .saturating_add(fragment.duplicate_risk_hint)
            .min(1_000),
        payload_byte_cost: fragment.payload_bytes.min(1_000),
        storage_pressure_cost: state.storage_pressure_permille,
        reproduction_pressure_penalty: state.r_est_permille.saturating_sub(1_000).min(1_000),
    })
}

fn rejection_reason(
    fragment: LocalPolicyFragmentCandidate,
    budget: LocalPolicyReducerBudget,
    payload_remaining: u32,
    storage_units_remaining: u32,
    selected_count: u32,
    r_est_permille: u32,
) -> Option<LocalPolicyRejectionReason> {
    if r_est_permille > budget.reproduction_target_max_permille {
        return Some(LocalPolicyRejectionReason::ReproductionBudget);
    }
    if selected_count >= budget.max_forwarding_decisions {
        return Some(LocalPolicyRejectionReason::ForwardingDecisionLimit);
    }
    if fragment.payload_bytes > payload_remaining {
        return Some(LocalPolicyRejectionReason::PayloadByteBudget);
    }
    if storage_units_remaining == 0 {
        return Some(LocalPolicyRejectionReason::StorageBudget);
    }
    None
}

fn decision_record(
    pair: ScoredPair,
    rejection_reason: Option<LocalPolicyRejectionReason>,
) -> LocalPolicyDecisionRecord {
    LocalPolicyDecisionRecord {
        policy_id: "local-evidence-policy".to_string(),
        peer_node_id: pair.peer.peer_node_id,
        fragment_id: pair.fragment.fragment_id,
        selected: rejection_reason.is_none(),
        rejection_reason,
        total_score: pair.score.total_score,
        score: pair.score,
    }
}

fn compare_pairs(left: &ScoredPair, right: &ScoredPair) -> std::cmp::Ordering {
    let left_candidate = LocalPolicyScoreCandidate {
        peer_node_id: left.peer.peer_node_id,
        fragment_id: left.fragment.fragment_id,
        score: left.score,
    };
    let right_candidate = LocalPolicyScoreCandidate {
        peer_node_id: right.peer.peer_node_id,
        fragment_id: right.fragment.fragment_id,
        score: right.score,
    };
    compare_scored_candidates(&right_candidate, &left_candidate)
}

#[cfg(test)]
mod tests {
    use super::{
        reduce_local_policy_forwarding, LocalPolicyFragmentCandidate, LocalPolicyPeerCandidate,
        LocalPolicyReducerBudget, LocalPolicyRejectionReason,
    };
    use crate::diffusion::local_policy::{
        local_policy_state_from_trace, LocalPolicyArrivalKind, LocalPolicyState,
        LocalPolicyStateTraceEvent,
    };

    fn state() -> LocalPolicyState {
        let trace = vec![
            LocalPolicyStateTraceEvent::Contact {
                round_index: 0,
                peer_node_id: 9,
                peer_cluster_id: 2,
                bridge_contact: true,
            },
            LocalPolicyStateTraceEvent::Contact {
                round_index: 1,
                peer_node_id: 4,
                peer_cluster_id: 1,
                bridge_contact: false,
            },
            LocalPolicyStateTraceEvent::Arrival {
                arrival_kind: LocalPolicyArrivalKind::Innovative,
            },
            LocalPolicyStateTraceEvent::Storage {
                retained_payload_bytes: 64,
                storage_capacity_bytes: 512,
            },
            LocalPolicyStateTraceEvent::Reproduction {
                active_forwarding_opportunities: 4,
                innovative_successor_opportunities: 3,
            },
        ];
        local_policy_state_from_trace(7, 512, &trace).expect("state")
    }

    fn peers() -> Vec<LocalPolicyPeerCandidate> {
        vec![
            LocalPolicyPeerCandidate { peer_node_id: 4 },
            LocalPolicyPeerCandidate { peer_node_id: 9 },
        ]
    }

    fn fragments() -> Vec<LocalPolicyFragmentCandidate> {
        vec![
            LocalPolicyFragmentCandidate {
                fragment_id: 1,
                payload_bytes: 32,
                expected_innovation_gain: 500,
                landscape_value: 100,
                demand_value: 0,
                duplicate_risk_hint: 0,
            },
            LocalPolicyFragmentCandidate {
                fragment_id: 2,
                payload_bytes: 32,
                expected_innovation_gain: 100,
                landscape_value: 0,
                demand_value: 0,
                duplicate_risk_hint: 200,
            },
        ]
    }

    fn budget() -> LocalPolicyReducerBudget {
        LocalPolicyReducerBudget {
            payload_byte_budget_remaining: 96,
            storage_payload_units_remaining: 2,
            reproduction_target_max_permille: 1_000,
            max_forwarding_decisions: 2,
        }
    }

    #[test]
    fn local_policy_reducer_selects_best_score_first() {
        let records = reduce_local_policy_forwarding(&state(), &peers(), &fragments(), budget());

        assert_eq!(records[0].peer_node_id, 9);
        assert_eq!(records[0].fragment_id, 1);
        assert!(records[0].selected);
        assert!(records[0].score.bridge_value > 0);
    }

    #[test]
    fn local_policy_reducer_records_payload_budget_exhaustion() {
        let mut budget = budget();
        budget.payload_byte_budget_remaining = 16;

        let records = reduce_local_policy_forwarding(&state(), &peers(), &fragments(), budget);

        assert!(records.iter().all(|record| !record.selected));
        assert!(records.iter().all(|record| {
            record.rejection_reason == Some(LocalPolicyRejectionReason::PayloadByteBudget)
        }));
    }

    #[test]
    fn local_policy_reducer_records_storage_pressure_rejection() {
        let mut budget = budget();
        budget.storage_payload_units_remaining = 0;

        let records = reduce_local_policy_forwarding(&state(), &peers(), &fragments(), budget);

        assert!(records.iter().all(|record| {
            record.rejection_reason == Some(LocalPolicyRejectionReason::StorageBudget)
        }));
    }

    #[test]
    fn local_policy_reducer_duplicate_heavy_input_lowers_score() {
        let mut duplicate_state = state();
        for _ in 0..4 {
            duplicate_state.record_arrival(LocalPolicyArrivalKind::Duplicate);
        }

        let clean = reduce_local_policy_forwarding(&state(), &peers(), &fragments(), budget());
        let duplicate =
            reduce_local_policy_forwarding(&duplicate_state, &peers(), &fragments(), budget());

        assert!(duplicate[0].total_score < clean[0].total_score);
    }

    #[test]
    fn local_policy_reducer_equal_score_tie_breaks_are_deterministic() {
        let peer_candidates = vec![
            LocalPolicyPeerCandidate { peer_node_id: 12 },
            LocalPolicyPeerCandidate { peer_node_id: 11 },
        ];
        let fragment_candidates = vec![
            LocalPolicyFragmentCandidate {
                fragment_id: 2,
                payload_bytes: 1,
                expected_innovation_gain: 100,
                landscape_value: 0,
                demand_value: 0,
                duplicate_risk_hint: 0,
            },
            LocalPolicyFragmentCandidate {
                fragment_id: 1,
                payload_bytes: 1,
                expected_innovation_gain: 100,
                landscape_value: 0,
                demand_value: 0,
                duplicate_risk_hint: 0,
            },
        ];

        let records = reduce_local_policy_forwarding(
            &state(),
            &peer_candidates,
            &fragment_candidates,
            budget(),
        );

        assert_eq!(records[0].peer_node_id, 11);
        assert_eq!(records[0].fragment_id, 1);
    }

    #[test]
    fn local_policy_reducer_replay_is_deterministic() {
        let first = reduce_local_policy_forwarding(&state(), &peers(), &fragments(), budget());
        let second = reduce_local_policy_forwarding(&state(), &peers(), &fragments(), budget());

        assert_eq!(first, second);
        assert!(first.iter().any(|record| record.selected));
        assert!(first
            .iter()
            .all(|record| record.score.total_score == record.total_score));
    }

    #[test]
    fn local_policy_reducer_enforces_reproduction_control() {
        let mut saturated = state();
        saturated.update_reproduction_estimate(1, 2);
        let mut budget = budget();
        budget.reproduction_target_max_permille = 800;

        let records = reduce_local_policy_forwarding(&saturated, &peers(), &fragments(), budget);

        assert!(records.iter().all(|record| {
            record.rejection_reason == Some(LocalPolicyRejectionReason::ReproductionBudget)
        }));
    }
}
// proc-macro-scope: local-policy reducer rows are artifact schema, not shared model vocabulary.
