//! Route quality scoring for the Babel engine.
//!
//! ## ETX formula
//!
//! Babel uses the Expected Transmission Count (ETX) formula for link cost:
//!
//!   `cost = 256 * 1_000_000 / (fwd_delivery_permille * rev_delivery_permille)`
//!
//! This captures asymmetric links better than the BATMAN TQ approach because it
//! incorporates BOTH directions of a link. For a perfectly symmetric active link
//! (1000‰ in both directions): `cost = 256 * 1_000_000 / (1_000 * 1_000) = 256`.
//!
//! For an asymmetric link where the forward direction is good (980‰) but the
//! reverse is poor (300‰), the cost rises sharply:
//! `cost ≈ 256 * 1_000_000 / (980 * 300) ≈ 871`. This is penalised more than
//! the symmetric equivalent because the poor reverse path means ACKs are likely
//! lost, increasing the true retransmission count. Classic BATMAN would score
//! this path using only the forward link TQ, underestimating the true cost.
//!
//! If either direction is absent or Faulted (delivery=0), cost = `BABEL_INFINITY`
//! (0xFFFF), making the route unusable. This replaces the echo-window
//! bidirectionality gate used by batman-classic.

use jacquard_core::{Link, LinkRuntimeState, RatioPermille, RouteDegradation};

use crate::gossip::BABEL_INFINITY;

/// Metric value below which a route is classified as degraded.
pub(crate) const METRIC_DEGRADED_AT: u16 = 512;

/// Permille scale denominator — the maximum value of a `RatioPermille`.
pub(crate) const PERMILLE_MAX: u32 = 1000;

/// Metric saturation cap: metric values at or above this map to quality 0.
const METRIC_SATURATION: u32 = 1024;

/// Compute the bidirectional ETX link cost.
///
/// Uses both forward and reverse delivery to capture asymmetric link cost.
/// Returns `BABEL_INFINITY` if either direction is absent or has zero delivery.
#[must_use]
pub(crate) fn link_cost(link_fwd: Option<&Link>, link_rev: Option<&Link>) -> u16 {
    let fwd = delivery_permille(link_fwd);
    let rev = delivery_permille(link_rev);
    if fwd == 0 || rev == 0 {
        return BABEL_INFINITY;
    }
    // ETX: 256 * 1_000_000 / (fwd * rev), using u64 to avoid overflow.
    let numerator: u64 = 256 * 1_000_000;
    let denominator = u64::from(fwd) * u64::from(rev);
    let cost = numerator / denominator;
    // Clamp to BABEL_INFINITY - 1 (not BABEL_INFINITY which means unreachable).
    #[allow(clippy::cast_possible_truncation)]
    u16::try_from(cost.min(u64::from(BABEL_INFINITY) - 1)).unwrap_or(BABEL_INFINITY - 1)
}

/// Compound link cost and a neighbor's reported metric (additive, not multiplicative).
///
/// Returns `BABEL_INFINITY` if either input equals `BABEL_INFINITY`. Otherwise
/// adds cost and neighbor_metric, saturating at `BABEL_INFINITY - 1`.
#[must_use]
pub(crate) fn compound_metric(cost: u16, neighbor_metric: u16) -> u16 {
    if cost >= BABEL_INFINITY || neighbor_metric >= BABEL_INFINITY {
        return BABEL_INFINITY;
    }
    cost.saturating_add(neighbor_metric).min(BABEL_INFINITY - 1)
}

/// Convert a Babel metric to a `RatioPermille` quality score.
///
/// metric=0 → 1000 (perfect); metric>=`METRIC_SATURATION` or `BABEL_INFINITY` → 0.
#[must_use]
pub(crate) fn metric_to_ratio(metric: u16) -> RatioPermille {
    if metric >= BABEL_INFINITY {
        return RatioPermille(0);
    }
    let m = u32::from(metric);
    if m >= METRIC_SATURATION {
        return RatioPermille(0);
    }
    // Linear mapping: 0 → 1000, METRIC_SATURATION-1 → ~1.
    let quality = (METRIC_SATURATION - m) * PERMILLE_MAX / METRIC_SATURATION;
    #[allow(clippy::cast_possible_truncation)]
    RatioPermille(u16::try_from(quality.min(u32::from(PERMILLE_MAX))).unwrap_or(0))
}

