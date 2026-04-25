//! Deterministic observer-ambiguity metric proxies.

use serde::{Deserialize, Serialize};

use super::{
    ObserverAttackerResult, ObserverEventKind, ObserverProjectionKind, ObserverTraceEvent,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverCostPoint {
    pub cost_bytes: u32,
    pub ambiguity_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverAmbiguityMetrics {
    pub projection_kind: ObserverProjectionKind,
    pub attacker_top1_accuracy_permille: u32,
    pub posterior_uncertainty_permille: u32,
    pub hidden_projection_proxy_label: String,
    pub hidden_projection_proxy_permille: u32,
    pub forwarding_contact_proxy_label: String,
    pub forwarding_contact_proxy_permille: u32,
    pub ambiguity_cost_frontier_area: u64,
}

pub(crate) fn observer_metrics_from_result(
    result: &ObserverAttackerResult,
    trace: &[ObserverTraceEvent],
    cost_bytes: u32,
) -> ObserverAmbiguityMetrics {
    let ambiguity = result.posterior_uncertainty_permille.min(1_000);
    ObserverAmbiguityMetrics {
        projection_kind: result.projection_kind,
        attacker_top1_accuracy_permille: top1_accuracy_permille(result),
        posterior_uncertainty_permille: ambiguity,
        hidden_projection_proxy_label: "mutual-information-style-hidden-trace-proxy".to_string(),
        hidden_projection_proxy_permille: 1_000_u32.saturating_sub(ambiguity),
        forwarding_contact_proxy_label: "mutual-information-style-forwarding-contact-proxy"
            .to_string(),
        forwarding_contact_proxy_permille: forwarding_contact_proxy_permille(trace),
        ambiguity_cost_frontier_area: ambiguity_cost_frontier_area(&[ObserverCostPoint {
            cost_bytes,
            ambiguity_permille: ambiguity,
        }]),
    }
}

pub(crate) fn ambiguity_cost_frontier_area(points: &[ObserverCostPoint]) -> u64 {
    if points.is_empty() {
        return 0;
    }
    let mut sorted = points.to_vec();
    sorted.sort_by_key(|point| (point.cost_bytes, point.ambiguity_permille));
    if sorted.len() == 1 {
        let point = sorted[0];
        return u64::from(point.cost_bytes).saturating_mul(u64::from(point.ambiguity_permille));
    }
    sorted
        .windows(2)
        .map(frontier_segment_area)
        .fold(0_u64, u64::saturating_add)
}

fn top1_accuracy_permille(result: &ObserverAttackerResult) -> u32 {
    if result.top_guess_cluster_id == result.hidden_cluster_id {
        1_000
    } else {
        0
    }
}

fn forwarding_contact_proxy_permille(trace: &[ObserverTraceEvent]) -> u32 {
    let contact_count = count_kind(trace, ObserverEventKind::Contact);
    let forwarding_count = count_kind(trace, ObserverEventKind::Forwarding);
    if contact_count == 0 {
        return 0;
    }
    forwarding_count
        .saturating_mul(1_000)
        .saturating_div(contact_count)
        .min(1_000)
}

fn count_kind(trace: &[ObserverTraceEvent], event_kind: ObserverEventKind) -> u32 {
    u32::try_from(
        trace
            .iter()
            .filter(|row| row.event_kind == event_kind)
            .count(),
    )
    .unwrap_or(u32::MAX)
}

fn frontier_segment_area(points: &[ObserverCostPoint]) -> u64 {
    debug_assert!(points.len() == 2);
    let left = points[0];
    let right = points[1];
    let width = right.cost_bytes.saturating_sub(left.cost_bytes);
    let height_sum = left
        .ambiguity_permille
        .saturating_add(right.ambiguity_permille);
    u64::from(width).saturating_mul(u64::from(height_sum)) / 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diffusion::{
        catalog::scenarios::build_coded_inference_readiness_scenario,
        coded_inference::build_coded_inference_readiness_log,
        observer::{
            project_observer_trace, run_observer_attacker, ObserverAttackerConfig,
            ObserverAttackerHypothesisScore, ObserverAttackerTarget, ObserverProjectionConfig,
        },
    };

    fn fixture() -> (ObserverAttackerResult, Vec<ObserverTraceEvent>) {
        let scenario = build_coded_inference_readiness_scenario();
        let log = build_coded_inference_readiness_log(41, &scenario);
        let trace = project_observer_trace(&log, &ObserverProjectionConfig::global());
        let result = run_observer_attacker(
            &ObserverAttackerConfig::anomaly_region("local-evidence-policy"),
            &trace,
            scenario.coded_inference.hidden_anomaly_cluster_id,
            scenario.coded_inference.cluster_count,
        );
        (result, trace)
    }

    #[test]
    fn observer_metrics_perfect_attacker_scores_top1_accuracy() {
        let (mut result, trace) = fixture();
        result.top_guess_cluster_id = result.hidden_cluster_id;
        let metrics = observer_metrics_from_result(&result, &trace, 128);

        assert_eq!(metrics.attacker_top1_accuracy_permille, 1_000);
        assert!(metrics.hidden_projection_proxy_label.contains("proxy"));
    }

    #[test]
    fn observer_metrics_blind_attacker_stays_bounded() {
        let scenario = build_coded_inference_readiness_scenario();
        let log = build_coded_inference_readiness_log(41, &scenario);
        let trace = project_observer_trace(&log, &ObserverProjectionConfig::blind());
        let result = run_observer_attacker(
            &ObserverAttackerConfig::anomaly_region("local-evidence-policy"),
            &trace,
            scenario.coded_inference.hidden_anomaly_cluster_id,
            scenario.coded_inference.cluster_count,
        );
        let metrics = observer_metrics_from_result(&result, &trace, 128);

        assert!(metrics.posterior_uncertainty_permille <= 1_000);
        assert!(metrics.forwarding_contact_proxy_permille <= 1_000);
    }

    #[test]
    fn observer_metrics_uniform_posterior_reports_full_uncertainty() {
        let result = ObserverAttackerResult {
            projection_kind: ObserverProjectionKind::Blind,
            target: ObserverAttackerTarget::AnomalyRegion,
            policy_family_id: "fixture".to_string(),
            hidden_cluster_id: 1,
            top_guess_cluster_id: 0,
            true_target_rank: 2,
            top_score: 0,
            posterior_uncertainty_permille: 1_000,
            candidate_scores: vec![
                ObserverAttackerHypothesisScore {
                    cluster_id: 0,
                    score: 0,
                },
                ObserverAttackerHypothesisScore {
                    cluster_id: 1,
                    score: 0,
                },
            ],
        };
        let metrics = observer_metrics_from_result(&result, &[], 64);

        assert_eq!(metrics.posterior_uncertainty_permille, 1_000);
        assert_eq!(metrics.hidden_projection_proxy_permille, 0);
    }

    #[test]
    fn observer_metrics_frontier_area_is_deterministic() {
        let points = [
            ObserverCostPoint {
                cost_bytes: 64,
                ambiguity_permille: 900,
            },
            ObserverCostPoint {
                cost_bytes: 32,
                ambiguity_permille: 1_000,
            },
            ObserverCostPoint {
                cost_bytes: 128,
                ambiguity_permille: 700,
            },
        ];

        assert_eq!(ambiguity_cost_frontier_area(&points), 81_600);
        assert_eq!(ambiguity_cost_frontier_area(&points), 81_600);
    }
}
// proc-macro-scope: observer metric rows are replay schema, not shared model vocabulary.
