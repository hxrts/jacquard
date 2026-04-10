//! Developer-facing in-memory link authoring.
//!
//! Most implementers should start here. This module exposes the intended
//! authoring flow for in-memory links:
//! - choose or construct a shared `LinkEndpoint`
//! - choose a link shape such as active, degraded, lossy, or recoverable
//! - optionally refine latency or repair semantics
//! - build the shared `Link`
//!
//! [`LinkPreset`] keeps [`SimulatedLinkProfile`] as the low-level escape
//! hatch when a test needs exact control over the underlying `LinkProfile` /
//! `LinkState` split. Callers provide shared `LinkEndpoint` values; this crate
//! stays endpoint-agnostic.

use jacquard_core::{
    DurationMs, Link, LinkEndpoint, LinkRuntimeState, PartitionRecoveryClass, RatioPermille,
    RepairCapability, Tick,
};

use crate::{
    SimulatedLinkProfile, DEFAULT_DELIVERY_CONFIDENCE_PERMILLE, DEFAULT_STABILITY_HORIZON_MS,
    REFERENCE_TYPICAL_RTT_MS,
};

/// Default transfer rate used by the reference in-memory link presets.
pub const DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC: u32 = 2048;

/// Typed setup for the common link preset path.
#[derive(Clone, Debug)]
pub struct LinkPresetOptions {
    pub endpoint: LinkEndpoint,
    pub observed_at_tick: Tick,
    pub delivery_confidence_permille: RatioPermille,
}

impl LinkPresetOptions {
    #[must_use]
    pub fn new(endpoint: LinkEndpoint, observed_at_tick: Tick) -> Self {
        Self {
            endpoint,
            observed_at_tick,
            delivery_confidence_permille: DEFAULT_DELIVERY_CONFIDENCE_PERMILLE,
        }
    }

    #[must_use]
    pub fn with_confidence(mut self, delivery_confidence_permille: RatioPermille) -> Self {
        self.delivery_confidence_permille = delivery_confidence_permille;
        self
    }
}

/// Preset-first wrapper around `SimulatedLinkProfile`.
#[derive(Clone, Debug)]
pub struct LinkPreset {
    simulated: SimulatedLinkProfile,
}

impl LinkPreset {
    #[must_use]
    pub fn new(endpoint: LinkEndpoint) -> Self {
        Self {
            simulated: SimulatedLinkProfile::new(endpoint),
        }
    }

    #[must_use]
    pub fn active(options: LinkPresetOptions) -> Self {
        Self::new(options.endpoint).active_at(options.observed_at_tick)
    }

    #[must_use]
    pub fn degraded(options: LinkPresetOptions) -> Self {
        Self::new(options.endpoint).degraded_at(options.observed_at_tick)
    }

    #[must_use]
    pub fn lossy(options: LinkPresetOptions) -> Self {
        let LinkPresetOptions {
            endpoint,
            observed_at_tick,
            delivery_confidence_permille,
        } = options;
        Self::new(endpoint)
            .active_at(observed_at_tick)
            .with_confidence(delivery_confidence_permille)
    }

    #[must_use]
    pub fn recoverable(options: LinkPresetOptions) -> Self {
        Self::active(options)
            .with_repair_capability(RepairCapability::ApplicationRetransmit)
            .with_partition_recovery(PartitionRecoveryClass::EndToEndRecoverable)
    }

    #[must_use]
    pub fn active_at(mut self, observed_at_tick: Tick) -> Self {
        self.simulated = self
            .simulated
            .with_runtime_state(LinkRuntimeState::Active)
            .with_runtime_observation(
                REFERENCE_TYPICAL_RTT_MS,
                DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC,
                DEFAULT_STABILITY_HORIZON_MS,
                observed_at_tick,
            );
        self
    }

    #[must_use]
    pub fn degraded_at(mut self, observed_at_tick: Tick) -> Self {
        self.simulated = self
            .simulated
            .with_runtime_state(LinkRuntimeState::Degraded)
            .with_runtime_observation(
                REFERENCE_TYPICAL_RTT_MS,
                DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC,
                DEFAULT_STABILITY_HORIZON_MS,
                observed_at_tick,
            );
        self
    }

    #[must_use]
    pub fn with_confidence(mut self, delivery_confidence_permille: RatioPermille) -> Self {
        self.simulated = self.simulated.with_quality(
            RatioPermille(50),
            delivery_confidence_permille,
            RatioPermille(900),
        );
        self
    }

    #[must_use]
    pub fn with_latency(mut self, latency_floor_ms: DurationMs) -> Self {
        self.simulated = self.simulated.with_latency_floor(latency_floor_ms);
        self
    }

    #[must_use]
    pub fn with_repair_capability(mut self, repair_capability: RepairCapability) -> Self {
        self.simulated = self.simulated.with_repair_capability(repair_capability);
        self
    }

    #[must_use]
    pub fn with_partition_recovery(mut self, partition_recovery: PartitionRecoveryClass) -> Self {
        self.simulated = self.simulated.with_partition_recovery(partition_recovery);
        self
    }

    #[must_use]
    pub fn into_simulated(self) -> SimulatedLinkProfile {
        self.simulated
    }

    #[must_use]
    pub fn build(self) -> Link {
        self.simulated.build()
    }
}

#[cfg(test)]
mod tests {
    use jacquard_adapter::opaque_endpoint;
    use jacquard_core::{ByteCount, LinkEndpoint, TransportKind};

    use super::*;

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
    }

    #[test]
    fn lossy_matches_expected_delivery_confidence() {
        let link = LinkPreset::lossy(
            LinkPresetOptions::new(endpoint(7), Tick(3)).with_confidence(RatioPermille(650)),
        )
        .build();

        assert_eq!(
            link.state
                .delivery_confidence_permille
                .value_or(RatioPermille(0)),
            RatioPermille(650)
        );
        assert_eq!(link.state.state, LinkRuntimeState::Active);
    }

    #[test]
    fn recoverable_upgrades_repair_defaults() {
        let link = LinkPreset::recoverable(LinkPresetOptions::new(endpoint(9), Tick(4))).build();

        assert_eq!(
            link.profile.repair_capability,
            RepairCapability::ApplicationRetransmit,
        );
        assert_eq!(
            link.profile.partition_recovery,
            PartitionRecoveryClass::EndToEndRecoverable,
        );
    }

    #[test]
    fn endpoint_first_active_constructor_preserves_endpoint_identity() {
        let endpoint = opaque_endpoint(TransportKind::WifiAware, vec![1, 2, 3], ByteCount(128));
        let link = LinkPreset::active(LinkPresetOptions::new(endpoint.clone(), Tick(2))).build();

        assert_eq!(link.endpoint, endpoint);
        assert_eq!(link.state.state, LinkRuntimeState::Active);
    }
}
