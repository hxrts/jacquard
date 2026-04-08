//! Developer-facing in-memory link authoring.
//!
//! Most implementers should start here. This module exposes the intended
//! authoring flow for in-memory links:
//! - choose an endpoint shape such as BLE or opaque
//! - choose a link shape such as active, degraded, lossy, or recoverable
//! - optionally refine latency or repair semantics
//! - build the shared `Link`
//!
//! [`ReferenceLink`] keeps [`SimulatedLinkProfile`] as the low-level escape
//! hatch when a test needs exact control over the underlying `LinkProfile` /
//! `LinkState` split.

use jacquard_core::{
    ByteCount, DurationMs, Link, LinkRuntimeState, PartitionRecoveryClass,
    RatioPermille, RepairCapability, Tick, TransportProtocol,
};

use crate::{
    ble_endpoint, opaque_endpoint, SimulatedLinkProfile, BLE_TYPICAL_RTT_MS,
    DEFAULT_STABILITY_HORIZON_MS,
};

/// Default transfer rate used by the reference in-memory link presets.
pub const DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC: u32 = 2048;

/// Preset-first wrapper around `SimulatedLinkProfile`.
#[derive(Clone, Debug)]
pub struct ReferenceLink {
    simulated: SimulatedLinkProfile,
}

impl ReferenceLink {
    #[must_use]
    pub fn ble(device_byte: u8) -> Self {
        Self {
            simulated: SimulatedLinkProfile::new(ble_endpoint(device_byte)),
        }
    }

    #[must_use]
    pub fn opaque(protocol: TransportProtocol, bytes: Vec<u8>, mtu: ByteCount) -> Self {
        Self {
            simulated: SimulatedLinkProfile::new(opaque_endpoint(protocol, bytes, mtu)),
        }
    }

    #[must_use]
    pub fn ble_active(device_byte: u8, observed_at_tick: Tick) -> Self {
        Self::ble(device_byte).active_at(observed_at_tick)
    }

    #[must_use]
    pub fn ble_degraded(device_byte: u8, observed_at_tick: Tick) -> Self {
        Self::ble(device_byte).degraded_at(observed_at_tick)
    }

    #[must_use]
    pub fn ble_lossy(
        device_byte: u8,
        delivery_confidence_permille: RatioPermille,
        observed_at_tick: Tick,
    ) -> Self {
        Self::ble_active(device_byte, observed_at_tick)
            .with_confidence(delivery_confidence_permille)
    }

    #[must_use]
    pub fn ble_recoverable(device_byte: u8, observed_at_tick: Tick) -> Self {
        Self::ble_active(device_byte, observed_at_tick)
            .with_repair_capability(RepairCapability::ApplicationRetransmit)
            .with_partition_recovery(PartitionRecoveryClass::EndToEndRecoverable)
    }

    #[must_use]
    pub fn active_at(mut self, observed_at_tick: Tick) -> Self {
        self.simulated = self
            .simulated
            .with_runtime_state(LinkRuntimeState::Active)
            .with_runtime_observation(
                BLE_TYPICAL_RTT_MS,
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
                BLE_TYPICAL_RTT_MS,
                DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC,
                DEFAULT_STABILITY_HORIZON_MS,
                observed_at_tick,
            );
        self
    }

    #[must_use]
    pub fn with_confidence(
        mut self,
        delivery_confidence_permille: RatioPermille,
    ) -> Self {
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
    pub fn with_repair_capability(
        mut self,
        repair_capability: RepairCapability,
    ) -> Self {
        self.simulated = self.simulated.with_repair_capability(repair_capability);
        self
    }

    #[must_use]
    pub fn with_partition_recovery(
        mut self,
        partition_recovery: PartitionRecoveryClass,
    ) -> Self {
        self.simulated = self.simulated.with_partition_recovery(partition_recovery);
        self
    }

    #[must_use]
    pub fn with_profile(
        mut self,
        latency_floor_ms: DurationMs,
        repair_capability: RepairCapability,
        partition_recovery: PartitionRecoveryClass,
    ) -> Self {
        self.simulated = self.simulated.with_profile(
            latency_floor_ms,
            repair_capability,
            partition_recovery,
        );
        self
    }

    #[must_use]
    pub fn with_runtime_observation(
        mut self,
        median_rtt_ms: DurationMs,
        transfer_rate_bytes_per_sec: u32,
        stability_horizon_ms: DurationMs,
        observed_at_tick: Tick,
    ) -> Self {
        self.simulated = self.simulated.with_runtime_observation(
            median_rtt_ms,
            transfer_rate_bytes_per_sec,
            stability_horizon_ms,
            observed_at_tick,
        );
        self
    }

    #[must_use]
    pub fn with_quality(
        mut self,
        loss_permille: RatioPermille,
        delivery_confidence_permille: RatioPermille,
        symmetry_permille: RatioPermille,
    ) -> Self {
        self.simulated = self.simulated.with_quality(
            loss_permille,
            delivery_confidence_permille,
            symmetry_permille,
        );
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
    use super::*;

    #[test]
    fn ble_lossy_matches_expected_delivery_confidence() {
        let link = ReferenceLink::ble_lossy(7, RatioPermille(650), Tick(3)).build();

        assert_eq!(
            link.state
                .delivery_confidence_permille
                .value_or(RatioPermille(0)),
            RatioPermille(650)
        );
        assert_eq!(link.state.state, LinkRuntimeState::Active);
    }

    #[test]
    fn ble_recoverable_upgrades_repair_defaults() {
        let link = ReferenceLink::ble_recoverable(9, Tick(4)).build();

        assert_eq!(
            link.profile.repair_capability,
            RepairCapability::ApplicationRetransmit,
        );
        assert_eq!(
            link.profile.partition_recovery,
            PartitionRecoveryClass::EndToEndRecoverable,
        );
    }
}
