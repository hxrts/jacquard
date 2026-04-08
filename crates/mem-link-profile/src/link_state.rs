//! `SimulatedLinkProfile`, a builder for a shared `Link` value plus the
//! BLE-GATT link-level defaults (`BLE_LATENCY_FLOOR_MS`, `BLE_TYPICAL_RTT_MS`,
//! `DEFAULT_STABILITY_HORIZON_MS`) used as seeds by tests and fixtures.

use jacquard_core::{
    Belief, DurationMs, Link, LinkEndpoint, LinkProfile, LinkRuntimeState, LinkState,
    PartitionRecoveryClass, RatioPermille, RepairCapability, Tick,
};

/// BLE latency floor (minimum one-way latency for a BLE GATT link).
pub const BLE_LATENCY_FLOOR_MS: DurationMs = DurationMs(8);
/// Typical round-trip time for a BLE GATT link.
pub const BLE_TYPICAL_RTT_MS: DurationMs = DurationMs(40);
/// Default stability horizon used when no better estimate is available.
pub const DEFAULT_STABILITY_HORIZON_MS: DurationMs = DurationMs(500);
/// Default loss rate for an active in-memory link.
pub(crate) const DEFAULT_LOSS_PERMILLE: RatioPermille = RatioPermille(50);
/// Default delivery confidence for an active in-memory link.
pub(crate) const DEFAULT_DELIVERY_CONFIDENCE_PERMILLE: RatioPermille =
    RatioPermille(950);
/// Default symmetry for an active in-memory link.
pub(crate) const DEFAULT_SYMMETRY_PERMILLE: RatioPermille = RatioPermille(900);

/// Builder for one in-memory directed link profile and its initial runtime
/// state.
///
/// The name stays intentionally short because it models one simulated directed
/// link end to end: endpoint identity, stable `LinkProfile`, and initial
/// `LinkState`.
#[derive(Clone, Debug)]
pub struct SimulatedLinkProfile {
    endpoint: LinkEndpoint,
    latency_floor_ms: DurationMs,
    repair_capability: RepairCapability,
    partition_recovery: PartitionRecoveryClass,
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
            latency_floor_ms: BLE_LATENCY_FLOOR_MS,
            repair_capability: RepairCapability::TransportRetransmit,
            partition_recovery: PartitionRecoveryClass::LocalReconnect,
            runtime_state: LinkRuntimeState::Active,
            median_rtt_ms: BLE_TYPICAL_RTT_MS,
            transfer_rate_bytes_per_sec: 2048,
            stability_horizon_ms: DEFAULT_STABILITY_HORIZON_MS,
            loss_permille: DEFAULT_LOSS_PERMILLE,
            delivery_confidence_permille: DEFAULT_DELIVERY_CONFIDENCE_PERMILLE,
            symmetry_permille: DEFAULT_SYMMETRY_PERMILLE,
            observed_at_tick: Tick(0),
        }
    }

    #[must_use]
    pub fn with_profile(
        mut self,
        latency_floor_ms: DurationMs,
        repair_capability: RepairCapability,
        partition_recovery: PartitionRecoveryClass,
    ) -> Self {
        self.latency_floor_ms = latency_floor_ms;
        self.repair_capability = repair_capability;
        self.partition_recovery = partition_recovery;
        self
    }

    #[must_use]
    pub fn with_runtime_state(mut self, runtime_state: LinkRuntimeState) -> Self {
        self.runtime_state = runtime_state;
        self
    }

    #[must_use]
    pub fn with_quality(
        mut self,
        loss_permille: RatioPermille,
        delivery_confidence_permille: RatioPermille,
        symmetry_permille: RatioPermille,
    ) -> Self {
        self.loss_permille = loss_permille;
        self.delivery_confidence_permille = delivery_confidence_permille;
        self.symmetry_permille = symmetry_permille;
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
        self.median_rtt_ms = median_rtt_ms;
        self.transfer_rate_bytes_per_sec = transfer_rate_bytes_per_sec;
        self.stability_horizon_ms = stability_horizon_ms;
        self.observed_at_tick = observed_at_tick;
        self
    }

    #[must_use]
    pub fn active(endpoint: LinkEndpoint, observed_at_tick: Tick) -> Self {
        Self::new(endpoint).with_runtime_observation(
            BLE_TYPICAL_RTT_MS,
            2048,
            DEFAULT_STABILITY_HORIZON_MS,
            observed_at_tick,
        )
    }

    #[must_use]
    pub fn active_with_confidence(
        endpoint: LinkEndpoint,
        delivery_confidence_permille: RatioPermille,
        observed_at_tick: Tick,
    ) -> Self {
        Self::active(endpoint, observed_at_tick).with_quality(
            DEFAULT_LOSS_PERMILLE,
            delivery_confidence_permille,
            DEFAULT_SYMMETRY_PERMILLE,
        )
    }

    #[must_use]
    pub fn build(self) -> Link {
        Link {
            endpoint: self.endpoint,
            profile: LinkProfile {
                latency_floor_ms: self.latency_floor_ms,
                repair_capability: self.repair_capability,
                partition_recovery: self.partition_recovery,
            },
            state: LinkState {
                state: self.runtime_state,
                median_rtt_ms: Belief::certain(
                    self.median_rtt_ms,
                    self.observed_at_tick,
                ),
                transfer_rate_bytes_per_sec: Belief::certain(
                    self.transfer_rate_bytes_per_sec,
                    self.observed_at_tick,
                ),
                stability_horizon_ms: Belief::certain(
                    self.stability_horizon_ms,
                    self.observed_at_tick,
                ),
                loss_permille: self.loss_permille,
                delivery_confidence_permille: Belief::certain(
                    self.delivery_confidence_permille,
                    self.observed_at_tick,
                ),
                symmetry_permille: Belief::certain(
                    self.symmetry_permille,
                    self.observed_at_tick,
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{ByteCount, EndpointAddress, Estimate, TransportProtocol};

    use super::*;

    #[test]
    fn build_preserves_profile_and_state_split() {
        let link = SimulatedLinkProfile::new(LinkEndpoint {
            protocol: TransportProtocol::WifiAware,
            address: EndpointAddress::Opaque(vec![1, 2, 3]),
            mtu_bytes: ByteCount(512),
        })
        .with_profile(
            DurationMs(3),
            RepairCapability::ApplicationRetransmit,
            PartitionRecoveryClass::EndToEndRecoverable,
        )
        .with_runtime_observation(
            DurationMs(9),
            2048,
            DEFAULT_STABILITY_HORIZON_MS,
            Tick(0),
        )
        .build();

        assert_eq!(link.profile.latency_floor_ms, DurationMs(3));
        assert_eq!(
            link.profile.repair_capability,
            RepairCapability::ApplicationRetransmit,
        );
        assert_eq!(
            link.profile.partition_recovery,
            PartitionRecoveryClass::EndToEndRecoverable,
        );
        assert_eq!(
            link.state.median_rtt_ms,
            Belief::Estimated(Estimate {
                value: DurationMs(9),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(0),
            })
        );
    }
}
