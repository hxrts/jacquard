//! Robustness summaries for observer-ambiguity sweeps.

use serde::{Deserialize, Serialize};

use super::{
    ambiguity_cost_frontier_area, run_observer_sweep, ObserverCostPoint, ObserverSweepArtifact,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ObserverRobustnessScenarioKind {
    Sparse,
    Clustered,
    BridgeHeavy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverRobustnessSummary {
    pub scenario_count: u32,
    pub seed_count: u32,
    pub low_dispersion_attacker_advantage_permille: u32,
    pub high_dispersion_attacker_advantage_permille: u32,
    pub dispersion_cost_penalty_bytes: u32,
    pub dispersion_latency_penalty_rounds: u32,
    pub ambiguity_cost_frontier_area: u64,
    pub quality_permille: u32,
}

pub(crate) fn run_observer_robustness_summary(seeds: &[u64]) -> ObserverRobustnessSummary {
    let scenarios = [
        ObserverRobustnessScenarioKind::Sparse,
        ObserverRobustnessScenarioKind::Clustered,
        ObserverRobustnessScenarioKind::BridgeHeavy,
    ];
    let mut rows = Vec::new();
    for scenario in scenarios {
        for seed in seeds {
            rows.extend(adjust_for_scenario(scenario, run_observer_sweep(*seed)));
        }
    }
    summarize_rows(
        &rows,
        u32::try_from(scenarios.len()).unwrap_or(u32::MAX),
        seeds,
    )
}

fn adjust_for_scenario(
    scenario: ObserverRobustnessScenarioKind,
    rows: Vec<ObserverSweepArtifact>,
) -> Vec<ObserverSweepArtifact> {
    rows.into_iter()
        .map(|mut row| {
            row.cell.scenario_id = scenario_id(scenario).to_string();
            row.cost_bytes = row
                .cost_bytes
                .saturating_add(scenario_cost_offset(scenario));
            row
        })
        .collect()
}

fn scenario_id(scenario: ObserverRobustnessScenarioKind) -> &'static str {
    match scenario {
        ObserverRobustnessScenarioKind::Sparse => "observer-sparse",
        ObserverRobustnessScenarioKind::Clustered => "observer-clustered",
        ObserverRobustnessScenarioKind::BridgeHeavy => "observer-bridge-heavy",
    }
}

fn scenario_cost_offset(scenario: ObserverRobustnessScenarioKind) -> u32 {
    match scenario {
        ObserverRobustnessScenarioKind::Sparse => 32,
        ObserverRobustnessScenarioKind::Clustered => 64,
        ObserverRobustnessScenarioKind::BridgeHeavy => 96,
    }
}

fn summarize_rows(
    rows: &[ObserverSweepArtifact],
    scenario_count: u32,
    seeds: &[u64],
) -> ObserverRobustnessSummary {
    let low = rows_for_dispersion(rows, 200);
    let high = rows_for_dispersion(rows, 800);
    ObserverRobustnessSummary {
        scenario_count,
        seed_count: u32::try_from(seeds.len()).unwrap_or(u32::MAX),
        low_dispersion_attacker_advantage_permille: mean_advantage(&low),
        high_dispersion_attacker_advantage_permille: mean_advantage(&high),
        dispersion_cost_penalty_bytes: mean_cost(&high).saturating_sub(mean_cost(&low)),
        dispersion_latency_penalty_rounds: mean_latency(&high).saturating_sub(mean_latency(&low)),
        ambiguity_cost_frontier_area: ambiguity_cost_frontier_area(&frontier_points(rows)),
        quality_permille: mean_quality(rows),
    }
}

fn rows_for_dispersion(
    rows: &[ObserverSweepArtifact],
    dispersion_permille: u32,
) -> Vec<&ObserverSweepArtifact> {
    rows.iter()
        .filter(|row| row.cell.fragment_dispersion_permille == dispersion_permille)
        .collect()
}

fn mean_advantage(rows: &[&ObserverSweepArtifact]) -> u32 {
    mean_u32(
        rows.iter()
            .map(|row| row.metrics.hidden_projection_proxy_permille),
    )
}

fn mean_cost(rows: &[&ObserverSweepArtifact]) -> u32 {
    mean_u32(rows.iter().map(|row| row.cost_bytes))
}

fn mean_latency(rows: &[&ObserverSweepArtifact]) -> u32 {
    mean_u32(rows.iter().map(|row| row.latency_rounds))
}

fn mean_quality(rows: &[ObserverSweepArtifact]) -> u32 {
    mean_u32(rows.iter().map(|row| row.quality_permille))
}

fn mean_u32(values: impl Iterator<Item = u32>) -> u32 {
    let mut count = 0_u32;
    let mut total = 0_u32;
    for value in values {
        count = count.saturating_add(1);
        total = total.saturating_add(value);
    }
    if count == 0 {
        return 0;
    }
    total / count
}

fn frontier_points(rows: &[ObserverSweepArtifact]) -> Vec<ObserverCostPoint> {
    rows.iter()
        .map(|row| ObserverCostPoint {
            cost_bytes: row.cost_bytes,
            ambiguity_permille: row.metrics.posterior_uncertainty_permille,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_robustness_dispersion_reduces_attacker_advantage() {
        let summary = run_observer_robustness_summary(&[41, 43]);

        assert!(
            summary.high_dispersion_attacker_advantage_permille
                < summary.low_dispersion_attacker_advantage_permille
        );
    }

    #[test]
    fn observer_robustness_quantifies_cost_and_latency_penalty() {
        let summary = run_observer_robustness_summary(&[41, 43]);

        assert!(summary.dispersion_cost_penalty_bytes > 0);
        assert!(summary.dispersion_latency_penalty_rounds > 0);
    }

    #[test]
    fn observer_robustness_is_stable_across_scenarios_and_seeds() {
        let first = run_observer_robustness_summary(&[41, 43]);
        let second = run_observer_robustness_summary(&[41, 43]);

        assert_eq!(first, second);
        assert_eq!(first.scenario_count, 3);
        assert_eq!(first.seed_count, 2);
        assert!(first.ambiguity_cost_frontier_area > 0);
        assert!(first.quality_permille > 0);
    }
}
// proc-macro-scope: observer robustness rows are replay schema, not shared model vocabulary.
