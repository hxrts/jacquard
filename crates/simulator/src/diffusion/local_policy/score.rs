//! Named integer score terms for simulator-local evidence forwarding policy.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

const TERM_MAX: i32 = 1_000;
const TERM_MIN: i32 = 0;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyScoreInput {
    pub expected_innovation_gain: u32,
    pub bridge_value: u32,
    pub landscape_value: u32,
    pub demand_value: u32,
    pub duplicate_risk: u32,
    pub payload_byte_cost: u32,
    pub storage_pressure_cost: u32,
    pub reproduction_pressure_penalty: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyScoreBreakdown {
    pub expected_innovation_gain: i32,
    pub bridge_value: i32,
    pub landscape_value: i32,
    pub demand_value: i32,
    pub duplicate_risk: i32,
    pub byte_cost: i32,
    pub storage_pressure_cost: i32,
    pub reproduction_pressure_penalty: i32,
    pub total_score: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyScoreCandidate {
    pub peer_node_id: u32,
    pub fragment_id: u32,
    pub score: LocalPolicyScoreBreakdown,
}

pub(crate) fn local_policy_score_from_input(
    input: LocalPolicyScoreInput,
) -> LocalPolicyScoreBreakdown {
    LocalPolicyScoreBreakdown::from_terms(
        bounded_term(input.expected_innovation_gain),
        bounded_term(input.bridge_value),
        bounded_term(input.landscape_value),
        bounded_term(input.demand_value),
        bounded_term(input.duplicate_risk),
        bounded_term(input.payload_byte_cost),
        bounded_term(input.storage_pressure_cost),
        bounded_term(input.reproduction_pressure_penalty),
    )
}

pub(crate) fn compare_scored_candidates(
    left: &LocalPolicyScoreCandidate,
    right: &LocalPolicyScoreCandidate,
) -> Ordering {
    left.score
        .total_score
        .cmp(&right.score.total_score)
        .then_with(|| right.score.duplicate_risk.cmp(&left.score.duplicate_risk))
        .then_with(|| right.score.byte_cost.cmp(&left.score.byte_cost))
        .then_with(|| right.peer_node_id.cmp(&left.peer_node_id))
        .then_with(|| right.fragment_id.cmp(&left.fragment_id))
}

impl LocalPolicyScoreBreakdown {
    pub(crate) fn from_terms(
        expected_innovation_gain: i32,
        bridge_value: i32,
        landscape_value: i32,
        demand_value: i32,
        duplicate_risk: i32,
        byte_cost: i32,
        storage_pressure_cost: i32,
        reproduction_pressure_penalty: i32,
    ) -> Self {
        let positive = expected_innovation_gain
            .saturating_add(bridge_value)
            .saturating_add(landscape_value)
            .saturating_add(demand_value);
        let negative = duplicate_risk
            .saturating_add(byte_cost)
            .saturating_add(storage_pressure_cost)
            .saturating_add(reproduction_pressure_penalty);
        Self {
            expected_innovation_gain,
            bridge_value,
            landscape_value,
            demand_value,
            duplicate_risk,
            byte_cost,
            storage_pressure_cost,
            reproduction_pressure_penalty,
            total_score: positive.saturating_sub(negative),
        }
    }
}

fn bounded_term(value: u32) -> i32 {
    i32::try_from(value.min(u32::try_from(TERM_MAX).unwrap_or(u32::MAX))).unwrap_or(TERM_MAX)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{
        compare_scored_candidates, local_policy_score_from_input, LocalPolicyScoreBreakdown,
        LocalPolicyScoreCandidate, LocalPolicyScoreInput, TERM_MAX, TERM_MIN,
    };

    fn base_input() -> LocalPolicyScoreInput {
        LocalPolicyScoreInput {
            expected_innovation_gain: 200,
            bridge_value: 100,
            landscape_value: 100,
            demand_value: 75,
            duplicate_risk: 50,
            payload_byte_cost: 25,
            storage_pressure_cost: 25,
            reproduction_pressure_penalty: 50,
        }
    }

    #[test]
    fn local_policy_score_formula_uses_named_terms_with_expected_signs() {
        let score = local_policy_score_from_input(base_input());

        assert_eq!(score.total_score, 325);
        assert_eq!(
            score.total_score,
            score
                .expected_innovation_gain
                .saturating_add(score.bridge_value)
                .saturating_add(score.landscape_value)
                .saturating_add(score.demand_value)
                .saturating_sub(score.duplicate_risk)
                .saturating_sub(score.byte_cost)
                .saturating_sub(score.storage_pressure_cost)
                .saturating_sub(score.reproduction_pressure_penalty)
        );
    }

    #[test]
    fn local_policy_score_positive_terms_increase_priority() {
        let base = local_policy_score_from_input(base_input()).total_score;
        let mut input = base_input();
        input.expected_innovation_gain = input.expected_innovation_gain.saturating_add(100);
        assert!(local_policy_score_from_input(input).total_score > base);

        input = base_input();
        input.bridge_value = input.bridge_value.saturating_add(100);
        assert!(local_policy_score_from_input(input).total_score > base);

        input = base_input();
        input.landscape_value = input.landscape_value.saturating_add(100);
        assert!(local_policy_score_from_input(input).total_score > base);

        input = base_input();
        input.demand_value = input.demand_value.saturating_add(100);
        assert!(local_policy_score_from_input(input).total_score > base);
    }

    #[test]
    fn local_policy_score_negative_terms_decrease_priority() {
        let base = local_policy_score_from_input(base_input()).total_score;
        let mut input = base_input();
        input.duplicate_risk = input.duplicate_risk.saturating_add(100);
        assert!(local_policy_score_from_input(input).total_score < base);

        input = base_input();
        input.payload_byte_cost = input.payload_byte_cost.saturating_add(100);
        assert!(local_policy_score_from_input(input).total_score < base);

        input = base_input();
        input.storage_pressure_cost = input.storage_pressure_cost.saturating_add(100);
        assert!(local_policy_score_from_input(input).total_score < base);

        input = base_input();
        input.reproduction_pressure_penalty =
            input.reproduction_pressure_penalty.saturating_add(100);
        assert!(local_policy_score_from_input(input).total_score < base);
    }

    #[test]
    fn local_policy_score_terms_are_bounded_and_saturating() {
        let score = local_policy_score_from_input(LocalPolicyScoreInput {
            expected_innovation_gain: u32::MAX,
            bridge_value: u32::MAX,
            landscape_value: u32::MAX,
            demand_value: u32::MAX,
            duplicate_risk: 0,
            payload_byte_cost: 0,
            storage_pressure_cost: 0,
            reproduction_pressure_penalty: 0,
        });

        assert_eq!(score.expected_innovation_gain, TERM_MAX);
        assert_eq!(score.bridge_value, TERM_MAX);
        assert_eq!(score.landscape_value, TERM_MAX);
        assert_eq!(score.demand_value, TERM_MAX);
        assert!(score.total_score >= TERM_MIN);
    }

    #[test]
    fn local_policy_score_serialization_preserves_each_named_term() {
        let score = local_policy_score_from_input(base_input());
        let serialized = serde_json::to_string(&score).expect("json");

        for field in [
            "expected_innovation_gain",
            "bridge_value",
            "landscape_value",
            "demand_value",
            "duplicate_risk",
            "byte_cost",
            "storage_pressure_cost",
            "reproduction_pressure_penalty",
            "total_score",
        ] {
            assert!(serialized.contains(field));
        }
    }

    #[test]
    fn local_policy_score_equal_total_tie_breaks_are_stable() {
        let low_peer = LocalPolicyScoreCandidate {
            peer_node_id: 7,
            fragment_id: 3,
            score: LocalPolicyScoreBreakdown::from_terms(100, 0, 0, 0, 0, 0, 0, 0),
        };
        let high_peer = LocalPolicyScoreCandidate {
            peer_node_id: 8,
            fragment_id: 1,
            score: LocalPolicyScoreBreakdown::from_terms(100, 0, 0, 0, 0, 0, 0, 0),
        };

        assert_eq!(
            compare_scored_candidates(&low_peer, &high_peer),
            Ordering::Greater
        );
    }
}
