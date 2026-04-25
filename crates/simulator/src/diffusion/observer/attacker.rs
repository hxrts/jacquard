//! Deterministic first attacker over observer projections.

use serde::{Deserialize, Serialize};

use super::projection::{ObserverEventKind, ObserverProjectionKind, ObserverTraceEvent};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ObserverAttackerTarget {
    AnomalyRegion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverAttackerConfig {
    pub target: ObserverAttackerTarget,
    pub policy_family_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverAttackerHypothesisScore {
    pub cluster_id: u8,
    pub score: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverAttackerResult {
    pub projection_kind: ObserverProjectionKind,
    pub target: ObserverAttackerTarget,
    pub policy_family_id: String,
    pub hidden_cluster_id: u8,
    pub top_guess_cluster_id: u8,
    pub true_target_rank: u32,
    pub top_score: u32,
    pub posterior_uncertainty_permille: u32,
    pub candidate_scores: Vec<ObserverAttackerHypothesisScore>,
}

impl ObserverAttackerConfig {
    #[must_use]
    pub(crate) fn anomaly_region(policy_family_id: &str) -> Self {
        Self {
            target: ObserverAttackerTarget::AnomalyRegion,
            policy_family_id: policy_family_id.to_string(),
        }
    }
}

pub(crate) fn run_observer_attacker(
    config: &ObserverAttackerConfig,
    trace: &[ObserverTraceEvent],
    hidden_cluster_id: u8,
    candidate_cluster_count: u8,
) -> ObserverAttackerResult {
    let projection_kind = trace
        .first()
        .map(|row| row.projection_kind)
        .unwrap_or(ObserverProjectionKind::Blind);
    let mut scores = initial_scores(candidate_cluster_count);
    for row in trace {
        score_row(row, &mut scores);
    }
    scores.sort_by(compare_hypothesis_scores);
    attacker_result(config, projection_kind, hidden_cluster_id, scores)
}

fn initial_scores(candidate_cluster_count: u8) -> Vec<ObserverAttackerHypothesisScore> {
    (0..candidate_cluster_count)
        .map(|cluster_id| ObserverAttackerHypothesisScore {
            cluster_id,
            score: 0,
        })
        .collect()
}

fn score_row(row: &ObserverTraceEvent, scores: &mut [ObserverAttackerHypothesisScore]) {
    let weight = match row.event_kind {
        ObserverEventKind::Contact => 1,
        ObserverEventKind::Forwarding => 4_u32.saturating_add(row.byte_count.unwrap_or(0) / 32),
    };
    add_cluster_score(scores, row.cluster_a, weight);
    add_cluster_score(scores, row.cluster_b, weight);
}

fn add_cluster_score(
    scores: &mut [ObserverAttackerHypothesisScore],
    cluster_id: Option<u8>,
    weight: u32,
) {
    let Some(cluster_id) = cluster_id else {
        return;
    };
    if let Some(score) = scores
        .iter_mut()
        .find(|score| score.cluster_id == cluster_id)
    {
        score.score = score.score.saturating_add(weight);
    }
}

fn compare_hypothesis_scores(
    left: &ObserverAttackerHypothesisScore,
    right: &ObserverAttackerHypothesisScore,
) -> std::cmp::Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| left.cluster_id.cmp(&right.cluster_id))
}

fn attacker_result(
    config: &ObserverAttackerConfig,
    projection_kind: ObserverProjectionKind,
    hidden_cluster_id: u8,
    scores: Vec<ObserverAttackerHypothesisScore>,
) -> ObserverAttackerResult {
    let top = scores
        .first()
        .copied()
        .unwrap_or(ObserverAttackerHypothesisScore {
            cluster_id: 0,
            score: 0,
        });
    ObserverAttackerResult {
        projection_kind,
        target: config.target,
        policy_family_id: config.policy_family_id.clone(),
        hidden_cluster_id,
        top_guess_cluster_id: top.cluster_id,
        true_target_rank: true_target_rank(&scores, hidden_cluster_id),
        top_score: top.score,
        posterior_uncertainty_permille: posterior_uncertainty_permille(&scores),
        candidate_scores: scores,
    }
}

fn true_target_rank(scores: &[ObserverAttackerHypothesisScore], hidden_cluster_id: u8) -> u32 {
    scores
        .iter()
        .position(|score| score.cluster_id == hidden_cluster_id)
        .and_then(|index| u32::try_from(index.saturating_add(1)).ok())
        .unwrap_or(u32::MAX)
}

fn posterior_uncertainty_permille(scores: &[ObserverAttackerHypothesisScore]) -> u32 {
    let total = scores
        .iter()
        .map(|score| score.score)
        .fold(0_u32, u32::saturating_add);
    if total == 0 {
        return 1_000;
    }
    let top_score = scores.first().map(|score| score.score).unwrap_or(0);
    1_000_u32.saturating_sub(top_score.saturating_mul(1_000).saturating_div(total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diffusion::{
        catalog::scenarios::build_coded_inference_readiness_scenario,
        coded_inference::build_coded_inference_readiness_log,
        observer::{project_observer_trace, ObserverProjectionConfig},
    };

    fn attack_with(config: &ObserverProjectionConfig) -> ObserverAttackerResult {
        let scenario = build_coded_inference_readiness_scenario();
        let hidden = scenario.coded_inference.hidden_anomaly_cluster_id;
        let cluster_count = scenario.coded_inference.cluster_count;
        let log = build_coded_inference_readiness_log(41, &scenario);
        let trace = project_observer_trace(&log, config);
        run_observer_attacker(
            &ObserverAttackerConfig::anomaly_region("local-evidence-policy"),
            &trace,
            hidden,
            cluster_count,
        )
    }

    #[test]
    fn observer_attacker_ranks_anomaly_candidates_from_projection() {
        let result = attack_with(&ObserverProjectionConfig::global());

        assert_eq!(result.target, ObserverAttackerTarget::AnomalyRegion);
        assert_eq!(result.candidate_scores.len(), 5);
        assert!(result.true_target_rank >= 1);
        assert!(result.true_target_rank <= 5);
        assert!(result.top_score > 0);
    }

    #[test]
    fn observer_attacker_does_not_require_hidden_simulator_fields() {
        let result = attack_with(&ObserverProjectionConfig::blind());

        assert_eq!(result.projection_kind, ObserverProjectionKind::Blind);
        assert_eq!(result.candidate_scores.len(), 5);
        assert!(result.posterior_uncertainty_permille <= 1_000);
    }

    #[test]
    fn observer_attacker_outputs_are_replay_deterministic() {
        let first = attack_with(&ObserverProjectionConfig::regional(vec![1, 100]));
        let second = attack_with(&ObserverProjectionConfig::regional(vec![1, 100]));

        assert_eq!(first, second);
    }
}
// proc-macro-scope: observer attacker rows are replay schema, not shared model vocabulary.
