//! Paper-facing coded-diffusion artifact boundary.
// proc-macro-scope: paper artifact contract types are lightweight facade schema, not shared model vocabulary.
//!
//! The generic diffusion runner and route-visible analysis writer must not
//! depend on these datasets. They are consumed by `analysis_2/` only.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaperExperimentArtifactContract {
    pub analysis_surface: &'static str,
    pub consumed_by_route_analysis: bool,
    pub required_csv_files: &'static [&'static str],
}

pub const ACTIVE_BELIEF_REQUIRED_CSV_FILES: &[&str] = &[
    "active_belief_figure_claim_map.csv",
    "active_belief_raw_rounds.csv",
    "active_belief_receiver_runs.csv",
    "active_belief_path_validation.csv",
    "active_belief_demand_ablation.csv",
    "active_belief_demand_byte_sweep.csv",
    "active_belief_high_gap_regimes.csv",
    "active_belief_adversarial_demand.csv",
    "active_belief_byzantine_injection.csv",
    "active_belief_scale_validation.csv",
    "active_belief_receiver_count_sweep.csv",
    "active_belief_independence_bottleneck.csv",
    "active_belief_convex_erm.csv",
    "coded_inference_experiment_a_landscape.csv",
    "coded_inference_experiment_a2_evidence_modes.csv",
    "coded_inference_experiment_b_path_free_recovery.csv",
    "coded_inference_experiment_c_phase_diagram.csv",
    "coded_inference_experiment_d_coding_vs_replication.csv",
    "coded_inference_experiment_e_observer_frontier.csv",
];

#[must_use]
pub const fn active_belief_artifact_contract() -> PaperExperimentArtifactContract {
    PaperExperimentArtifactContract {
        analysis_surface: "analysis_2",
        consumed_by_route_analysis: false,
        required_csv_files: ACTIVE_BELIEF_REQUIRED_CSV_FILES,
    }
}
