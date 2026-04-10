//! Bounded state containers for the field routing engine.
//!
//! The `bounded_bucket!` macro generates `EntropyBucket`, `SupportBucket`,
//! `DivergenceBucket`, and `ResidualBucket` as clamped u16 values with a 1000
//! permille ceiling. `OperatingRegime` (Sparse, Congested, RetentionFavorable,
//! Unstable, Adversarial) and `RoutingPosture` (Opportunistic, Structured,
//! RetentionBiased, RiskSuppressed) classify the current network environment.
//! `DestinationFieldState` holds per-destination corridor belief, posterior,
//! progress belief, and frontier neighbors. `FieldEngineState` aggregates all
//! destination state under hard size limits: 32 tracked destinations, 8 active,
//! 4 frontier slots per destination, and 3 alternates per route.

#![expect(
    dead_code,
    reason = "phase-3 bounded field state is integrated incrementally across later phases"
)]

use std::collections::BTreeMap;

use jacquard_core::{
    Belief, DestinationId, GatewayId, HealthScore, LinkEndpoint, NodeId, PenaltyPoints, RouteEpoch,
    ServiceId, Tick, TimeWindow, ROUTE_HOP_COUNT_MAX,
};

use crate::summary::FieldSummary;

pub(crate) const MAX_TRACKED_DESTINATIONS: usize = 32;
pub(crate) const MAX_ACTIVE_DESTINATIONS: usize = 8;
pub(crate) const MAX_FRONTIER_SIZE: usize = 4;
pub(crate) const MAX_ALTERNATE_COUNT: usize = MAX_FRONTIER_SIZE.saturating_sub(1);
pub(crate) const OBSERVER_CACHE_REFRESH_TICKS: u64 = 2;
pub(crate) const SUMMARY_HEARTBEAT_TICKS: u64 = 4;
const BUCKET_MAX: u16 = 1000;

macro_rules! bounded_bucket {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
        pub(crate) struct $name(u16);

        impl $name {
            pub(crate) const MAX: u16 = BUCKET_MAX;

            #[must_use]
            pub(crate) fn new(value: u16) -> Self {
                Self(value.min(Self::MAX))
            }

            #[must_use]
            pub(crate) fn value(self) -> u16 {
                self.0
            }

            #[must_use]
            pub(crate) fn saturating_add(self, rhs: u16) -> Self {
                Self::new(self.0.saturating_add(rhs))
            }
        }
    };
}

