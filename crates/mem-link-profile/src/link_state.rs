use jacquard_core::{
    Belief, DurationMs, Estimate, Link, LinkEndpoint, LinkRuntimeState, LinkState,
    RatioPermille, Tick,
};

/// Builder for one in-memory directed link profile and its current runtime
/// state.
#[derive(Clone, Debug)]
pub struct SimulatedLinkProfile {
    endpoint: LinkEndpoint,
    runtime_state: LinkRuntimeState,
    median_rtt_ms: DurationMs,
    transfer_rate_bytes_per_sec: u32,
    stability_horizon_ms: DurationMs,
    loss_permille: RatioPermille,
    delivery_confidence_permille: RatioPermille,
    symmetry_permille: RatioPermille,
    observed_at_tick: Tick,
}

impl SimulatedLinkProfile {
    #[must_use]
    pub fn new(endpoint: LinkEndpoint) -> Self {
        Self {
            endpoint,
            runtime_state: LinkRuntimeState::Active,
            median_rtt_ms: DurationMs(40),
            transfer_rate_bytes_per_sec: 2048,
            stability_horizon_ms: DurationMs(500),
            loss_permille: RatioPermille(50),
            delivery_confidence_permille: RatioPermille(950),
            symmetry_permille: RatioPermille(900),
            observed_at_tick: Tick(0),
        }
    }

    #[must_use]
    pub fn with_runtime_state(mut self, runtime_state: LinkRuntimeState) -> Self {
        self.runtime_state = runtime_state;
        self
    }

    #[must_use]
    pub fn with_median_rtt(mut self, median_rtt_ms: DurationMs) -> Self {
        self.median_rtt_ms = median_rtt_ms;
        self
    }

    #[must_use]
    pub fn with_transfer_rate(mut self, transfer_rate_bytes_per_sec: u32) -> Self {
        self.transfer_rate_bytes_per_sec = transfer_rate_bytes_per_sec;
        self
    }

    #[must_use]
    pub fn with_stability_horizon(mut self, stability_horizon_ms: DurationMs) -> Self {
        self.stability_horizon_ms = stability_horizon_ms;
        self
    }

    #[must_use]
    pub fn with_loss(mut self, loss_permille: RatioPermille) -> Self {
        self.loss_permille = loss_permille;
        self
    }

    #[must_use]
    pub fn with_delivery_confidence(
        mut self,
        delivery_confidence_permille: RatioPermille,
    ) -> Self {
        self.delivery_confidence_permille = delivery_confidence_permille;
        self
    }

    #[must_use]
    pub fn with_symmetry(mut self, symmetry_permille: RatioPermille) -> Self {
        self.symmetry_permille = symmetry_permille;
        self
    }

    #[must_use]
    pub fn with_observed_at_tick(mut self, observed_at_tick: Tick) -> Self {
        self.observed_at_tick = observed_at_tick;
        self
    }

    #[must_use]
    pub fn build(self) -> Link {
        Link {
            endpoint: self.endpoint,
            state: LinkState {
                state: self.runtime_state,
                median_rtt_ms: self.median_rtt_ms,
                transfer_rate_bytes_per_sec: Belief::Estimated(Estimate {
                    value: self.transfer_rate_bytes_per_sec,
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: self.observed_at_tick,
                }),
                stability_horizon_ms: Belief::Estimated(Estimate {
                    value: self.stability_horizon_ms,
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: self.observed_at_tick,
                }),
                loss_permille: self.loss_permille,
                delivery_confidence_permille: Belief::Estimated(Estimate {
                    value: self.delivery_confidence_permille,
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: self.observed_at_tick,
                }),
                symmetry_permille: Belief::Estimated(Estimate {
                    value: self.symmetry_permille,
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: self.observed_at_tick,
                }),
            },
        }
    }
}
