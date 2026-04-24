//! Diffusion message-delivery simulation with posture, scoring, and statistics.

use std::collections::{BTreeMap, BTreeSet};

mod baselines;
mod catalog;
mod coded_inference;
mod model;
mod posture;
mod runtime;
mod scoring;
mod stats;

use model::{
    DiffusionContactEvent, DiffusionFieldPosture, DiffusionMessageMode, DiffusionMobilityProfile,
    DiffusionNodeSpec, DiffusionScenarioSpec, DiffusionTransportKind,
};
#[allow(unused_imports)]
use posture::{
    classify_field_transfer, compute_field_posture_signals, count_field_posture_round,
    covered_target_clusters, desired_field_posture, diffusion_bridge_candidate,
    diffusion_destination_cluster, diffusion_source_cluster, dominant_field_posture_name,
    field_budget_kind, field_forwarding_suppressed, holder_count_in_cluster, initial_field_budget,
    initial_field_posture, sender_energy_ratio_permille,
};
#[allow(unused_imports)]
use runtime::{
    coverage_permille_for, is_target_node, is_terminal_target, node_by_id,
    scenario_target_cluster_count, FieldBudgetKind, FieldBudgetState, FieldExecutionMetrics,
    FieldPostureMetrics, FieldPostureSignals, FieldSuppressionState, FieldTransferFeatures,
    ForwardingGeometry, ForwardingNodes, ForwardingOpportunity, ForwardingScoreContext,
    HolderState, PendingTransfer,
};
#[allow(unused_imports)]
use scoring::forwarding_score;
#[allow(unused_imports)]
use stats::{mean_option_u32, mean_u32, min_max_spread_u32, mode_option_string, mode_string};

#[cfg(test)]
use runtime::{
    execute_diffusion_runs_parallel, execute_diffusion_runs_serial, simulate_diffusion_run,
};

pub use catalog::{diffusion_local_stage_suite, diffusion_local_suite, diffusion_smoke_suite};
pub use model::{
    DiffusionAggregateSummary, DiffusionArtifacts, DiffusionBoundarySummary,
    DiffusionForwardingStyle, DiffusionManifest, DiffusionPolicyConfig, DiffusionRegimeDescriptor,
    DiffusionRunSummary, DiffusionSuite,
};
pub use runtime::{aggregate_diffusion_runs, run_diffusion_suite, summarize_diffusion_boundaries};

#[cfg(test)]
mod tests {
    use super::{
        diffusion_local_stage_suite, diffusion_local_suite, diffusion_smoke_suite,
        execute_diffusion_runs_parallel, execute_diffusion_runs_serial, simulate_diffusion_run,
    };

    #[test]
    fn diffusion_parallel_suite_matches_serial_ordered_runs() {
        let suite = diffusion_smoke_suite();
        let serial = execute_diffusion_runs_serial(&suite);
        let parallel = execute_diffusion_runs_parallel(&suite);

        assert_eq!(serial, parallel);
    }

    #[test]
    fn diffusion_runs_are_repeatable() {
        let suite = diffusion_smoke_suite();
        let first = simulate_diffusion_run(&suite.runs[0]);
        let second = simulate_diffusion_run(&suite.runs[0]);

        assert_eq!(first, second);
    }

    #[test]
    fn diffusion_local_stage_suites_materialize_runs() {
        for stage_id in [
            "diffusion-local-stage-1",
            "diffusion-local-stage-2",
            "diffusion-local-stage-3",
            "diffusion-local-stage-4",
        ] {
            let suite =
                diffusion_local_stage_suite(stage_id).expect("diffusion local stage should exist");
            assert!(
                !suite.runs.is_empty(),
                "{stage_id} should include diffusion runs"
            );
            assert!(
                suite
                    .runs
                    .iter()
                    .all(|run| run.family_id.starts_with("diffusion-")),
                "{stage_id} should only include diffusion-prefixed family ids"
            );
        }
    }

    #[test]
    fn diffusion_suites_include_mercator_profile() {
        let suite = diffusion_smoke_suite();
        assert!(
            suite
                .runs
                .iter()
                .any(|run| run.policy.config_id == "mercator"),
            "diffusion smoke suite should include mercator"
        );
    }

    #[test]
    fn diffusion_mercator_runs_cover_phase_five_fixtures() {
        let suite = diffusion_local_suite();
        for family_id in [
            "diffusion-bridge-drought",
            "diffusion-energy-starved-relay",
            "diffusion-congestion-cascade",
            "diffusion-large-sparse-threshold-moderate",
            "diffusion-large-regional-shift-moderate",
        ] {
            assert!(
                suite
                    .runs
                    .iter()
                    .any(|run| run.family_id == family_id && run.policy.config_id == "mercator"),
                "{family_id} should include mercator diffusion rows"
            );
        }
    }
}
