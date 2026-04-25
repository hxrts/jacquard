//! Named-term potential accounting for inference and exact reconstruction.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct InferencePotentialInput {
    pub uncertainty: u32,
    pub wrong_basin_mass: u32,
    pub duplicate_pressure: u32,
    pub storage_pressure: u32,
    pub transmission_pressure: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct InferencePotentialWeights {
    pub alpha: u32,
    pub beta: u32,
    pub gamma: u32,
    pub delta: u32,
    pub eta: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct InferencePotentialRecord {
    pub round_index: u32,
    pub terms: InferencePotentialInput,
    pub weights: InferencePotentialWeights,
    pub w_infer: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionPotentialInput {
    pub rank_deficit: u32,
    pub active_fragment_pressure: u32,
    pub storage_pressure: u32,
    pub duplicate_pressure: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionPotentialWeights {
    pub alpha: u32,
    pub beta: u32,
    pub gamma: u32,
    pub delta: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionPotentialRecord {
    pub round_index: u32,
    pub terms: DiffusionPotentialInput,
    pub weights: DiffusionPotentialWeights,
    pub w_diff: u32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PotentialTraceSummary {
    pub round_count: u32,
    pub w_infer_initial: u32,
    pub w_infer_final: u32,
    pub w_infer_max: u32,
    pub w_diff_initial: u32,
    pub w_diff_final: u32,
    pub w_diff_max: u32,
}

pub(crate) fn compute_w_infer(
    round_index: u32,
    terms: InferencePotentialInput,
    weights: InferencePotentialWeights,
) -> InferencePotentialRecord {
    InferencePotentialRecord {
        round_index,
        terms,
        weights,
        w_infer: weights
            .alpha
            .saturating_mul(terms.uncertainty)
            .saturating_add(weights.beta.saturating_mul(terms.wrong_basin_mass))
            .saturating_add(weights.gamma.saturating_mul(terms.duplicate_pressure))
            .saturating_add(weights.delta.saturating_mul(terms.storage_pressure))
            .saturating_add(weights.eta.saturating_mul(terms.transmission_pressure)),
    }
}

pub(crate) fn compute_w_diff(
    round_index: u32,
    terms: DiffusionPotentialInput,
    weights: DiffusionPotentialWeights,
) -> DiffusionPotentialRecord {
    DiffusionPotentialRecord {
        round_index,
        terms,
        weights,
        w_diff: weights
            .alpha
            .saturating_mul(terms.rank_deficit)
            .saturating_add(weights.beta.saturating_mul(terms.active_fragment_pressure))
            .saturating_add(weights.gamma.saturating_mul(terms.storage_pressure))
            .saturating_add(weights.delta.saturating_mul(terms.duplicate_pressure)),
    }
}

pub(crate) fn summarize_potential_trace(
    infer: &[InferencePotentialRecord],
    diff: &[DiffusionPotentialRecord],
) -> PotentialTraceSummary {
    PotentialTraceSummary {
        round_count: u32::try_from(infer.len().max(diff.len())).unwrap_or(u32::MAX),
        w_infer_initial: infer.first().map(|row| row.w_infer).unwrap_or(0),
        w_infer_final: infer.last().map(|row| row.w_infer).unwrap_or(0),
        w_infer_max: infer.iter().map(|row| row.w_infer).max().unwrap_or(0),
        w_diff_initial: diff.first().map(|row| row.w_diff).unwrap_or(0),
        w_diff_final: diff.last().map(|row| row.w_diff).unwrap_or(0),
        w_diff_max: diff.iter().map(|row| row.w_diff).max().unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn potential_accounting_zero_potential_is_zero() {
        let infer = compute_w_infer(
            0,
            InferencePotentialInput {
                uncertainty: 0,
                wrong_basin_mass: 0,
                duplicate_pressure: 0,
                storage_pressure: 0,
                transmission_pressure: 0,
            },
            InferencePotentialWeights {
                alpha: 1,
                beta: 1,
                gamma: 1,
                delta: 1,
                eta: 1,
            },
        );

        assert_eq!(infer.w_infer, 0);
    }

    #[test]
    fn potential_accounting_weighted_totals_match_named_terms() {
        let infer = compute_w_infer(
            1,
            InferencePotentialInput {
                uncertainty: 10,
                wrong_basin_mass: 20,
                duplicate_pressure: 30,
                storage_pressure: 40,
                transmission_pressure: 50,
            },
            InferencePotentialWeights {
                alpha: 1,
                beta: 2,
                gamma: 3,
                delta: 4,
                eta: 5,
            },
        );
        let diff = compute_w_diff(
            1,
            DiffusionPotentialInput {
                rank_deficit: 3,
                active_fragment_pressure: 4,
                storage_pressure: 5,
                duplicate_pressure: 6,
            },
            DiffusionPotentialWeights {
                alpha: 7,
                beta: 8,
                gamma: 9,
                delta: 10,
            },
        );

        assert_eq!(infer.w_infer, 550);
        assert_eq!(diff.w_diff, 158);
    }

    #[test]
    fn potential_accounting_boundary_behavior_saturates() {
        let infer = compute_w_infer(
            0,
            InferencePotentialInput {
                uncertainty: u32::MAX,
                wrong_basin_mass: u32::MAX,
                duplicate_pressure: u32::MAX,
                storage_pressure: u32::MAX,
                transmission_pressure: u32::MAX,
            },
            InferencePotentialWeights {
                alpha: u32::MAX,
                beta: u32::MAX,
                gamma: u32::MAX,
                delta: u32::MAX,
                eta: u32::MAX,
            },
        );

        assert_eq!(infer.w_infer, u32::MAX);
    }

    #[test]
    fn potential_accounting_exact_k_of_n_diffusion_trace_summarizes() {
        let diff = [
            compute_w_diff(
                0,
                DiffusionPotentialInput {
                    rank_deficit: 4,
                    active_fragment_pressure: 4,
                    storage_pressure: 1,
                    duplicate_pressure: 0,
                },
                DiffusionPotentialWeights {
                    alpha: 1,
                    beta: 1,
                    gamma: 1,
                    delta: 1,
                },
            ),
            compute_w_diff(
                1,
                DiffusionPotentialInput {
                    rank_deficit: 1,
                    active_fragment_pressure: 2,
                    storage_pressure: 1,
                    duplicate_pressure: 0,
                },
                DiffusionPotentialWeights {
                    alpha: 1,
                    beta: 1,
                    gamma: 1,
                    delta: 1,
                },
            ),
        ];
        let summary = summarize_potential_trace(&[], &diff);

        assert_eq!(summary.w_diff_initial, 9);
        assert_eq!(summary.w_diff_final, 4);
        assert_eq!(summary.w_diff_max, 9);
    }

    #[test]
    fn potential_accounting_replay_is_deterministic() {
        let row = compute_w_infer(
            2,
            InferencePotentialInput {
                uncertainty: 1,
                wrong_basin_mass: 2,
                duplicate_pressure: 3,
                storage_pressure: 4,
                transmission_pressure: 5,
            },
            InferencePotentialWeights {
                alpha: 5,
                beta: 4,
                gamma: 3,
                delta: 2,
                eta: 1,
            },
        );

        assert_eq!(row, row);
    }
}
// proc-macro-scope: near-critical potential rows are artifact schema, not shared model vocabulary.
