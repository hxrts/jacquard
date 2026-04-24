//! Plot-ready near-critical artifact rows.

use serde::{Deserialize, Serialize};

use super::{
    run_near_critical_sweep, ControllerModeKind, NearCriticalControllerMode,
    NearCriticalSweepArtifact,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalRoundArtifact {
    pub round_index: u32,
    pub r_est_permille: u32,
    pub r_low_permille: u32,
    pub r_high_permille: u32,
    pub controller_action: NearCriticalControllerMode,
    pub storage_saturated: bool,
    pub transmission_saturated: bool,
    pub byte_saturated: bool,
    pub w_infer: u32,
    pub w_diff: u32,
    pub byte_cost: u32,
    pub transmission_cost: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalSummaryArtifact {
    pub row_count: u32,
    pub bounded_round_count: u32,
    pub recovery_count: u32,
    pub commitment_count: u32,
    pub max_byte_cost: u32,
    pub max_duplicate_pressure: u32,
    pub max_storage_pressure: u32,
    pub w_infer_max: u32,
    pub w_diff_max: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalArtifactBundle {
    pub rounds: Vec<NearCriticalRoundArtifact>,
    pub summary: NearCriticalSummaryArtifact,
}

pub(crate) fn near_critical_artifact_rows(seed: u64) -> NearCriticalArtifactBundle {
    let sweep = run_near_critical_sweep(seed);
    let rounds = sweep
        .iter()
        .enumerate()
        .map(|(index, row)| round_artifact(u32::try_from(index).unwrap_or(u32::MAX), row))
        .collect::<Vec<_>>();
    let summary = summary_artifact(&sweep);
    NearCriticalArtifactBundle { rounds, summary }
}

fn round_artifact(round_index: u32, row: &NearCriticalSweepArtifact) -> NearCriticalRoundArtifact {
    NearCriticalRoundArtifact {
        round_index,
        r_est_permille: row.controller_decision.r_est_permille,
        r_low_permille: row.controller_decision.r_low_permille,
        r_high_permille: row.controller_decision.r_high_permille,
        controller_action: row.controller_decision.mode,
        storage_saturated: row.controller_decision.cap_state.storage_saturated,
        transmission_saturated: row.controller_decision.cap_state.transmission_saturated,
        byte_saturated: row.controller_decision.cap_state.byte_saturated,
        w_infer: row.w_infer,
        w_diff: row.w_diff,
        byte_cost: row.byte_cost,
        transmission_cost: row.transmission_cost,
    }
}

fn summary_artifact(rows: &[NearCriticalSweepArtifact]) -> NearCriticalSummaryArtifact {
    NearCriticalSummaryArtifact {
        row_count: u32::try_from(rows.len()).unwrap_or(u32::MAX),
        bounded_round_count: u32::try_from(
            rows.iter()
                .filter(|row| row.cell.controller_mode == ControllerModeKind::Full)
                .count(),
        )
        .unwrap_or(u32::MAX),
        recovery_count: u32::try_from(rows.iter().filter(|row| row.recovery_permille > 0).count())
            .unwrap_or(u32::MAX),
        commitment_count: u32::try_from(
            rows.iter()
                .filter(|row| row.commitment_permille > 0)
                .count(),
        )
        .unwrap_or(u32::MAX),
        max_byte_cost: rows.iter().map(|row| row.byte_cost).max().unwrap_or(0),
        max_duplicate_pressure: rows
            .iter()
            .map(|row| row.duplicate_pressure)
            .max()
            .unwrap_or(0),
        max_storage_pressure: rows
            .iter()
            .map(|row| row.storage_pressure)
            .max()
            .unwrap_or(0),
        w_infer_max: rows.iter().map(|row| row.w_infer).max().unwrap_or(0),
        w_diff_max: rows.iter().map(|row| row.w_diff).max().unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_critical_artifacts_expose_plot_ready_round_columns() {
        let bundle = near_critical_artifact_rows(41);
        let json = serde_json::to_string(&bundle.rounds[0]).expect("json");

        for field in [
            "r_est_permille",
            "r_low_permille",
            "r_high_permille",
            "controller_action",
            "w_infer",
            "w_diff",
            "byte_cost",
            "transmission_cost",
        ] {
            assert!(json.contains(field));
        }
    }

    #[test]
    fn near_critical_artifacts_summary_is_deterministic() {
        let first = near_critical_artifact_rows(41);
        let second = near_critical_artifact_rows(41);

        assert_eq!(first, second);
        assert_eq!(first.summary.row_count, 18);
        assert!(first.summary.w_infer_max > 0);
        assert!(first.summary.w_diff_max > 0);
    }
}
