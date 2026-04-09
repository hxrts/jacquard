use jacquard_core::{
    Belief, DurationMs, Link, LinkRuntimeState, RatioPermille, RouteDegradation,
    TransportKind,
};

/// BATMAN-private TQ-like scalar derived from an OGM-equivalent baseline plus
/// optional richer Jacquard link observations.
///
/// Required baseline:
/// - `LinkRuntimeState` as a coarse OGM-equivalent reachability signal
///
/// Optional enrichments when present:
/// - `delivery_confidence_permille`
/// - `symmetry_permille`
/// - `transfer_rate_bytes_per_sec`
/// - `stability_horizon_ms`
#[must_use]
pub(crate) fn derive_tq(
    link: &Link,
) -> (RatioPermille, RouteDegradation, TransportKind) {
    let mut score_total = u32::from(ogm_equivalent_tq(link.state.state).0);
    let mut score_terms = 1_u32;

    if let Some(delivery) = belief_value(&link.state.delivery_confidence_permille) {
        score_total = score_total.saturating_add(u32::from(delivery.0));
        score_terms = score_terms.saturating_add(1);
    }
    if let Some(symmetry) = belief_value(&link.state.symmetry_permille) {
        score_total = score_total.saturating_add(u32::from(symmetry.0));
        score_terms = score_terms.saturating_add(1);
    }
    if let Some(throughput) = normalize_bytes_per_sec(
        &link.state.transfer_rate_bytes_per_sec.value(),
        128_000,
    ) {
        score_total = score_total.saturating_add(throughput);
        score_terms = score_terms.saturating_add(1);
    }
    if let Some(stability) =
        normalize_duration_ms(&link.state.stability_horizon_ms.value(), 4_000)
    {
        score_total = score_total.saturating_add(stability);
        score_terms = score_terms.saturating_add(1);
    }

    let tq = RatioPermille(
        u16::try_from(score_total / score_terms).expect("permille score"),
    );
    let degradation = if tq.0 < 700 {
        RouteDegradation::Degraded(jacquard_core::DegradationReason::LinkInstability)
    } else {
        RouteDegradation::None
    };
    (tq, degradation, link.endpoint.transport_kind.clone())
}

#[must_use]
pub(crate) fn tq_product(left: RatioPermille, right: RatioPermille) -> RatioPermille {
    let value = (u32::from(left.0) * u32::from(right.0)) / 1000;
    RatioPermille(u16::try_from(value).expect("permille product"))
}

fn belief_value(value: &Belief<RatioPermille>) -> Option<RatioPermille> {
    match value {
        | Belief::Absent => None,
        | Belief::Estimated(estimate) => Some(estimate.value),
    }
}

fn normalize_bytes_per_sec(value: &Option<u32>, saturating_at: u32) -> Option<u32> {
    value
        .map(|value| value.saturating_mul(1000) / saturating_at)
        .map(|value| value.min(1000))
}

fn normalize_duration_ms(
    value: &Option<DurationMs>,
    saturating_at: u32,
) -> Option<u32> {
    value
        .map(|value| value.0.saturating_mul(1000) / saturating_at)
        .map(|value| value.min(1000))
}

fn ogm_equivalent_tq(state: LinkRuntimeState) -> RatioPermille {
    match state {
        | LinkRuntimeState::Active => RatioPermille(900),
        | LinkRuntimeState::Degraded => RatioPermille(650),
        | LinkRuntimeState::Suspended => RatioPermille(250),
        | LinkRuntimeState::Faulted => RatioPermille(0),
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        ByteCount, EndpointLocator, LinkEndpoint, LinkProfile, LinkState,
        PartitionRecoveryClass, RepairCapability, Tick, TransportKind,
    };

    use super::*;

    fn link_with_richer_observations(remote: u8, delivery: u16, symmetry: u16) -> Link {
        Link {
            endpoint: LinkEndpoint {
                transport_kind: TransportKind::WifiAware,
                locator: EndpointLocator::Opaque(vec![remote]),
                mtu_bytes: ByteCount(64),
            },
            profile: LinkProfile {
                latency_floor_ms: DurationMs(5),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::certain(128_000, Tick(1)),
                stability_horizon_ms: Belief::certain(DurationMs(4_000), Tick(1)),
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::certain(
                    RatioPermille(delivery),
                    Tick(1),
                ),
                symmetry_permille: Belief::certain(RatioPermille(symmetry), Tick(1)),
            },
        }
    }

    #[test]
    fn tq_derivation_orders_links_deterministically_and_breaks_ties() {
        let higher = derive_tq(&link_with_richer_observations(2, 960, 950)).0;
        let lower = derive_tq(&link_with_richer_observations(3, 800, 790)).0;

        assert!(higher > lower);
        assert!(crate::private_state::better_path(
            Some((lower, 2)),
            higher,
            2
        ));
        assert!(crate::private_state::better_path(
            Some((higher, 3)),
            higher,
            2
        ));
    }

    #[test]
    fn tq_derivation_has_an_ogm_equivalent_baseline_without_richer_beliefs() {
        let link = Link {
            endpoint: LinkEndpoint {
                transport_kind: TransportKind::WifiAware,
                locator: EndpointLocator::Opaque(vec![9]),
                mtu_bytes: ByteCount(64),
            },
            profile: LinkProfile {
                latency_floor_ms: DurationMs(5),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(500),
                delivery_confidence_permille: Belief::Absent,
                symmetry_permille: Belief::Absent,
            },
        };

        let (tq, degradation, _) = derive_tq(&link);
        assert_eq!(tq, RatioPermille(900));
        assert_eq!(degradation, RouteDegradation::None);
    }
}
