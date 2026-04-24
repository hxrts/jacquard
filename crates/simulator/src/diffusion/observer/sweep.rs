//! Deterministic ambiguity knob sweep.

use serde::{Deserialize, Serialize};

use super::{
    observer_metrics_from_result, project_observer_trace, run_observer_attacker,
    ObserverAmbiguityMetrics, ObserverAttackerConfig, ObserverAttackerResult,
    ObserverAttackerTarget, ObserverProjectionConfig, ObserverProjectionKind,
};
use crate::diffusion::{
    catalog::scenarios::build_coded_inference_readiness_scenario,
    coded_inference::{build_coded_inference_readiness_log, CodedInferenceReadinessLog},
    model::CodedInferenceReadinessScenario,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ObserverForwardingRandomness {
    StableOrder,
    SeededPermutation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverSweepCell {
    pub seed: u64,
    pub scenario_id: String,
    pub projection_kind: ObserverProjectionKind,
    pub attacker_target: ObserverAttackerTarget,
    pub coding_rate_k: u32,
    pub coding_rate_n: u32,
    pub fragment_dispersion_permille: u32,
    pub forwarding_randomness: ObserverForwardingRandomness,
    pub path_diversity_preference_permille: u32,
    pub reproduction_target_low_permille: u32,
    pub reproduction_target_high_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverSweepArtifact {
    pub cell: ObserverSweepCell,
    pub attacker_result: ObserverAttackerResult,
    pub metrics: ObserverAmbiguityMetrics,
    pub cost_bytes: u32,
    pub latency_rounds: u32,
    pub quality_permille: u32,
}

pub(crate) fn run_observer_sweep(seed: u64) -> Vec<ObserverSweepArtifact> {
    let scenario = build_coded_inference_readiness_scenario();
    let log = build_coded_inference_readiness_log(seed, &scenario);
    observer_sweep_cells(seed)
        .into_iter()
        .map(|cell| run_cell(&scenario, &log, cell))
        .collect()
}

pub(crate) fn observer_sweep_cells(seed: u64) -> Vec<ObserverSweepCell> {
    let mut cells = Vec::new();
    for projection_kind in [
        ObserverProjectionKind::Global,
        ObserverProjectionKind::Blind,
    ] {
        for (coding_rate_k, coding_rate_n) in [(4, 12), (8, 12)] {
            for fragment_dispersion_permille in [200, 800] {
                for forwarding_randomness in [
                    ObserverForwardingRandomness::StableOrder,
                    ObserverForwardingRandomness::SeededPermutation,
                ] {
                    append_band_cells(
                        &mut cells,
                        seed,
                        projection_kind,
                        coding_rate_k,
                        coding_rate_n,
                        fragment_dispersion_permille,
                        forwarding_randomness,
                    );
                }
            }
        }
    }
    cells
}

fn append_band_cells(
    cells: &mut Vec<ObserverSweepCell>,
    seed: u64,
    projection_kind: ObserverProjectionKind,
    coding_rate_k: u32,
    coding_rate_n: u32,
    fragment_dispersion_permille: u32,
    forwarding_randomness: ObserverForwardingRandomness,
) {
    for path_diversity_preference_permille in [0, 500] {
        for (low, high) in [(800, 1_000), (900, 1_000)] {
            cells.push(ObserverSweepCell {
                seed,
                scenario_id: "coded-inference-observer".to_string(),
                projection_kind,
                attacker_target: ObserverAttackerTarget::AnomalyRegion,
                coding_rate_k,
                coding_rate_n,
                fragment_dispersion_permille,
                forwarding_randomness,
                path_diversity_preference_permille,
                reproduction_target_low_permille: low,
                reproduction_target_high_permille: high,
            });
        }
    }
}

fn run_cell(
    scenario: &CodedInferenceReadinessScenario,
    log: &CodedInferenceReadinessLog,
    cell: ObserverSweepCell,
) -> ObserverSweepArtifact {
    let trace = project_observer_trace(&log, &projection_config(cell.projection_kind));
    let attacker = run_observer_attacker(
        &ObserverAttackerConfig::anomaly_region("local-evidence-policy"),
        &trace,
        scenario.coded_inference.hidden_anomaly_cluster_id,
        scenario.coded_inference.cluster_count,
    );
    let attacker_result = adjust_attacker_for_knobs(&cell, attacker);
    let cost_bytes = cost_bytes_for(&cell);
    let metrics = observer_metrics_from_result(&attacker_result, &trace, cost_bytes);
    ObserverSweepArtifact {
        latency_rounds: latency_rounds_for(&cell),
        quality_permille: quality_permille_for(&cell),
        cell,
        attacker_result,
        metrics,
        cost_bytes,
    }
}

fn projection_config(projection_kind: ObserverProjectionKind) -> ObserverProjectionConfig {
    match projection_kind {
        ObserverProjectionKind::Global => ObserverProjectionConfig::global(),
        ObserverProjectionKind::Regional => ObserverProjectionConfig::regional(vec![100]),
        ObserverProjectionKind::Endpoint => ObserverProjectionConfig::endpoint(100),
        ObserverProjectionKind::Blind => ObserverProjectionConfig::blind(),
    }
}

fn adjust_attacker_for_knobs(
    cell: &ObserverSweepCell,
    mut attacker: ObserverAttackerResult,
) -> ObserverAttackerResult {
    let dispersion_uncertainty = cell.fragment_dispersion_permille / 4;
    let randomness_uncertainty = match cell.forwarding_randomness {
        ObserverForwardingRandomness::StableOrder => 0,
        ObserverForwardingRandomness::SeededPermutation => 50,
    };
    attacker.posterior_uncertainty_permille = attacker
        .posterior_uncertainty_permille
        .saturating_add(dispersion_uncertainty)
        .saturating_add(randomness_uncertainty)
        .min(1_000);
    attacker
}

fn cost_bytes_for(cell: &ObserverSweepCell) -> u32 {
    let base = cell.coding_rate_k.saturating_mul(32);
    base.saturating_add(cell.fragment_dispersion_permille / 2)
        .saturating_add(cell.path_diversity_preference_permille / 4)
}

fn latency_rounds_for(cell: &ObserverSweepCell) -> u32 {
    8_u32
        .saturating_add(cell.fragment_dispersion_permille / 200)
        .saturating_add(match cell.forwarding_randomness {
            ObserverForwardingRandomness::StableOrder => 0,
            ObserverForwardingRandomness::SeededPermutation => 1,
        })
}

fn quality_permille_for(cell: &ObserverSweepCell) -> u32 {
    950_u32
        .saturating_sub(cell.fragment_dispersion_permille / 20)
        .saturating_sub(cell.path_diversity_preference_permille / 50)
        .max(700)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_sweep_generates_complete_deterministic_cells() {
        let first = run_observer_sweep(41);
        let second = run_observer_sweep(41);

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn observer_sweep_cells_carry_knob_values() {
        let sweep = run_observer_sweep(41);

        assert!(sweep.iter().any(|row| row.cell.coding_rate_k == 4));
        assert!(sweep.iter().any(|row| row.cell.coding_rate_k == 8));
        assert!(sweep
            .iter()
            .any(|row| row.cell.fragment_dispersion_permille == 800));
        assert!(sweep.iter().any(|row| {
            row.cell.forwarding_randomness == ObserverForwardingRandomness::SeededPermutation
        }));
        assert!(sweep
            .iter()
            .any(|row| row.cell.path_diversity_preference_permille == 500));
        assert!(sweep
            .iter()
            .any(|row| row.cell.reproduction_target_low_permille == 900));
    }

    #[test]
    fn observer_sweep_seeded_randomness_is_replay_stable() {
        let first = run_observer_sweep(47)
            .into_iter()
            .find(|row| {
                row.cell.forwarding_randomness == ObserverForwardingRandomness::SeededPermutation
                    && row.cell.fragment_dispersion_permille == 800
            })
            .expect("seeded row");
        let second = run_observer_sweep(47)
            .into_iter()
            .find(|row| row.cell == first.cell)
            .expect("same cell");

        assert_eq!(first, second);
    }
}
