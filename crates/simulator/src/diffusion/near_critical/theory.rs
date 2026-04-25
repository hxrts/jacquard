//! Qualitative near-critical theory fixtures.

use serde::{Deserialize, Serialize};

use super::{run_near_critical_sweep, ControllerModeKind, NearCriticalSweepRegion};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum NearCriticalTheoryFixtureKind {
    SubcriticalCollapse,
    SupercriticalCostGrowth,
    ControlledNearCritical,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalTheoryFixture {
    pub kind: NearCriticalTheoryFixtureKind,
    pub recovery_permille: u32,
    pub quality_permille: u32,
    pub byte_cost: u32,
    pub potential_bounded: bool,
    pub full_controller_beats_ablation: bool,
}

// long-block-exception: fixture table mirrors the theorem-facing near-critical scenarios.
pub(crate) fn run_near_critical_theory_fixtures(seed: u64) -> Vec<NearCriticalTheoryFixture> {
    let sweep = run_near_critical_sweep(seed);
    let subcritical = sweep
        .iter()
        .find(|row| {
            row.cell.region == NearCriticalSweepRegion::Subcritical
                && row.cell.controller_mode == ControllerModeKind::Full
                && row.cell.forwarding_budget == 1
        })
        .expect("subcritical");
    let supercritical = sweep
        .iter()
        .find(|row| {
            row.cell.region == NearCriticalSweepRegion::Supercritical
                && row.cell.controller_mode == ControllerModeKind::Disabled
                && row.cell.forwarding_budget == 4
        })
        .expect("supercritical");
    let controlled = sweep
        .iter()
        .find(|row| {
            row.cell.region == NearCriticalSweepRegion::NearCritical
                && row.cell.controller_mode == ControllerModeKind::Full
                && row.cell.forwarding_budget == 4
        })
        .expect("controlled");
    let ablation = sweep
        .iter()
        .find(|row| {
            row.cell.region == NearCriticalSweepRegion::NearCritical
                && row.cell.controller_mode == ControllerModeKind::Disabled
                && row.cell.forwarding_budget == 4
        })
        .expect("ablation");
    vec![
        NearCriticalTheoryFixture {
            kind: NearCriticalTheoryFixtureKind::SubcriticalCollapse,
            recovery_permille: subcritical.recovery_permille,
            quality_permille: subcritical.quality_permille,
            byte_cost: subcritical.byte_cost,
            potential_bounded: subcritical.w_infer <= 3_000,
            full_controller_beats_ablation: false,
        },
        NearCriticalTheoryFixture {
            kind: NearCriticalTheoryFixtureKind::SupercriticalCostGrowth,
            recovery_permille: supercritical.recovery_permille,
            quality_permille: supercritical.quality_permille,
            byte_cost: supercritical.byte_cost,
            potential_bounded: supercritical.w_infer <= 3_000,
            full_controller_beats_ablation: false,
        },
        NearCriticalTheoryFixture {
            kind: NearCriticalTheoryFixtureKind::ControlledNearCritical,
            recovery_permille: controlled.recovery_permille,
            quality_permille: controlled.quality_permille,
            byte_cost: controlled.byte_cost,
            potential_bounded: controlled.w_infer <= ablation.w_infer,
            full_controller_beats_ablation: controlled.byte_cost <= ablation.byte_cost,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(
        fixtures: &[NearCriticalTheoryFixture],
        kind: NearCriticalTheoryFixtureKind,
    ) -> &NearCriticalTheoryFixture {
        fixtures
            .iter()
            .find(|fixture| fixture.kind == kind)
            .expect("fixture")
    }

    #[test]
    fn near_critical_theory_subcritical_fixture_collapses() {
        let fixtures = run_near_critical_theory_fixtures(41);
        let subcritical = fixture(
            &fixtures,
            NearCriticalTheoryFixtureKind::SubcriticalCollapse,
        );

        assert_eq!(subcritical.recovery_permille, 0);
        assert!(subcritical.quality_permille < 800);
    }

    #[test]
    fn near_critical_theory_supercritical_fixture_recovers_with_high_cost() {
        let fixtures = run_near_critical_theory_fixtures(41);
        let supercritical = fixture(
            &fixtures,
            NearCriticalTheoryFixtureKind::SupercriticalCostGrowth,
        );

        assert_eq!(supercritical.recovery_permille, 1000);
        assert!(supercritical.byte_cost >= 128);
    }

    #[test]
    fn near_critical_theory_controlled_fixture_recovers_with_bounded_potential() {
        let fixtures = run_near_critical_theory_fixtures(41);
        let controlled = fixture(
            &fixtures,
            NearCriticalTheoryFixtureKind::ControlledNearCritical,
        );

        assert_eq!(controlled.recovery_permille, 1000);
        assert!(controlled.potential_bounded);
        assert!(controlled.full_controller_beats_ablation);
    }

    #[test]
    fn near_critical_theory_replay_is_deterministic() {
        let first = run_near_critical_theory_fixtures(41);
        let second = run_near_critical_theory_fixtures(41);

        assert_eq!(first, second);
    }
}
// proc-macro-scope: near-critical theory fixture rows are artifact schema, not shared model vocabulary.
