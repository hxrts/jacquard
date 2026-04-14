//! Route quality scoring for the classic BATMAN engine.
//!
//! In contrast to the enhanced batman engine, this module derives TQ solely
//! from the `LinkRuntimeState` baseline — no Jacquard-specific beliefs
//! (delivery confidence, symmetry, transfer rate, stability horizon) are
//! incorporated. This matches the spec's purely receive-ratio-based quality
//! model where the only per-link input is the coarse reachability state.
//!
//! - `derive_tq` — maps `LinkRuntimeState` to a permille TQ score using the
//!   OGM-equivalent baseline and classifies links below 700 permille as
//!   `Degraded`.
//! - `tq_product` — multiplies two permille values to derive a compound
//!   end-to-end score, matching the classical BATMAN propagation rule.
//! - `ogm_equivalent_tq` — exposed for gossip and private-state callers that
//!   need to score a link without accessing a full `Link` value.

use jacquard_core::{Link, LinkRuntimeState, RatioPermille, RouteDegradation, TransportKind};

/// TQ permille value below which a link is classified as `Degraded`.
pub(crate) const TQ_DEGRADED_BELOW: u16 = 700;

/// Permille scale denominator — the maximum value of a `RatioPermille`.
pub(crate) const PERMILLE_MAX: u32 = 1000;

/// Derive TQ from the OGM-equivalent baseline only, with no Jacquard link
/// belief enrichments.
///
/// Returns `(tq, degradation, transport_kind)`. Links with state `Faulted` or
/// `Suspended` produce TQ values far below the `TQ_DEGRADED_BELOW` floor.
#[must_use]
pub(crate) fn derive_tq(link: &Link) -> (RatioPermille, RouteDegradation, TransportKind) {
    let tq = ogm_equivalent_tq(link.state.state);
    let degradation = if tq.0 < TQ_DEGRADED_BELOW {
        RouteDegradation::Degraded(jacquard_core::DegradationReason::LinkInstability)
    } else {
        RouteDegradation::None
    };
    (tq, degradation, link.endpoint.transport_kind.clone())
}

/// Compound two permille quality scores (classic BATMAN TQ propagation rule).
///
/// `(left * right) / 1000`, saturating at 1000. This is the same formula used
/// by both the spec and the enhanced batman engine.
#[must_use]
pub(crate) fn tq_product(left: RatioPermille, right: RatioPermille) -> RatioPermille {
    let value = (u32::from(left.0) * u32::from(right.0)) / PERMILLE_MAX;
    RatioPermille(u16::try_from(value).expect("permille product"))
}

/// Coarse OGM-equivalent TQ baseline derived from `LinkRuntimeState`.
///
/// These values match the enhanced batman engine's baseline to allow direct
/// score comparison between the two engines. They do not change when Jacquard
/// link beliefs are absent.
#[must_use]
pub(crate) fn ogm_equivalent_tq(state: LinkRuntimeState) -> RatioPermille {
    match state {
        LinkRuntimeState::Active => RatioPermille(900),
        LinkRuntimeState::Degraded => RatioPermille(650),
        LinkRuntimeState::Suspended => RatioPermille(250),
        LinkRuntimeState::Faulted => RatioPermille(0),
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        Belief, ByteCount, DurationMs, EndpointLocator, LinkEndpoint, LinkProfile, LinkState,
        PartitionRecoveryClass, RepairCapability, Tick, TransportKind,
    };

    use super::*;

    fn fixture_link() -> Link {
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
                state: LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::certain(128_000, Tick(1)),
                stability_horizon_ms: Belief::certain(DurationMs(4_000), Tick(1)),
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::certain(RatioPermille(960), Tick(1)),
                symmetry_permille: Belief::certain(RatioPermille(950), Tick(1)),
            },
        }
    }

    #[test]
    fn derive_tq_uses_only_state_baseline_ignoring_richer_beliefs() {
        let (tq, _, _) = derive_tq(&fixture_link());
        // Must equal the plain ogm_equivalent_tq(Active) = 900, not an
        // enriched average incorporating delivery_confidence or symmetry.
        assert_eq!(tq, RatioPermille(900));
    }

    #[test]
    fn tq_product_matches_classical_batman_formula() {
        let a = RatioPermille(900);
        let b = RatioPermille(800);
        // (900 * 800) / 1000 = 720
        assert_eq!(tq_product(a, b), RatioPermille(720));
    }

    #[test]
    fn tq_product_saturates_at_1000() {
        assert_eq!(
            tq_product(RatioPermille(1000), RatioPermille(1000)),
            RatioPermille(1000)
        );
    }
}
