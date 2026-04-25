//! Target-band and forwarding-budget sweep artifacts.

use serde::{Deserialize, Serialize};

use super::{
    compute_w_diff, compute_w_infer, decide_near_critical_controller, DiffusionPotentialInput,
    DiffusionPotentialWeights, InferencePotentialInput, InferencePotentialWeights,
    NearCriticalControllerConfig, NearCriticalControllerDecision, NearCriticalControllerMode,
    NearCriticalOpportunityState, NearCriticalResourceUsage, ReproductionPressureSummary,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum NearCriticalSweepRegion {
    Subcritical,
    NearCritical,
    Supercritical,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ControllerModeKind {
    Full,
    Disabled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalSweepCell {
    pub scenario_id: String,
    pub seed: u64,
    pub region: NearCriticalSweepRegion,
    pub controller_mode: ControllerModeKind,
    pub r_low_permille: u32,
    pub r_high_permille: u32,
    pub forwarding_budget: u32,
    pub storage_cap_units: u32,
    pub transmission_cap_count: u32,
    pub payload_byte_cap: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalSweepArtifact {
    pub cell: NearCriticalSweepCell,
    pub controller_decision: NearCriticalControllerDecision,
    pub recovery_permille: u32,
    pub commitment_permille: u32,
    pub quality_permille: u32,
    pub byte_cost: u32,
    pub transmission_cost: u32,
    pub storage_pressure: u32,
    pub duplicate_pressure: u32,
    pub w_infer: u32,
    pub w_diff: u32,
}

pub(crate) fn run_near_critical_sweep(seed: u64) -> Vec<NearCriticalSweepArtifact> {
    sweep_cells(seed).into_iter().map(run_cell).collect()
}

fn sweep_cells(seed: u64) -> Vec<NearCriticalSweepCell> {
    let bands = [
        (NearCriticalSweepRegion::Subcritical, 0, 400),
        (NearCriticalSweepRegion::NearCritical, 800, 1000),
        (NearCriticalSweepRegion::Supercritical, 1000, 1000),
    ];
    let budgets = [1, 2, 4, 16, 32, 64];
    let mut cells = Vec::new();
    for (region, low, high) in bands {
        for forwarding_budget in budgets {
            for controller_mode in [ControllerModeKind::Full, ControllerModeKind::Disabled] {
                cells.push(NearCriticalSweepCell {
                    scenario_id: "coded-inference-near-critical".to_string(),
                    seed,
                    region,
                    controller_mode,
                    r_low_permille: low,
                    r_high_permille: high,
                    forwarding_budget,
                    storage_cap_units: 4,
                    transmission_cap_count: forwarding_budget,
                    payload_byte_cap: forwarding_budget.saturating_mul(32),
                });
            }
        }
    }
    cells
}

fn run_cell(cell: NearCriticalSweepCell) -> NearCriticalSweepArtifact {
    let pressure = pressure_for(cell.region);
    let config = NearCriticalControllerConfig::try_new(
        cell.r_low_permille,
        cell.r_high_permille,
        cell.storage_cap_units,
        cell.transmission_cap_count,
        cell.payload_byte_cap,
    )
    .expect("valid sweep config");
    let decision = if cell.controller_mode == ControllerModeKind::Full {
        decide_near_critical_controller(
            config,
            pressure,
            NearCriticalResourceUsage {
                storage_units: 0,
                transmission_count: 0,
                payload_bytes: 0,
            },
            NearCriticalOpportunityState {
                candidate_forwarding_opportunities: cell.forwarding_budget,
                payload_bytes_per_opportunity: 32,
            },
        )
    } else {
        disabled_decision(config, pressure, cell.forwarding_budget)
    };
    artifact_from_decision(cell, pressure, decision)
}

pub(crate) fn pressure_for(region: NearCriticalSweepRegion) -> ReproductionPressureSummary {
    let r_est_permille = match region {
        NearCriticalSweepRegion::Subcritical => 200,
        NearCriticalSweepRegion::NearCritical => 900,
        NearCriticalSweepRegion::Supercritical => 1000,
    };
    ReproductionPressureSummary {
        active_forwarding_opportunities: 4,
        innovative_successor_opportunities: r_est_permille / 250,
        r_est_permille,
        duplicate_arrivals: 1000_u32.saturating_sub(r_est_permille),
        ..ReproductionPressureSummary::default()
    }
}

pub(crate) fn disabled_decision(
    config: NearCriticalControllerConfig,
    pressure: ReproductionPressureSummary,
    forwarding_budget: u32,
) -> NearCriticalControllerDecision {
    let emitted_opportunities = forwarding_budget
        .min(config.transmission_cap_count)
        .min(config.payload_byte_cap.checked_div(32).unwrap_or(0));
    NearCriticalControllerDecision {
        r_est_permille: pressure.r_est_permille,
        r_low_permille: config.r_low_permille,
        r_high_permille: config.r_high_permille,
        mode: NearCriticalControllerMode::Steady,
        cap_state: super::NearCriticalCapState {
            storage_saturated: false,
            transmission_saturated: false,
            byte_saturated: false,
        },
        input_opportunities: forwarding_budget,
        emitted_opportunities,
        suppressed_opportunities: 0,
        added_opportunities: 0,
    }
}

fn artifact_from_decision(
    cell: NearCriticalSweepCell,
    pressure: ReproductionPressureSummary,
    decision: NearCriticalControllerDecision,
) -> NearCriticalSweepArtifact {
    let transmission_cost = decision.emitted_opportunities;
    let byte_cost = transmission_cost.saturating_mul(32);
    let quality_permille = pressure
        .r_est_permille
        .saturating_mul(transmission_cost)
        .saturating_div(cell.forwarding_budget.max(1))
        .min(1000);
    let infer = compute_w_infer(
        0,
        InferencePotentialInput {
            uncertainty: 1000_u32.saturating_sub(quality_permille),
            wrong_basin_mass: 1000_u32.saturating_sub(quality_permille),
            duplicate_pressure: pressure.duplicate_arrivals.min(1000),
            storage_pressure: transmission_cost.saturating_mul(100),
            transmission_pressure: byte_cost,
        },
        InferencePotentialWeights {
            alpha: 1,
            beta: 1,
            gamma: 1,
            delta: 1,
            eta: 1,
        },
    );
    let diff = compute_w_diff(
        0,
        DiffusionPotentialInput {
            rank_deficit: 1000_u32.saturating_sub(quality_permille),
            active_fragment_pressure: pressure.active_forwarding_opportunities,
            storage_pressure: transmission_cost.saturating_mul(100),
            duplicate_pressure: pressure.duplicate_arrivals.min(1000),
        },
        DiffusionPotentialWeights {
            alpha: 1,
            beta: 1,
            gamma: 1,
            delta: 1,
        },
    );
    NearCriticalSweepArtifact {
        cell,
        controller_decision: decision,
        recovery_permille: if quality_permille >= 800 { 1000 } else { 0 },
        commitment_permille: if quality_permille >= 800 { 1000 } else { 0 },
        quality_permille,
        byte_cost,
        transmission_cost,
        storage_pressure: transmission_cost.saturating_mul(100),
        duplicate_pressure: pressure.duplicate_arrivals.min(1000),
        w_infer: infer.w_infer,
        w_diff: diff.w_diff,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_critical_sweep_generates_complete_deterministic_cells() {
        let first = run_near_critical_sweep(41);
        let second = run_near_critical_sweep(41);

        assert_eq!(first, second);
        assert_eq!(first.len(), 18);
    }

    #[test]
    fn near_critical_sweep_cells_carry_band_budget_and_mode() {
        let sweep = run_near_critical_sweep(41);

        assert!(sweep
            .iter()
            .any(|row| row.cell.region == NearCriticalSweepRegion::Subcritical));
        assert!(sweep
            .iter()
            .any(|row| row.cell.region == NearCriticalSweepRegion::NearCritical));
        assert!(sweep
            .iter()
            .any(|row| row.cell.region == NearCriticalSweepRegion::Supercritical));
        assert!(sweep.iter().any(|row| row.cell.forwarding_budget == 1));
        assert!(sweep.iter().any(|row| row.cell.forwarding_budget == 4));
        assert!(sweep
            .iter()
            .any(|row| row.cell.controller_mode == ControllerModeKind::Full));
        assert!(sweep
            .iter()
            .any(|row| row.cell.controller_mode == ControllerModeKind::Disabled));
    }

    #[test]
    fn near_critical_ablation_preserves_caps_and_budget_schema() {
        let sweep = run_near_critical_sweep(41);
        let full = sweep
            .iter()
            .find(|row| {
                row.cell.controller_mode == ControllerModeKind::Full
                    && row.cell.region == NearCriticalSweepRegion::NearCritical
                    && row.cell.forwarding_budget == 4
            })
            .expect("full");
        let ablation = sweep
            .iter()
            .find(|row| {
                row.cell.controller_mode == ControllerModeKind::Disabled
                    && row.cell.region == NearCriticalSweepRegion::NearCritical
                    && row.cell.forwarding_budget == 4
            })
            .expect("ablation");

        assert_eq!(full.cell.storage_cap_units, ablation.cell.storage_cap_units);
        assert_eq!(
            full.cell.transmission_cap_count,
            ablation.cell.transmission_cap_count
        );
        assert_eq!(full.cell.payload_byte_cap, ablation.cell.payload_byte_cap);
        assert_eq!(ablation.cell.controller_mode, ControllerModeKind::Disabled);
    }

    #[test]
    fn near_critical_ablation_does_not_apply_band_adjustments() {
        let below = disabled_decision(
            NearCriticalControllerConfig::try_new(800, 900, 4, 4, 128).expect("config"),
            pressure_for(NearCriticalSweepRegion::Subcritical),
            2,
        );
        let above = disabled_decision(
            NearCriticalControllerConfig::try_new(800, 900, 4, 4, 128).expect("config"),
            pressure_for(NearCriticalSweepRegion::Supercritical),
            2,
        );

        assert_eq!(below.mode, NearCriticalControllerMode::Steady);
        assert_eq!(below.emitted_opportunities, 2);
        assert_eq!(above.mode, NearCriticalControllerMode::Steady);
        assert_eq!(above.emitted_opportunities, 2);
    }

    #[test]
    fn near_critical_ablation_differs_from_full_controller_on_fixture() {
        let config = NearCriticalControllerConfig::try_new(800, 900, 4, 4, 128).expect("config");
        let pressure = pressure_for(NearCriticalSweepRegion::Subcritical);
        let full = decide_near_critical_controller(
            config,
            pressure,
            NearCriticalResourceUsage {
                storage_units: 0,
                transmission_count: 0,
                payload_bytes: 0,
            },
            NearCriticalOpportunityState {
                candidate_forwarding_opportunities: 2,
                payload_bytes_per_opportunity: 32,
            },
        );
        let ablation = disabled_decision(config, pressure, 2);

        assert_ne!(full.mode, ablation.mode);
        assert_ne!(full.emitted_opportunities, ablation.emitted_opportunities);
    }
}
