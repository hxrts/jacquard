//! Plot-ready observer ambiguity artifact rows.

use serde::{Deserialize, Serialize};

use super::{
    run_observer_robustness_summary, run_observer_sweep, ObserverAttackerTarget,
    ObserverForwardingRandomness, ObserverProjectionKind, ObserverRobustnessSummary,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverArtifactRow {
    pub observer_projection_identity: ObserverProjectionKind,
    pub attacker_target: ObserverAttackerTarget,
    pub coding_rate_k: u32,
    pub coding_rate_n: u32,
    pub fragment_dispersion_permille: u32,
    pub forwarding_randomness: ObserverForwardingRandomness,
    pub path_diversity_preference_permille: u32,
    pub reproduction_target_low_permille: u32,
    pub reproduction_target_high_permille: u32,
    pub top_guess_cluster_id: u8,
    pub true_target_rank: u32,
    pub attacker_top1_accuracy_permille: u32,
    pub posterior_uncertainty_permille: u32,
    pub hidden_projection_proxy_permille: u32,
    pub forwarding_contact_proxy_permille: u32,
    pub ambiguity_cost_frontier_area: u64,
    pub cost_bytes: u32,
    pub latency_rounds: u32,
    pub quality_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverArtifactBundle {
    pub rows: Vec<ObserverArtifactRow>,
    pub summary: ObserverRobustnessSummary,
}

pub(crate) fn observer_artifact_rows(seed: u64) -> ObserverArtifactBundle {
    let rows = run_observer_sweep(seed)
        .into_iter()
        .map(|row| artifact_row(&row))
        .collect::<Vec<_>>();
    let summary = run_observer_robustness_summary(&[seed]);
    ObserverArtifactBundle { rows, summary }
}

fn artifact_row(row: &super::ObserverSweepArtifact) -> ObserverArtifactRow {
    ObserverArtifactRow {
        observer_projection_identity: row.cell.projection_kind,
        attacker_target: row.cell.attacker_target,
        coding_rate_k: row.cell.coding_rate_k,
        coding_rate_n: row.cell.coding_rate_n,
        fragment_dispersion_permille: row.cell.fragment_dispersion_permille,
        forwarding_randomness: row.cell.forwarding_randomness,
        path_diversity_preference_permille: row.cell.path_diversity_preference_permille,
        reproduction_target_low_permille: row.cell.reproduction_target_low_permille,
        reproduction_target_high_permille: row.cell.reproduction_target_high_permille,
        top_guess_cluster_id: row.attacker_result.top_guess_cluster_id,
        true_target_rank: row.attacker_result.true_target_rank,
        attacker_top1_accuracy_permille: row.metrics.attacker_top1_accuracy_permille,
        posterior_uncertainty_permille: row.metrics.posterior_uncertainty_permille,
        hidden_projection_proxy_permille: row.metrics.hidden_projection_proxy_permille,
        forwarding_contact_proxy_permille: row.metrics.forwarding_contact_proxy_permille,
        ambiguity_cost_frontier_area: row.metrics.ambiguity_cost_frontier_area,
        cost_bytes: row.cost_bytes,
        latency_rounds: row.latency_rounds,
        quality_permille: row.quality_permille,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_artifacts_expose_plot_ready_columns() {
        let bundle = observer_artifact_rows(41);
        let json = serde_json::to_string(&bundle.rows[0]).expect("json");

        for field in [
            "observer_projection_identity",
            "attacker_target",
            "fragment_dispersion_permille",
            "forwarding_randomness",
            "attacker_top1_accuracy_permille",
            "posterior_uncertainty_permille",
            "hidden_projection_proxy_permille",
            "ambiguity_cost_frontier_area",
            "cost_bytes",
            "latency_rounds",
            "quality_permille",
        ] {
            assert!(json.contains(field));
        }
    }

    #[test]
    fn observer_artifacts_are_deterministic() {
        let first = observer_artifact_rows(41);
        let second = observer_artifact_rows(41);

        assert_eq!(first, second);
        assert_eq!(first.rows.len(), 64);
        assert!(first.summary.ambiguity_cost_frontier_area > 0);
    }
}
// proc-macro-scope: observer artifact rows are replay schema, not shared model vocabulary.