bounded_bucket!(EntropyBucket);
bounded_bucket!(SupportBucket);
bounded_bucket!(DivergenceBucket);
bounded_bucket!(ResidualBucket);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OperatingRegime {
    Sparse,
    Congested,
    RetentionFavorable,
    Unstable,
    Adversarial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RoutingPosture {
    Opportunistic,
    Structured,
    RetentionBiased,
    RiskSuppressed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ObservationClass {
    DirectOnly,
    ForwardPropagated,
    ReverseValidated,
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DestinationInterestClass {
    Dormant,
    Propagated,
    Transit,
    LocalOrigin,
    Pinned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DestinationInterest {
    pub(crate) class: DestinationInterestClass,
    pub(crate) last_material_interest: Option<Tick>,
}

impl Default for DestinationInterest {
    fn default() -> Self {
        Self {
            class: DestinationInterestClass::Dormant,
            last_material_interest: None,
        }
    }
}

impl DestinationInterest {
    pub(crate) fn promote(&mut self, class: DestinationInterestClass, tick: Tick) {
        if class > self.class {
            self.class = class;
        }
        self.last_material_interest = Some(tick);
    }

    #[must_use]
    pub(crate) fn eviction_rank(&self) -> (DestinationInterestClass, Tick) {
        (self.class, self.last_material_interest.unwrap_or(Tick(0)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct HopBand {
    pub(crate) min_hops: u8,
    pub(crate) max_hops: u8,
}

impl HopBand {
    #[must_use]
    pub(crate) fn new(min_hops: u8, max_hops: u8) -> Self {
        let min_hops = min_hops.min(ROUTE_HOP_COUNT_MAX);
        let max_hops = max_hops.max(min_hops).min(ROUTE_HOP_COUNT_MAX);
        Self { min_hops, max_hops }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegimeBeliefState {
    pub(crate) sparse: SupportBucket,
    pub(crate) congested: SupportBucket,
    pub(crate) retention_favorable: SupportBucket,
    pub(crate) unstable: SupportBucket,
    pub(crate) adversarial: SupportBucket,
}

impl Default for RegimeBeliefState {
    fn default() -> Self {
        Self {
            sparse: SupportBucket::new(BUCKET_MAX),
            congested: SupportBucket::default(),
            retention_favorable: SupportBucket::default(),
            unstable: SupportBucket::default(),
            adversarial: SupportBucket::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DestinationPosterior {
    pub(crate) usability_entropy: EntropyBucket,
    pub(crate) top_corridor_mass: SupportBucket,
    pub(crate) regime_belief: RegimeBeliefState,
    pub(crate) predicted_observation_class: ObservationClass,
}

impl Default for DestinationPosterior {
    fn default() -> Self {
        Self {
            usability_entropy: EntropyBucket::default(),
            top_corridor_mass: SupportBucket::new(BUCKET_MAX),
            regime_belief: RegimeBeliefState::default(),
            predicted_observation_class: ObservationClass::DirectOnly,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProgressBelief {
    pub(crate) progress_score: Belief<HealthScore>,
    pub(crate) uncertainty_penalty: Belief<PenaltyPoints>,
    pub(crate) posterior_support: SupportBucket,
}

impl Default for ProgressBelief {
    fn default() -> Self {
        Self {
            progress_score: Belief::Absent,
            uncertainty_penalty: Belief::Absent,
            posterior_support: SupportBucket::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CorridorBeliefEnvelope {
    pub(crate) expected_hop_band: HopBand,
    pub(crate) delivery_support: SupportBucket,
    pub(crate) congestion_penalty: EntropyBucket,
    pub(crate) retention_affinity: SupportBucket,
    pub(crate) validity_window: TimeWindow,
}

impl CorridorBeliefEnvelope {
    #[must_use]
    pub(crate) fn new(now: Tick) -> Self {
        Self {
            expected_hop_band: HopBand::new(1, ROUTE_HOP_COUNT_MAX),
            delivery_support: SupportBucket::default(),
            congestion_penalty: EntropyBucket::default(),
            retention_affinity: SupportBucket::default(),
            validity_window: TimeWindow::new(now, Tick(now.0.saturating_add(1)))
                .expect("field validity window"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NeighborContinuation {
    pub(crate) neighbor_id: NodeId,
    pub(crate) net_value: SupportBucket,
    pub(crate) downstream_support: SupportBucket,
    pub(crate) expected_hop_band: HopBand,
    pub(crate) freshness: Tick,
}

impl NeighborContinuation {
    #[must_use]
    pub(crate) fn ordering_key(&self) -> (u16, u16, u8, Tick, NodeId) {
        (
            self.net_value.value(),
            self.downstream_support.value(),
            self.expected_hop_band.min_hops,
            self.freshness,
            self.neighbor_id,
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ContinuationFrontier {
    entries: Vec<NeighborContinuation>,
}

impl ContinuationFrontier {
    #[must_use]
    pub(crate) fn insert(mut self, continuation: NeighborContinuation) -> Self {
        self.entries
            .retain(|entry| entry.neighbor_id != continuation.neighbor_id);
        self.entries.push(continuation);
        self.entries.sort_by(|left, right| {
            right
                .ordering_key()
                .cmp(&left.ordering_key())
                .then_with(|| left.neighbor_id.cmp(&right.neighbor_id))
        });
        self.entries.truncate(MAX_FRONTIER_SIZE);
        self
    }

    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub(crate) fn as_slice(&self) -> &[NeighborContinuation] {
        &self.entries
    }

    #[must_use]
    pub(crate) fn prune_stale(mut self, now_tick: Tick, max_age_ticks: u64) -> Self {
        self.entries
            .retain(|entry| now_tick.0.saturating_sub(entry.freshness.0) <= max_age_ticks);
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MeanFieldState {
    pub(crate) relay_alignment: SupportBucket,
    pub(crate) congestion_alignment: SupportBucket,
    pub(crate) retention_alignment: SupportBucket,
    pub(crate) risk_alignment: SupportBucket,
    pub(crate) field_strength: SupportBucket,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ControlState {
    pub(crate) congestion_price: EntropyBucket,
    pub(crate) relay_price: EntropyBucket,
    pub(crate) retention_price: EntropyBucket,
    pub(crate) risk_price: EntropyBucket,
    pub(crate) congestion_error_integral: ResidualBucket,
    pub(crate) retention_error_integral: ResidualBucket,
    pub(crate) relay_error_integral: ResidualBucket,
    pub(crate) churn_error_integral: ResidualBucket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegimeObserverState {
    pub(crate) current: OperatingRegime,
    pub(crate) current_regime_score: SupportBucket,
    pub(crate) regime_error_residual: ResidualBucket,
    pub(crate) log_likelihood_margin: DivergenceBucket,
    pub(crate) regime_change_threshold: ResidualBucket,
    pub(crate) regime_hysteresis_threshold: ResidualBucket,
    pub(crate) dwell_until_tick: Tick,
}

impl Default for RegimeObserverState {
    fn default() -> Self {
        Self {
            current: OperatingRegime::Sparse,
            current_regime_score: SupportBucket::new(800),
            regime_error_residual: ResidualBucket::default(),
            log_likelihood_margin: DivergenceBucket::default(),
            regime_change_threshold: ResidualBucket::new(700),
            regime_hysteresis_threshold: ResidualBucket::new(400),
            dwell_until_tick: Tick(0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PostureControllerState {
    pub(crate) current: RoutingPosture,
    pub(crate) stability_margin: SupportBucket,
    pub(crate) convergence_score: SupportBucket,
    pub(crate) posture_switch_threshold: ResidualBucket,
    pub(crate) last_transition_tick: Tick,
}

impl Default for PostureControllerState {
    fn default() -> Self {
        Self {
            current: RoutingPosture::Structured,
            stability_margin: SupportBucket::new(800),
            convergence_score: SupportBucket::new(800),
            posture_switch_threshold: ResidualBucket::new(500),
            last_transition_tick: Tick(0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DestinationFieldState {
    pub(crate) destination: DestinationKey,
    pub(crate) posterior: DestinationPosterior,
    pub(crate) progress_belief: ProgressBelief,
    pub(crate) corridor_belief: CorridorBeliefEnvelope,
    pub(crate) frontier: ContinuationFrontier,
    pub(crate) interest: DestinationInterest,
    pub(crate) observer_cache: ObserverCacheState,
    pub(crate) publication: SummaryPublicationState,
}

impl DestinationFieldState {
    #[must_use]
    pub(crate) fn new(destination: DestinationKey, now: Tick) -> Self {
        Self {
            destination,
            posterior: DestinationPosterior::default(),
            progress_belief: ProgressBelief::default(),
            corridor_belief: CorridorBeliefEnvelope::new(now),
            frontier: ContinuationFrontier::default(),
            interest: DestinationInterest::default(),
            observer_cache: ObserverCacheState::default(),
            publication: SummaryPublicationState::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ObserverInputSignature {
    pub(crate) topology_epoch: RouteEpoch,
    pub(crate) regime: OperatingRegime,
    pub(crate) direct_digest: u64,
    pub(crate) forward_digest: u64,
    pub(crate) reverse_digest: u64,
    pub(crate) control_signature: [u16; 8],
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ObserverCacheState {
    pub(crate) last_signature: Option<ObserverInputSignature>,
    pub(crate) last_updated_at: Option<Tick>,
}

impl ObserverCacheState {
    #[must_use]
    pub(crate) fn should_refresh(&self, signature: ObserverInputSignature, now_tick: Tick) -> bool {
        let Some(previous) = self.last_signature else {
            return true;
        };
        if previous != signature {
            return true;
        }
        let Some(last_updated_at) = self.last_updated_at else {
            return true;
        };
        now_tick.0.saturating_sub(last_updated_at.0) >= OBSERVER_CACHE_REFRESH_TICKS
    }

    pub(crate) fn record(&mut self, signature: ObserverInputSignature, now_tick: Tick) {
        self.last_signature = Some(signature);
        self.last_updated_at = Some(now_tick);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SummaryPublicationState {
    pub(crate) last_summary: Option<FieldSummary>,
    pub(crate) last_sent_at: Option<Tick>,
}

impl SummaryPublicationState {
    pub(crate) fn record(&mut self, summary: FieldSummary, now_tick: Tick) {
        self.last_summary = Some(summary);
        self.last_sent_at = Some(now_tick);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldEngineState {
    pub(crate) destinations: BTreeMap<DestinationKey, DestinationFieldState>,
    pub(crate) neighbor_endpoints: BTreeMap<NodeId, LinkEndpoint>,
    pub(crate) mean_field: MeanFieldState,
    pub(crate) controller: ControlState,
    pub(crate) regime: RegimeObserverState,
    pub(crate) posture: PostureControllerState,
    pub(crate) last_tick_processed: Tick,
}

impl FieldEngineState {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            destinations: BTreeMap::new(),
            neighbor_endpoints: BTreeMap::new(),
            mean_field: MeanFieldState::default(),
            controller: ControlState::default(),
            regime: RegimeObserverState::default(),
            posture: PostureControllerState::default(),
            last_tick_processed: Tick(0),
        }
    }

    pub(crate) fn note_tick(&mut self, tick: Tick) {
        self.last_tick_processed = tick;
    }

    pub(crate) fn upsert_destination_interest(
        &mut self,
        destination: &DestinationId,
        class: DestinationInterestClass,
        tick: Tick,
    ) -> &mut DestinationFieldState {
        let key = DestinationKey::from(destination);
        if !self.destinations.contains_key(&key)
            && self.destinations.len() >= MAX_TRACKED_DESTINATIONS
        {
            let evict = self
                .destinations
                .iter()
                .min_by_key(|(key, state)| {
                    (
                        state.interest.eviction_rank().0,
                        state.interest.eviction_rank().1,
                        *key,
                    )
                })
                .map(|(key, _)| key.clone())
                .expect("bounded destination eviction candidate");
            self.destinations.remove(&evict);
        }
        let state = self
            .destinations
            .entry(key.clone())
            .or_insert_with(|| DestinationFieldState::new(key, tick));
        state.interest.promote(class, tick);
        state
    }

    #[must_use]
    pub(crate) fn tracked_destination_count(&self) -> usize {
        self.destinations.len()
    }

    #[must_use]
    pub(crate) fn active_destination_keys(&self) -> Vec<DestinationKey> {
        let mut ranked = self
            .destinations
            .iter()
            .filter(|(_, state)| destination_is_active(state))
            .map(|(destination, state)| {
                (
                    (
                        state.interest.class,
                        state.posterior.top_corridor_mass,
                        state.corridor_belief.delivery_support,
                        state.interest.last_material_interest.unwrap_or(Tick(0)),
                    ),
                    destination.clone(),
                )
            })
            .collect::<Vec<_>>();
        ranked.sort_by(
            |(left_rank, left_destination), (right_rank, right_destination)| {
                right_rank
                    .cmp(left_rank)
                    .then_with(|| left_destination.cmp(right_destination))
            },
        );
        ranked
            .into_iter()
            .map(|(_, destination)| destination)
            .take(MAX_ACTIVE_DESTINATIONS)
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DestinationKey {
    Gateway(GatewayId),
    Node(NodeId),
    Service(Vec<u8>),
}

impl From<&DestinationId> for DestinationKey {
    fn from(value: &DestinationId) -> Self {
        match value {
            DestinationId::Gateway(id) => Self::Gateway(*id),
            DestinationId::Node(id) => Self::Node(*id),
            DestinationId::Service(id) => Self::Service(id.0.clone()),
        }
    }
}

impl From<&DestinationKey> for DestinationId {
    fn from(value: &DestinationKey) -> Self {
        match value {
            DestinationKey::Gateway(id) => Self::Gateway(*id),
            DestinationKey::Node(id) => Self::Node(*id),
            DestinationKey::Service(id) => Self::Service(ServiceId(id.clone())),
        }
    }
}

fn destination_is_active(state: &DestinationFieldState) -> bool {
    state.interest.class > DestinationInterestClass::Dormant
        || state.posterior.top_corridor_mass.value() > 0
        || state.corridor_belief.delivery_support.value() > 0
        || !state.frontier.as_slice().is_empty()
}

#[cfg(test)]
mod tests {
    use jacquard_core::{DestinationId, NodeId, PROVIDER_CANDIDATE_COUNT_MAX};

    use super::*;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    #[test]
    fn bucket_types_clamp_to_maximum() {
        assert_eq!(EntropyBucket::new(1_500).value(), BUCKET_MAX);
        assert_eq!(SupportBucket::new(1_500).value(), BUCKET_MAX);
        assert_eq!(DivergenceBucket::new(1_500).value(), BUCKET_MAX);
        assert_eq!(ResidualBucket::new(1_500).value(), BUCKET_MAX);
    }

    #[test]
    fn hop_band_clamps_and_orders_bounds() {
        let band = HopBand::new(20, 4);
        assert_eq!(band.min_hops, ROUTE_HOP_COUNT_MAX);
        assert_eq!(band.max_hops, ROUTE_HOP_COUNT_MAX);
    }

    #[test]
    fn continuation_frontier_is_bounded_and_deterministically_ordered() {
        let mut frontier = ContinuationFrontier::default();
        for (byte, value) in [(4, 300), (1, 900), (3, 700), (2, 800), (5, 1000)] {
            frontier = frontier.insert(NeighborContinuation {
                neighbor_id: node(byte),
                net_value: SupportBucket::new(value),
                downstream_support: SupportBucket::new(value),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(u64::from(byte)),
            });
        }

        assert_eq!(frontier.len(), MAX_FRONTIER_SIZE);
        let ordered = frontier
            .as_slice()
            .iter()
            .map(|entry| entry.neighbor_id)
            .collect::<Vec<_>>();
        assert_eq!(ordered, vec![node(5), node(1), node(2), node(3)]);
    }

    #[test]
    fn continuation_frontier_prunes_stale_entries() {
        let frontier = ContinuationFrontier::default()
            .insert(NeighborContinuation {
                neighbor_id: node(1),
                net_value: SupportBucket::new(900),
                downstream_support: SupportBucket::new(900),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(2),
            })
            .insert(NeighborContinuation {
                neighbor_id: node(2),
                net_value: SupportBucket::new(800),
                downstream_support: SupportBucket::new(800),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(7),
            })
            .prune_stale(Tick(10), 4);

        let remaining = frontier
            .as_slice()
            .iter()
            .map(|entry| entry.neighbor_id)
            .collect::<Vec<_>>();
        assert_eq!(remaining, vec![node(2)]);
    }

    #[test]
    fn destination_store_evicts_lowest_interest_then_oldest_tick() {
        let mut state = FieldEngineState::new();
        for index in 0..MAX_TRACKED_DESTINATIONS {
            let destination = DestinationId::Node(node(u8::try_from(index + 1).unwrap()));
            state.upsert_destination_interest(
                &destination,
                DestinationInterestClass::Propagated,
                Tick(u64::try_from(index + 1).unwrap()),
            );
        }

        let pinned = DestinationId::Node(node(1));
        state.upsert_destination_interest(&pinned, DestinationInterestClass::Pinned, Tick(999));

        let new_destination = DestinationId::Node(node(250));
        state.upsert_destination_interest(
            &new_destination,
            DestinationInterestClass::Transit,
            Tick(1_000),
        );

        assert_eq!(state.tracked_destination_count(), MAX_TRACKED_DESTINATIONS);
        assert!(state
            .destinations
            .contains_key(&DestinationKey::from(&pinned)));
        assert!(state
            .destinations
            .contains_key(&DestinationKey::from(&new_destination)));
        assert!(!state
            .destinations
            .contains_key(&DestinationKey::from(&DestinationId::Node(node(2)))));
    }

    #[test]
    fn destination_record_stays_tightly_bounded() {
        let state = DestinationFieldState::new(DestinationKey::Node(node(9)), Tick(10));
        assert!(state.frontier.len() <= MAX_FRONTIER_SIZE);
        assert_eq!(
            state.corridor_belief.expected_hop_band.max_hops,
            ROUTE_HOP_COUNT_MAX
        );
        assert!(state.posterior.top_corridor_mass.value() <= BUCKET_MAX);
    }

    #[test]
    fn tracked_destination_limit_stays_within_shared_candidate_budget() {
        assert!(MAX_TRACKED_DESTINATIONS <= usize::from(PROVIDER_CANDIDATE_COUNT_MAX));
    }

    #[test]
    fn active_destination_selection_is_sparse_and_deterministic() {
        let mut state = FieldEngineState::new();
        for index in 0..(MAX_ACTIVE_DESTINATIONS + 4) {
            let destination = DestinationId::Node(node(u8::try_from(index + 1).unwrap()));
            let destination_state = state.upsert_destination_interest(
                &destination,
                DestinationInterestClass::Transit,
                Tick(u64::try_from(index + 1).unwrap()),
            );
            destination_state.posterior.top_corridor_mass =
                SupportBucket::new(u16::try_from(700 + index).unwrap());
        }

        let active_once = state.active_destination_keys();
        let active_twice = state.active_destination_keys();
        assert_eq!(active_once, active_twice);
        assert_eq!(active_once.len(), MAX_ACTIVE_DESTINATIONS);
    }

    #[test]
    fn observer_cache_skips_until_refresh_window_expires() {
        let signature = ObserverInputSignature {
            topology_epoch: RouteEpoch(4),
            regime: OperatingRegime::Sparse,
            direct_digest: 11,
            forward_digest: 0,
            reverse_digest: 0,
            control_signature: [1; 8],
        };
        let mut cache = ObserverCacheState::default();
        assert!(cache.should_refresh(signature, Tick(4)));
        cache.record(signature, Tick(4));
        assert!(!cache.should_refresh(signature, Tick(5)));
        assert!(cache.should_refresh(signature, Tick(6)));
    }
}
