//! Ablation variants for the simulator-local evidence policy.

use serde::{Deserialize, Serialize};

use super::{
    reduce_local_policy_forwarding, LocalPolicyDecisionRecord, LocalPolicyFragmentCandidate,
    LocalPolicyPeerCandidate, LocalPolicyReducerBudget, LocalPolicyScoreBreakdown,
    LocalPolicyState,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum LocalPolicyAblationVariant {
    FullPolicy,
    NoBridgeScore,
    NoDuplicateRisk,
    NoLandscapeValue,
    NoReproductionControl,
    DeterministicRandomForwarding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyAblationDecisionRecord {
    pub variant: LocalPolicyAblationVariant,
    pub disabled_terms: Vec<String>,
    pub decision: LocalPolicyDecisionRecord,
}

pub(crate) fn run_local_policy_ablation(
    variant: LocalPolicyAblationVariant,
    seed: u64,
    state: &LocalPolicyState,
    peer_candidates: &[LocalPolicyPeerCandidate],
    fragment_candidates: &[LocalPolicyFragmentCandidate],
    budget: LocalPolicyReducerBudget,
) -> Vec<LocalPolicyAblationDecisionRecord> {
    if variant == LocalPolicyAblationVariant::DeterministicRandomForwarding {
        return random_forwarding(seed, variant, peer_candidates, fragment_candidates, budget);
    }
    let mut ablated_state = state.clone();
    let mut ablated_fragments = fragment_candidates.to_vec();
    let mut ablated_budget = budget;
    apply_variant(
        variant,
        &mut ablated_state,
        &mut ablated_fragments,
        &mut ablated_budget,
    );
    reduce_local_policy_forwarding(
        &ablated_state,
        peer_candidates,
        &ablated_fragments,
        ablated_budget,
    )
    .into_iter()
    .map(|mut decision| {
        decision.policy_id = variant.policy_id().to_string();
        LocalPolicyAblationDecisionRecord {
            variant,
            disabled_terms: variant.disabled_terms(),
            decision,
        }
    })
    .collect()
}

impl LocalPolicyAblationVariant {
    fn policy_id(self) -> &'static str {
        match self {
            Self::FullPolicy => "local-evidence-policy",
            Self::NoBridgeScore => "local-evidence-policy-no-bridge",
            Self::NoDuplicateRisk => "local-evidence-policy-no-duplicate-risk",
            Self::NoLandscapeValue => "local-evidence-policy-no-landscape",
            Self::NoReproductionControl => "local-evidence-policy-no-reproduction-control",
            Self::DeterministicRandomForwarding => "deterministic-random-forwarding",
        }
    }

    fn disabled_terms(self) -> Vec<String> {
        match self {
            Self::FullPolicy => Vec::new(),
            Self::NoBridgeScore => vec!["bridge_value".to_string()],
            Self::NoDuplicateRisk => vec!["duplicate_risk".to_string()],
            Self::NoLandscapeValue => vec!["landscape_value".to_string()],
            Self::NoReproductionControl => {
                vec!["reproduction_pressure_penalty".to_string()]
            }
            Self::DeterministicRandomForwarding => vec![
                "expected_innovation_gain".to_string(),
                "bridge_value".to_string(),
                "landscape_value".to_string(),
                "duplicate_risk".to_string(),
                "byte_cost".to_string(),
                "storage_pressure_cost".to_string(),
                "reproduction_pressure_penalty".to_string(),
            ],
        }
    }
}

fn apply_variant(
    variant: LocalPolicyAblationVariant,
    state: &mut LocalPolicyState,
    fragments: &mut [LocalPolicyFragmentCandidate],
    budget: &mut LocalPolicyReducerBudget,
) {
    match variant {
        LocalPolicyAblationVariant::FullPolicy => {}
        LocalPolicyAblationVariant::NoBridgeScore => {
            for peer in state.peers.values_mut() {
                peer.bridge_score_permille = 0;
            }
        }
        LocalPolicyAblationVariant::NoDuplicateRisk => {
            state.recent_duplicate_rate_permille = 0;
            for fragment in fragments {
                fragment.duplicate_risk_hint = 0;
            }
        }
        LocalPolicyAblationVariant::NoLandscapeValue => {
            for fragment in fragments {
                fragment.landscape_value = 0;
            }
        }
        LocalPolicyAblationVariant::NoReproductionControl => {
            budget.reproduction_target_max_permille = 1_000;
            state.r_est_permille = 0;
        }
        LocalPolicyAblationVariant::DeterministicRandomForwarding => {}
    }
}

fn random_forwarding(
    seed: u64,
    variant: LocalPolicyAblationVariant,
    peer_candidates: &[LocalPolicyPeerCandidate],
    fragment_candidates: &[LocalPolicyFragmentCandidate],
    budget: LocalPolicyReducerBudget,
) -> Vec<LocalPolicyAblationDecisionRecord> {
    let mut pairs = random_pairs(seed, peer_candidates, fragment_candidates);
    pairs.sort_by_key(|pair| pair.0);
    let mut payload_remaining = budget.payload_byte_budget_remaining;
    let mut storage_remaining = budget.storage_payload_units_remaining;
    let mut selected_count = 0_u32;
    pairs
        .into_iter()
        .map(|(_key, peer, fragment)| {
            let selected = fragment.payload_bytes <= payload_remaining
                && storage_remaining > 0
                && selected_count < budget.max_forwarding_decisions;
            if selected {
                payload_remaining = payload_remaining.saturating_sub(fragment.payload_bytes);
                storage_remaining = storage_remaining.saturating_sub(1);
                selected_count = selected_count.saturating_add(1);
            }
            random_record(variant, peer, fragment, selected)
        })
        .collect()
}

fn random_pairs(
    seed: u64,
    peer_candidates: &[LocalPolicyPeerCandidate],
    fragment_candidates: &[LocalPolicyFragmentCandidate],
) -> Vec<(u64, LocalPolicyPeerCandidate, LocalPolicyFragmentCandidate)> {
    let mut pairs = Vec::with_capacity(
        peer_candidates
            .len()
            .saturating_mul(fragment_candidates.len()),
    );
    for peer in peer_candidates {
        for fragment in fragment_candidates {
            pairs.push((
                stable_random_key(seed, peer.peer_node_id, fragment.fragment_id),
                *peer,
                *fragment,
            ));
        }
    }
    pairs
}

fn random_record(
    variant: LocalPolicyAblationVariant,
    peer: LocalPolicyPeerCandidate,
    fragment: LocalPolicyFragmentCandidate,
    selected: bool,
) -> LocalPolicyAblationDecisionRecord {
    LocalPolicyAblationDecisionRecord {
        variant,
        disabled_terms: variant.disabled_terms(),
        decision: LocalPolicyDecisionRecord {
            policy_id: variant.policy_id().to_string(),
            peer_node_id: peer.peer_node_id,
            fragment_id: fragment.fragment_id,
            selected,
            rejection_reason: None,
            total_score: 0,
            score: LocalPolicyScoreBreakdown::from_terms(0, 0, 0, 0, 0, 0, 0),
        },
    }
}

fn stable_random_key(seed: u64, peer_node_id: u32, fragment_id: u32) -> u64 {
    let mut value = seed ^ u64::from(peer_node_id).rotate_left(17);
    value ^= u64::from(fragment_id).rotate_left(41);
    value = value.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^ (value >> 33)
}

#[cfg(test)]
mod tests {
    use super::{run_local_policy_ablation, LocalPolicyAblationVariant};
    use crate::diffusion::local_policy::{
        local_policy_state_from_trace, LocalPolicyArrivalKind, LocalPolicyFragmentCandidate,
        LocalPolicyPeerCandidate, LocalPolicyReducerBudget, LocalPolicyState,
        LocalPolicyStateTraceEvent,
    };

    fn state() -> LocalPolicyState {
        local_policy_state_from_trace(
            7,
            512,
            &[
                LocalPolicyStateTraceEvent::Contact {
                    round_index: 0,
                    peer_node_id: 9,
                    peer_cluster_id: 2,
                    bridge_contact: true,
                },
                LocalPolicyStateTraceEvent::Arrival {
                    arrival_kind: LocalPolicyArrivalKind::Duplicate,
                },
                LocalPolicyStateTraceEvent::Reproduction {
                    active_forwarding_opportunities: 1,
                    innovative_successor_opportunities: 1,
                },
            ],
        )
        .expect("state")
    }

    fn peers() -> Vec<LocalPolicyPeerCandidate> {
        vec![LocalPolicyPeerCandidate { peer_node_id: 9 }]
    }

    fn fragments() -> Vec<LocalPolicyFragmentCandidate> {
        vec![LocalPolicyFragmentCandidate {
            fragment_id: 3,
            payload_bytes: 32,
            expected_innovation_gain: 500,
            landscape_value: 300,
            duplicate_risk_hint: 200,
        }]
    }

    fn budget() -> LocalPolicyReducerBudget {
        LocalPolicyReducerBudget {
            payload_byte_budget_remaining: 64,
            storage_payload_units_remaining: 2,
            reproduction_target_max_permille: 800,
            max_forwarding_decisions: 1,
        }
    }

    #[test]
    fn local_policy_ablation_disables_only_bridge_score() {
        let full = run_local_policy_ablation(
            LocalPolicyAblationVariant::FullPolicy,
            41,
            &state(),
            &peers(),
            &fragments(),
            budget(),
        );
        let no_bridge = run_local_policy_ablation(
            LocalPolicyAblationVariant::NoBridgeScore,
            41,
            &state(),
            &peers(),
            &fragments(),
            budget(),
        );

        assert!(no_bridge[0]
            .disabled_terms
            .contains(&"bridge_value".to_string()));
        assert_eq!(no_bridge[0].decision.score.bridge_value, 0);
        assert!(full[0].decision.score.bridge_value > 0);
    }

    #[test]
    fn local_policy_ablation_disables_duplicate_risk_term() {
        let no_duplicate = run_local_policy_ablation(
            LocalPolicyAblationVariant::NoDuplicateRisk,
            41,
            &state(),
            &peers(),
            &fragments(),
            budget(),
        );

        assert_eq!(no_duplicate[0].decision.score.duplicate_risk, 0);
        assert!(no_duplicate[0]
            .disabled_terms
            .contains(&"duplicate_risk".to_string()));
    }

    #[test]
    fn local_policy_ablation_disables_landscape_value_term() {
        let no_landscape = run_local_policy_ablation(
            LocalPolicyAblationVariant::NoLandscapeValue,
            41,
            &state(),
            &peers(),
            &fragments(),
            budget(),
        );

        assert_eq!(no_landscape[0].decision.score.landscape_value, 0);
        assert!(no_landscape[0]
            .disabled_terms
            .contains(&"landscape_value".to_string()));
    }

    #[test]
    fn local_policy_ablation_disables_reproduction_control_path() {
        let full = run_local_policy_ablation(
            LocalPolicyAblationVariant::FullPolicy,
            41,
            &state(),
            &peers(),
            &fragments(),
            budget(),
        );
        let no_reproduction = run_local_policy_ablation(
            LocalPolicyAblationVariant::NoReproductionControl,
            41,
            &state(),
            &peers(),
            &fragments(),
            budget(),
        );

        assert!(!full[0].decision.selected);
        assert!(no_reproduction[0].decision.selected);
        assert!(no_reproduction[0]
            .disabled_terms
            .contains(&"reproduction_pressure_penalty".to_string()));
    }

    #[test]
    fn local_policy_ablation_random_forwarding_is_deterministic_and_same_budget() {
        let first = run_local_policy_ablation(
            LocalPolicyAblationVariant::DeterministicRandomForwarding,
            41,
            &state(),
            &peers(),
            &fragments(),
            budget(),
        );
        let second = run_local_policy_ablation(
            LocalPolicyAblationVariant::DeterministicRandomForwarding,
            41,
            &state(),
            &peers(),
            &fragments(),
            budget(),
        );

        assert_eq!(first, second);
        assert!(first[0].decision.selected);
        assert_eq!(
            first[0].decision.policy_id,
            "deterministic-random-forwarding"
        );
        assert_eq!(first[0].decision.score.total_score, 0);
    }
}
