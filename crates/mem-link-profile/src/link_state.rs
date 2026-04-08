use jacquard_core::{
    Belief, DurationMs, Estimate, Link, LinkEndpoint, LinkProfile, LinkRuntimeState,
    LinkState, PartitionRecoveryClass, RatioPermille, RepairCapability, Tick,
};

/// BLE latency floor (minimum one-way latency for a BLE GATT link).
pub const BLE_LATENCY_FLOOR_MS: DurationMs = DurationMs(8);
/// Typical round-trip time for a BLE GATT link.
pub const BLE_TYPICAL_RTT_MS: DurationMs = DurationMs(40);
/// Default stability horizon used when no better estimate is available.
pub const DEFAULT_STABILITY_HORIZON_MS: DurationMs = DurationMs(500);

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
            loss_permille: RatioPermille(50),
            delivery_confidence_permille: RatioPermille(950),
            symmetry_permille: RatioPermille(900),
            observed_at_tick: Tick(0),
        }
    }

    #[must_use]
    pub fn with_latency_floor(mut self, latency_floor_ms: DurationMs) -> Self {
        self.latency_floor_ms = latency_floor_ms;
        self
    }

    #[must_use]
    pub fn with_repair_capability(
        mut self,
        repair_capability: RepairCapability,
    ) -> Self {
        self.repair_capability = repair_capability;
        self
    }

    #[must_use]
    pub fn with_partition_recovery(
        mut self,
        partition_recovery: PartitionRecoveryClass,
    ) -> Self {
        self.partition_recovery = partition_recovery;
        self
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
            profile: LinkProfile {
                latency_floor_ms: self.latency_floor_ms,
                repair_capability: self.repair_capability,
                partition_recovery: self.partition_recovery,
            },
            state: LinkState {
                state: self.runtime_state,
                median_rtt_ms: Belief::Estimated(Estimate {
                    value: self.median_rtt_ms,
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: self.observed_at_tick,
                }),
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

#[cfg(test)]
mod tests {
    use jacquard_core::{ByteCount, EndpointAddress, TransportProtocol};

    use super::*;

    #[test]
    fn build_preserves_profile_and_state_split() {
        let link = SimulatedLinkProfile::new(LinkEndpoint {
            protocol: TransportProtocol::WifiAware,
            address: EndpointAddress::Opaque(vec![1, 2, 3]),
            mtu_bytes: ByteCount(512),
        })
        .with_latency_floor(DurationMs(3))
        .with_repair_capability(RepairCapability::ApplicationRetransmit)
        .with_partition_recovery(PartitionRecoveryClass::EndToEndRecoverable)
        .with_median_rtt(DurationMs(9))
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