/// Classify a Babel metric as degraded or nominal.
#[must_use]
pub(crate) fn metric_degradation(metric: u16) -> RouteDegradation {
    if metric >= METRIC_DEGRADED_AT {
        RouteDegradation::Degraded(jacquard_core::DegradationReason::LinkInstability)
    } else {
        RouteDegradation::None
    }
}

/// Extract the delivery permille from a link's state.
///
/// Returns 0 for Faulted (or absent), 250 for Suspended, and the stored
/// delivery_confidence (or a default) for Degraded and Active.
fn delivery_permille(link: Option<&Link>) -> u16 {
    let Some(link) = link else {
        return 0;
    };
    match link.state.state {
        LinkRuntimeState::Faulted => 0,
        LinkRuntimeState::Suspended => 250,
        LinkRuntimeState::Degraded => link
            .state
            .delivery_confidence_permille
            .value_or(RatioPermille(500))
            .0,
        LinkRuntimeState::Active => link
            .state
            .delivery_confidence_permille
            .value_or(RatioPermille(900))
            .0,
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use jacquard_core::{
        Belief, ByteCount, DurationMs, EndpointLocator, LinkEndpoint, LinkProfile, LinkState,
        PartitionRecoveryClass, RatioPermille, RepairCapability, Tick, TransportKind,
    };

    use super::*;

    fn fixture_link(state: LinkRuntimeState, confidence_permille: u16) -> Link {
        Link {
            endpoint: LinkEndpoint {
                transport_kind: TransportKind::WifiAware,
                locator: EndpointLocator::Opaque(vec![2]),
                mtu_bytes: ByteCount(64),
            },
            profile: LinkProfile {
                latency_floor_ms: DurationMs(5),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::certain(128_000, Tick(1)),
                stability_horizon_ms: Belief::certain(DurationMs(4_000), Tick(1)),
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::certain(
                    RatioPermille(confidence_permille),
                    Tick(1),
                ),
                symmetry_permille: Belief::certain(RatioPermille(950), Tick(1)),
            },
        }
    }

    #[test]
    fn symmetric_active_link_yields_low_cost() {
        let fwd = fixture_link(LinkRuntimeState::Active, 1000);
        let rev = fixture_link(LinkRuntimeState::Active, 1000);
        // 256 * 1_000_000 / (1000 * 1000) = 256
        assert_eq!(link_cost(Some(&fwd), Some(&rev)), 256);
    }

    #[test]
    fn asymmetric_link_costs_more_than_symmetric_equivalent() {
        let fwd = fixture_link(LinkRuntimeState::Active, 980);
        let rev = fixture_link(LinkRuntimeState::Degraded, 300);
        let symmetric = fixture_link(LinkRuntimeState::Active, 640);
        // asymmetric fwd=980, rev=300 should cost more than symmetric 640/640
        let asymmetric_cost = link_cost(Some(&fwd), Some(&rev));
        let sym_cost = link_cost(Some(&symmetric), Some(&symmetric));
        assert!(
            asymmetric_cost > sym_cost,
            "asymmetric cost {asymmetric_cost} should exceed symmetric {sym_cost}"
        );
    }

    #[test]
    fn absent_reverse_link_yields_infinity() {
        let fwd = fixture_link(LinkRuntimeState::Active, 1000);
        assert_eq!(link_cost(Some(&fwd), None), BABEL_INFINITY);
    }

    #[test]
    fn compound_metric_is_additive() {
        assert_eq!(compound_metric(256, 256), 512);
        assert_eq!(compound_metric(0, 0), 0);
    }

    #[test]
    fn metric_to_ratio_maps_zero_to_perfect() {
        assert_eq!(metric_to_ratio(0), RatioPermille(1000));
    }

    #[test]
    fn metric_to_ratio_maps_infinity_to_zero() {
        assert_eq!(metric_to_ratio(BABEL_INFINITY), RatioPermille(0));
    }
}
