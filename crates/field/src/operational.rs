//! Reduced operational view for Field decision code.
//!
//! This stays below posterior truth and canonical route truth. It is a cheap,
//! deterministic classification layer over already-owned runtime state.

use jacquard_core::Tick;

use crate::{
    route::FieldContinuityBand,
    state::{DestinationFieldState, DestinationKey},
    FieldSearchConfig,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldDestinationClass {
    Node,
    Gateway,
    Service,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldDestinationDecisionContext {
    pub(crate) destination: DestinationKey,
    pub(crate) destination_class: FieldDestinationClass,
    pub(crate) node_discovery_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldRuntimeDecisionContext {
    pub(crate) service_bias: bool,
    pub(crate) discovery_node_route: bool,
    pub(crate) degraded_band: bool,
    pub(crate) bootstrap_band: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SupportBand {
    Weak,
    Emerging,
    Stable,
    Strong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RetentionBand {
    Low,
    Useful,
    Strong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EntropyBand {
    Low,
    Medium,
    High,
    Severe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FreshnessClass {
    Fresh,
    Aging,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldOperationalView {
    pub(crate) service_bias: bool,
    pub(crate) support_permille: u16,
    pub(crate) retention_permille: u16,
    pub(crate) entropy_permille: u16,
    pub(crate) top_mass_permille: u16,
    pub(crate) support_band: SupportBand,
    pub(crate) retention_band: RetentionBand,
    pub(crate) entropy_band: EntropyBand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldRouteOperationalView {
    pub(crate) freshness_class: FreshnessClass,
    pub(crate) freshness_age_ticks: u64,
}

impl FieldDestinationDecisionContext {
    #[must_use]
    pub(crate) fn new(destination: &DestinationKey, search_config: &FieldSearchConfig) -> Self {
        let destination_class = match destination {
            DestinationKey::Node(_) => FieldDestinationClass::Node,
            DestinationKey::Gateway(_) => FieldDestinationClass::Gateway,
            DestinationKey::Service(_) => FieldDestinationClass::Service,
        };
        Self {
            destination: destination.clone(),
            destination_class,
            node_discovery_enabled: search_config.node_discovery_enabled(),
        }
    }

    #[must_use]
    pub(crate) fn service_bias(&self) -> bool {
        self.destination_class == FieldDestinationClass::Service
    }

    #[must_use]
    pub(crate) fn discovery_node_route(&self) -> bool {
        self.destination_class == FieldDestinationClass::Node && self.node_discovery_enabled
    }
}

impl FieldRuntimeDecisionContext {
    #[must_use]
    pub(crate) fn new(
        destination: &FieldDestinationDecisionContext,
        continuity_band: FieldContinuityBand,
    ) -> Self {
        Self {
            service_bias: destination.service_bias(),
            discovery_node_route: destination.discovery_node_route(),
            degraded_band: continuity_band == FieldContinuityBand::DegradedSteady,
            bootstrap_band: continuity_band == FieldContinuityBand::Bootstrap,
        }
    }
}

#[must_use]
pub(crate) fn destination_operational_view(
    destination_state: &DestinationFieldState,
) -> FieldOperationalView {
    let support_permille = destination_state.corridor_belief.delivery_support.value();
    let retention_permille = destination_state.corridor_belief.retention_affinity.value();
    let entropy_permille = destination_state.posterior.usability_entropy.value();
    let top_mass_permille = destination_state.posterior.top_corridor_mass.value();
    FieldOperationalView {
        service_bias: matches!(destination_state.destination, DestinationKey::Service(_)),
        support_permille,
        retention_permille,
        entropy_permille,
        top_mass_permille,
        support_band: classify_support(support_permille),
        retention_band: classify_retention(retention_permille),
        entropy_band: classify_entropy(entropy_permille),
    }
}

#[must_use]
pub(crate) fn route_operational_view(
    now_tick: Tick,
    neighbor_freshness: Tick,
) -> FieldRouteOperationalView {
    let freshness_age_ticks = now_tick.0.saturating_sub(neighbor_freshness.0);
    FieldRouteOperationalView {
        freshness_class: classify_freshness(freshness_age_ticks),
        freshness_age_ticks,
    }
}

#[must_use]
pub(crate) fn classify_support(value: u16) -> SupportBand {
    match value {
        0..=179 => SupportBand::Weak,
        180..=259 => SupportBand::Emerging,
        260..=319 => SupportBand::Stable,
        _ => SupportBand::Strong,
    }
}

#[must_use]
pub(crate) fn classify_retention(value: u16) -> RetentionBand {
    match value {
        0..=179 => RetentionBand::Low,
        180..=279 => RetentionBand::Useful,
        _ => RetentionBand::Strong,
    }
}

#[must_use]
pub(crate) fn classify_entropy(value: u16) -> EntropyBand {
    match value {
        0..=249 => EntropyBand::Low,
        250..=599 => EntropyBand::Medium,
        600..=849 => EntropyBand::High,
        _ => EntropyBand::Severe,
    }
}

#[must_use]
pub(crate) fn classify_freshness(age_ticks: u64) -> FreshnessClass {
    match age_ticks {
        0..=2 => FreshnessClass::Fresh,
        3..=4 => FreshnessClass::Aging,
        _ => FreshnessClass::Stale,
    }
}

#[cfg(test)]
mod tests {
    use std::{hint::black_box, mem::size_of};

    use jacquard_core::{DestinationId, NodeId, Tick};

    use super::*;
    use crate::state::{DestinationFieldState, DestinationKey, EntropyBucket, SupportBucket};

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    #[test]
    fn destination_operational_view_classifies_bands_deterministically() {
        let mut state = DestinationFieldState::new(
            DestinationKey::from(&DestinationId::Service(jacquard_core::ServiceId(vec![
                7;
                16
            ]))),
            Tick(1),
        );
        state.corridor_belief.delivery_support = SupportBucket::new(310);
        state.corridor_belief.retention_affinity = SupportBucket::new(290);
        state.posterior.usability_entropy = EntropyBucket::new(610);
        state.posterior.top_corridor_mass = SupportBucket::new(330);

        let view = destination_operational_view(&state);
        assert!(view.service_bias);
        assert_eq!(view.support_band, SupportBand::Stable);
        assert_eq!(view.retention_band, RetentionBand::Strong);
        assert_eq!(view.entropy_band, EntropyBand::High);
    }

    #[test]
    fn route_operational_view_uses_small_freshness_classes() {
        let fresh = route_operational_view(Tick(10), Tick(9));
        let aging = route_operational_view(Tick(10), Tick(7));
        let stale = route_operational_view(Tick(10), Tick(4));

        assert_eq!(fresh.freshness_class, FreshnessClass::Fresh);
        assert_eq!(aging.freshness_class, FreshnessClass::Aging);
        assert_eq!(stale.freshness_class, FreshnessClass::Stale);
        assert_eq!(stale.freshness_age_ticks, 6);
    }

    #[test]
    fn support_band_thresholds_stay_stable() {
        assert_eq!(classify_support(179), SupportBand::Weak);
        assert_eq!(classify_support(180), SupportBand::Emerging);
        assert_eq!(classify_support(260), SupportBand::Stable);
        assert_eq!(classify_support(320), SupportBand::Strong);
        assert_eq!(classify_retention(279), RetentionBand::Useful);
        assert_eq!(classify_entropy(849), EntropyBand::High);
        assert_eq!(classify_entropy(850), EntropyBand::Severe);
        assert_eq!(node(1), node(1));
    }

    #[test]
    fn operational_views_stay_small_for_hot_path_reuse() {
        assert!(size_of::<FieldOperationalView>() <= 16);
        assert!(size_of::<FieldRouteOperationalView>() <= 16);
    }

    #[test]
    #[ignore = "manual hot-path benchmark"]
    fn hot_path_operational_view_benchmark() {
        let mut state = DestinationFieldState::new(
            DestinationKey::from(&DestinationId::Node(node(9))),
            Tick(1),
        );
        state.corridor_belief.delivery_support = SupportBucket::new(310);
        state.corridor_belief.retention_affinity = SupportBucket::new(290);
        state.posterior.usability_entropy = EntropyBucket::new(610);
        state.posterior.top_corridor_mass = SupportBucket::new(330);

        for _ in 0..10_000 {
            black_box(destination_operational_view(black_box(&state)));
            black_box(route_operational_view(Tick(10), Tick(7)));
        }
    }
}
