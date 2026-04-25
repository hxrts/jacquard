//! Near-critical controller over rolling reproduction pressure.

use serde::{Deserialize, Serialize};

use super::ReproductionPressureSummary;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NearCriticalControllerError {
    InvalidBand,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalControllerConfig {
    pub r_low_permille: u32,
    pub r_high_permille: u32,
    pub storage_cap_units: u32,
    pub transmission_cap_count: u32,
    pub payload_byte_cap: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalResourceUsage {
    pub storage_units: u32,
    pub transmission_count: u32,
    pub payload_bytes: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalOpportunityState {
    pub candidate_forwarding_opportunities: u32,
    pub payload_bytes_per_opportunity: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum NearCriticalControllerMode {
    IncreaseOpportunity,
    Steady,
    SuppressForwarding,
    HardCapBlocked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalCapState {
    pub storage_saturated: bool,
    pub transmission_saturated: bool,
    pub byte_saturated: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NearCriticalControllerDecision {
    pub r_est_permille: u32,
    pub r_low_permille: u32,
    pub r_high_permille: u32,
    pub mode: NearCriticalControllerMode,
    pub cap_state: NearCriticalCapState,
    pub input_opportunities: u32,
    pub emitted_opportunities: u32,
    pub suppressed_opportunities: u32,
    pub added_opportunities: u32,
}

impl NearCriticalControllerConfig {
    pub(crate) fn try_new(
        r_low_permille: u32,
        r_high_permille: u32,
        storage_cap_units: u32,
        transmission_cap_count: u32,
        payload_byte_cap: u32,
    ) -> Result<Self, NearCriticalControllerError> {
        if r_low_permille > r_high_permille || r_high_permille > 1_000 {
            return Err(NearCriticalControllerError::InvalidBand);
        }
        Ok(Self {
            r_low_permille,
            r_high_permille,
            storage_cap_units,
            transmission_cap_count,
            payload_byte_cap,
        })
    }
}

pub(crate) fn decide_near_critical_controller(
    config: NearCriticalControllerConfig,
    pressure: ReproductionPressureSummary,
    usage: NearCriticalResourceUsage,
    opportunity: NearCriticalOpportunityState,
) -> NearCriticalControllerDecision {
    let cap_state = NearCriticalCapState {
        storage_saturated: usage.storage_units >= config.storage_cap_units,
        transmission_saturated: usage.transmission_count >= config.transmission_cap_count,
        byte_saturated: usage.payload_bytes >= config.payload_byte_cap,
    };
    let input = opportunity.candidate_forwarding_opportunities;
    if cap_state.storage_saturated || cap_state.transmission_saturated || cap_state.byte_saturated {
        return decision(
            config,
            pressure,
            cap_state,
            input,
            0,
            NearCriticalControllerMode::HardCapBlocked,
        );
    }
    if pressure.r_est_permille > config.r_high_permille {
        return decision(
            config,
            pressure,
            cap_state,
            input,
            0,
            NearCriticalControllerMode::SuppressForwarding,
        );
    }
    let remaining_cap = remaining_opportunity_cap(config, usage, opportunity);
    if pressure.r_est_permille < config.r_low_permille {
        return decision(
            config,
            pressure,
            cap_state,
            input,
            input.saturating_add(1).min(remaining_cap),
            NearCriticalControllerMode::IncreaseOpportunity,
        );
    }
    decision(
        config,
        pressure,
        cap_state,
        input,
        input.min(remaining_cap),
        NearCriticalControllerMode::Steady,
    )
}

fn remaining_opportunity_cap(
    config: NearCriticalControllerConfig,
    usage: NearCriticalResourceUsage,
    opportunity: NearCriticalOpportunityState,
) -> u32 {
    let transmission_remaining = config
        .transmission_cap_count
        .saturating_sub(usage.transmission_count);
    let byte_remaining = config.payload_byte_cap.saturating_sub(usage.payload_bytes);
    let byte_limited = if opportunity.payload_bytes_per_opportunity == 0 {
        0
    } else {
        byte_remaining.saturating_div(opportunity.payload_bytes_per_opportunity)
    };
    transmission_remaining.min(byte_limited)
}

fn decision(
    config: NearCriticalControllerConfig,
    pressure: ReproductionPressureSummary,
    cap_state: NearCriticalCapState,
    input: u32,
    emitted: u32,
    mode: NearCriticalControllerMode,
) -> NearCriticalControllerDecision {
    NearCriticalControllerDecision {
        r_est_permille: pressure.r_est_permille,
        r_low_permille: config.r_low_permille,
        r_high_permille: config.r_high_permille,
        mode,
        cap_state,
        input_opportunities: input,
        emitted_opportunities: emitted,
        suppressed_opportunities: input.saturating_sub(emitted),
        added_opportunities: emitted.saturating_sub(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diffusion::near_critical::ReproductionPressureSummary;

    fn config() -> NearCriticalControllerConfig {
        NearCriticalControllerConfig::try_new(800, 900, 4, 4, 128).expect("config")
    }

    fn pressure(r_est_permille: u32) -> ReproductionPressureSummary {
        ReproductionPressureSummary {
            r_est_permille,
            ..ReproductionPressureSummary::default()
        }
    }

    fn usage(
        storage_units: u32,
        transmission_count: u32,
        payload_bytes: u32,
    ) -> NearCriticalResourceUsage {
        NearCriticalResourceUsage {
            storage_units,
            transmission_count,
            payload_bytes,
        }
    }

    #[test]
    fn near_critical_controller_rejects_invalid_band() {
        assert_eq!(
            NearCriticalControllerConfig::try_new(900, 800, 1, 1, 1),
            Err(NearCriticalControllerError::InvalidBand)
        );
    }

    #[test]
    fn near_critical_controller_adjusts_below_inside_and_above_band() {
        let opportunity = NearCriticalOpportunityState {
            candidate_forwarding_opportunities: 2,
            payload_bytes_per_opportunity: 32,
        };
        let below =
            decide_near_critical_controller(config(), pressure(200), usage(0, 0, 0), opportunity);
        let inside =
            decide_near_critical_controller(config(), pressure(900), usage(0, 0, 0), opportunity);
        let above =
            decide_near_critical_controller(config(), pressure(1000), usage(0, 0, 0), opportunity);

        assert_eq!(below.mode, NearCriticalControllerMode::IncreaseOpportunity);
        assert_eq!(below.added_opportunities, 1);
        assert_eq!(inside.mode, NearCriticalControllerMode::Steady);
        assert_eq!(above.mode, NearCriticalControllerMode::SuppressForwarding);
    }

    #[test]
    fn near_critical_controller_hard_caps_block_before_band_adjustment() {
        let opportunity = NearCriticalOpportunityState {
            candidate_forwarding_opportunities: 2,
            payload_bytes_per_opportunity: 32,
        };
        for usage in [usage(4, 0, 0), usage(0, 4, 0), usage(0, 0, 128)] {
            let decision =
                decide_near_critical_controller(config(), pressure(200), usage, opportunity);
            assert_eq!(decision.mode, NearCriticalControllerMode::HardCapBlocked);
            assert_eq!(decision.emitted_opportunities, 0);
        }
    }

    #[test]
    fn near_critical_controller_zero_budget_blocks() {
        let config = NearCriticalControllerConfig::try_new(0, 1000, 0, 0, 0).expect("config");
        let decision = decide_near_critical_controller(
            config,
            pressure(0),
            usage(0, 0, 0),
            NearCriticalOpportunityState {
                candidate_forwarding_opportunities: 2,
                payload_bytes_per_opportunity: 32,
            },
        );

        assert_eq!(decision.mode, NearCriticalControllerMode::HardCapBlocked);
    }

    #[test]
    fn near_critical_controller_clamps_added_opportunity_to_remaining_caps() {
        let decision = decide_near_critical_controller(
            config(),
            pressure(200),
            usage(0, 3, 96),
            NearCriticalOpportunityState {
                candidate_forwarding_opportunities: 1,
                payload_bytes_per_opportunity: 32,
            },
        );

        assert_eq!(
            decision.mode,
            NearCriticalControllerMode::IncreaseOpportunity
        );
        assert_eq!(decision.emitted_opportunities, 1);
        assert_eq!(decision.added_opportunities, 0);
    }

    #[test]
    fn near_critical_controller_deterministic_for_equal_inputs() {
        let opportunity = NearCriticalOpportunityState {
            candidate_forwarding_opportunities: 2,
            payload_bytes_per_opportunity: 32,
        };
        let first =
            decide_near_critical_controller(config(), pressure(200), usage(0, 0, 0), opportunity);
        let second =
            decide_near_critical_controller(config(), pressure(200), usage(0, 0, 0), opportunity);

        assert_eq!(first, second);
    }
}
// proc-macro-scope: near-critical controller records are artifact schema, not shared model vocabulary.
