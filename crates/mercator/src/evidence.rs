//! Bounded evidence graph and epoch-safety state for Mercator.

use std::collections::BTreeMap;

use jacquard_core::{
    DestinationId, DurationMs, GatewayId, NodeId, OrderStamp, RouteEpoch, RouteId, ServiceId, Tick,
};
use serde::{Deserialize, Serialize};

use crate::MercatorEvidenceBounds;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorDiagnostics {
    pub selected_result_rounds: u32,
    pub no_candidate_attempts: u32,
    pub inadmissible_candidate_attempts: u32,
    pub support_withdrawal_count: u32,
    pub stale_persistence_rounds: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MercatorSupportState {
    Fresh,
    Suspect,
    Repairing,
    Withdrawn,
    CustodyOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MercatorObjectiveKey {
    Node(NodeId),
    Service(ServiceId),
    Gateway(GatewayId),
}

impl MercatorObjectiveKey {
    #[must_use]
    pub fn destination(destination: DestinationId) -> Self {
        match destination {
            DestinationId::Node(node) => Self::Node(node),
            DestinationId::Service(service) => Self::Service(service),
            DestinationId::Gateway(gateway) => Self::Gateway(gateway),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorEvidenceMeta {
    pub topology_epoch: RouteEpoch,
    pub observed_at_tick: Tick,
    pub valid_for: DurationMs,
    pub order: OrderStamp,
}

impl MercatorEvidenceMeta {
    #[must_use]
    pub fn new(
        topology_epoch: RouteEpoch,
        observed_at_tick: Tick,
        valid_for: DurationMs,
        order: OrderStamp,
    ) -> Self {
        Self {
            topology_epoch,
            observed_at_tick,
            valid_for,
            order,
        }
    }

    #[must_use]
    pub fn crosses_disruption(self, disruption_epoch: RouteEpoch) -> bool {
        self.topology_epoch < disruption_epoch
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorLinkEvidence {
    pub from: NodeId,
    pub to: NodeId,
    pub bidirectional_confidence: u16,
    pub asymmetric_penalty: u16,
    pub meta: MercatorEvidenceMeta,
}

impl MercatorLinkEvidence {
    #[must_use]
    pub fn pruning_key(&self) -> (u8, u16, std::cmp::Reverse<u16>, Tick, OrderStamp, NodeId) {
        (
            1,
            self.bidirectional_confidence,
            std::cmp::Reverse(self.asymmetric_penalty),
            self.meta.observed_at_tick,
            self.meta.order,
            self.to,
        )
    }

    fn invalidate_for_disruption(&mut self, disruption_epoch: RouteEpoch) -> bool {
        if self.meta.crosses_disruption(disruption_epoch) {
            self.bidirectional_confidence = 0;
            self.asymmetric_penalty = u16::MAX;
            return true;
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorReverseLinkEvidence {
    pub from: NodeId,
    pub to: NodeId,
    pub reverse_confidence: u16,
    pub meta: MercatorEvidenceMeta,
}

impl MercatorReverseLinkEvidence {
    #[must_use]
    pub fn pruning_key(&self) -> (u16, Tick, OrderStamp, NodeId, NodeId) {
        (
            self.reverse_confidence,
            self.meta.observed_at_tick,
            self.meta.order,
            self.from,
            self.to,
        )
    }

    fn invalidate_for_disruption(&mut self, disruption_epoch: RouteEpoch) -> bool {
        if self.meta.crosses_disruption(disruption_epoch) {
            self.reverse_confidence = 0;
            return true;
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorRouteSupport {
    pub route_id: RouteId,
    pub objective: MercatorObjectiveKey,
    pub state: MercatorSupportState,
    pub support_score: u16,
    pub last_loss_epoch: Option<RouteEpoch>,
    pub stale_started_at: Option<Tick>,
    pub meta: MercatorEvidenceMeta,
}

impl MercatorRouteSupport {
    #[must_use]
    pub fn pruning_key(&self) -> (u8, u16, Tick, OrderStamp, MercatorObjectiveKey, RouteId) {
        (
            support_state_rank(self.state),
            self.support_score,
            self.meta.observed_at_tick,
            self.meta.order,
            self.objective.clone(),
            self.route_id,
        )
    }

    fn invalidate_for_disruption(&mut self, disruption_epoch: RouteEpoch, now: Tick) -> bool {
        if !self.meta.crosses_disruption(disruption_epoch) {
            return false;
        }
        let was_active = matches!(
            self.state,
            MercatorSupportState::Fresh
                | MercatorSupportState::Suspect
                | MercatorSupportState::Repairing
        );
        self.support_score = 0;
        self.state = MercatorSupportState::Withdrawn;
        if was_active {
            self.stale_started_at = Some(now);
        }
        true
    }

    #[must_use]
    pub fn post_disruption_stale_rounds(&self, now: Tick) -> u32 {
        if self.state != MercatorSupportState::Withdrawn {
            return 0;
        }
        let Some(started_at) = self.stale_started_at else {
            return 0;
        };
        u32::try_from(now.0.saturating_sub(started_at.0)).unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorBrokerPressure {
    pub broker: NodeId,
    pub participation_count: u32,
    pub pressure_score: u16,
    pub meta: MercatorEvidenceMeta,
}

impl MercatorBrokerPressure {
    #[must_use]
    pub fn pruning_key(&self) -> (std::cmp::Reverse<u16>, u32, Tick, OrderStamp, NodeId) {
        (
            std::cmp::Reverse(self.pressure_score),
            self.participation_count,
            self.meta.observed_at_tick,
            self.meta.order,
            self.broker,
        )
    }

    fn invalidate_for_disruption(&mut self, disruption_epoch: RouteEpoch) -> bool {
        if self.meta.crosses_disruption(disruption_epoch) {
            self.participation_count = 0;
            self.pressure_score = u16::MAX;
            return true;
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorServiceSupport {
    pub objective: MercatorObjectiveKey,
    pub provider: NodeId,
    pub support_score: u16,
    pub meta: MercatorEvidenceMeta,
}

impl MercatorServiceSupport {
    #[must_use]
    pub fn pruning_key(&self) -> (u16, Tick, OrderStamp, MercatorObjectiveKey, NodeId) {
        (
            self.support_score,
            self.meta.observed_at_tick,
            self.meta.order,
            self.objective.clone(),
            self.provider,
        )
    }

    fn invalidate_for_disruption(&mut self, disruption_epoch: RouteEpoch) -> bool {
        if self.meta.crosses_disruption(disruption_epoch) {
            self.support_score = 0;
            return true;
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorCustodyOpportunity {
    pub objective: MercatorObjectiveKey,
    pub carrier: NodeId,
    pub improvement_score: u16,
    pub custody_pressure: u16,
    pub meta: MercatorEvidenceMeta,
}

impl MercatorCustodyOpportunity {
    #[must_use]
    pub fn pruning_key(
        &self,
    ) -> (
        u16,
        std::cmp::Reverse<u16>,
        Tick,
        OrderStamp,
        MercatorObjectiveKey,
        NodeId,
    ) {
        (
            self.improvement_score,
            std::cmp::Reverse(self.custody_pressure),
            self.meta.observed_at_tick,
            self.meta.order,
            self.objective.clone(),
            self.carrier,
        )
    }

    fn invalidate_for_disruption(&mut self, disruption_epoch: RouteEpoch) -> bool {
        if self.meta.crosses_disruption(disruption_epoch) {
            self.improvement_score = 0;
            self.custody_pressure = u16::MAX;
            return true;
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorEvidenceGraph {
    bounds: MercatorEvidenceBounds,
    link_evidence: BTreeMap<(NodeId, NodeId), MercatorLinkEvidence>,
    reverse_link_evidence: BTreeMap<(NodeId, NodeId), MercatorReverseLinkEvidence>,
    route_support: BTreeMap<RouteId, MercatorRouteSupport>,
    broker_pressure: BTreeMap<NodeId, MercatorBrokerPressure>,
    service_support: BTreeMap<(MercatorObjectiveKey, NodeId), MercatorServiceSupport>,
    custody_opportunities: BTreeMap<(MercatorObjectiveKey, NodeId), MercatorCustodyOpportunity>,
    diagnostics: MercatorDiagnostics,
    latest_disruption_epoch: Option<RouteEpoch>,
}

impl MercatorEvidenceGraph {
    #[must_use]
    pub fn new(bounds: MercatorEvidenceBounds) -> Self {
        Self {
            bounds,
            link_evidence: BTreeMap::new(),
            reverse_link_evidence: BTreeMap::new(),
            route_support: BTreeMap::new(),
            broker_pressure: BTreeMap::new(),
            service_support: BTreeMap::new(),
            custody_opportunities: BTreeMap::new(),
            diagnostics: MercatorDiagnostics::default(),
            latest_disruption_epoch: None,
        }
    }

    #[must_use]
    pub fn bounds(&self) -> MercatorEvidenceBounds {
        self.bounds
    }

    #[must_use]
    pub fn diagnostics(&self) -> MercatorDiagnostics {
        self.diagnostics
    }

    #[must_use]
    pub fn latest_disruption_epoch(&self) -> Option<RouteEpoch> {
        self.latest_disruption_epoch
    }

    #[must_use]
    pub fn link_evidence(&self) -> Vec<MercatorLinkEvidence> {
        self.link_evidence.values().copied().collect()
    }

    #[must_use]
    pub fn route_support(&self) -> Vec<MercatorRouteSupport> {
        self.route_support.values().cloned().collect()
    }

    pub fn record_link_evidence(&mut self, evidence: MercatorLinkEvidence) {
        self.link_evidence
            .insert((evidence.from, evidence.to), evidence);
        prune_btree_map(
            &mut self.link_evidence,
            self.bounds.neighbor_count_max,
            MercatorLinkEvidence::pruning_key,
        );
    }

    pub fn record_reverse_link_support(&mut self, evidence: MercatorReverseLinkEvidence) {
        self.reverse_link_evidence
            .insert((evidence.from, evidence.to), evidence);
        prune_btree_map(
            &mut self.reverse_link_evidence,
            self.bounds.neighbor_count_max,
            MercatorReverseLinkEvidence::pruning_key,
        );
    }

    pub fn record_route_support(&mut self, support: MercatorRouteSupport) {
        self.route_support.insert(support.route_id, support);
        prune_btree_map(
            &mut self.route_support,
            self.bounds.corridor_alternate_count_max,
            MercatorRouteSupport::pruning_key,
        );
    }

    pub fn record_broker_pressure(&mut self, pressure: MercatorBrokerPressure) {
        self.broker_pressure.insert(pressure.broker, pressure);
        prune_btree_map(
            &mut self.broker_pressure,
            self.bounds.candidate_broker_count_max,
            MercatorBrokerPressure::pruning_key,
        );
    }

    pub fn record_service_support(&mut self, support: MercatorServiceSupport) {
        self.service_support
            .insert((support.objective.clone(), support.provider), support);
        prune_btree_map(
            &mut self.service_support,
            self.bounds.service_evidence_count_max,
            MercatorServiceSupport::pruning_key,
        );
    }

    pub fn record_custody_opportunity(&mut self, opportunity: MercatorCustodyOpportunity) {
        self.custody_opportunities.insert(
            (opportunity.objective.clone(), opportunity.carrier),
            opportunity,
        );
        prune_btree_map(
            &mut self.custody_opportunities,
            self.bounds.custody_opportunity_count_max,
            MercatorCustodyOpportunity::pruning_key,
        );
    }

    pub fn record_selected_result_round(&mut self) {
        self.diagnostics.selected_result_rounds =
            self.diagnostics.selected_result_rounds.saturating_add(1);
    }

    pub fn record_no_candidate_attempt(&mut self) {
        self.diagnostics.no_candidate_attempts =
            self.diagnostics.no_candidate_attempts.saturating_add(1);
    }

    pub fn record_inadmissible_candidate_attempt(&mut self) {
        self.diagnostics.inadmissible_candidate_attempts = self
            .diagnostics
            .inadmissible_candidate_attempts
            .saturating_add(1);
    }

    pub fn invalidate_disruption_epoch(&mut self, disruption_epoch: RouteEpoch, now: Tick) {
        self.latest_disruption_epoch = Some(disruption_epoch);
        let mut withdrawal_count = 0_u32;
        for evidence in self.link_evidence.values_mut() {
            withdrawal_count = withdrawal_count.saturating_add(u32::from(
                evidence.invalidate_for_disruption(disruption_epoch),
            ));
        }
        for evidence in self.reverse_link_evidence.values_mut() {
            withdrawal_count = withdrawal_count.saturating_add(u32::from(
                evidence.invalidate_for_disruption(disruption_epoch),
            ));
        }
        for support in self.route_support.values_mut() {
            withdrawal_count = withdrawal_count.saturating_add(u32::from(
                support.invalidate_for_disruption(disruption_epoch, now),
            ));
        }
        for pressure in self.broker_pressure.values_mut() {
            withdrawal_count = withdrawal_count.saturating_add(u32::from(
                pressure.invalidate_for_disruption(disruption_epoch),
            ));
        }
        for support in self.service_support.values_mut() {
            withdrawal_count = withdrawal_count.saturating_add(u32::from(
                support.invalidate_for_disruption(disruption_epoch),
            ));
        }
        for opportunity in self.custody_opportunities.values_mut() {
            withdrawal_count = withdrawal_count.saturating_add(u32::from(
                opportunity.invalidate_for_disruption(disruption_epoch),
            ));
        }
        self.diagnostics.support_withdrawal_count = self
            .diagnostics
            .support_withdrawal_count
            .saturating_add(withdrawal_count);
        self.diagnostics.stale_persistence_rounds = self
            .route_support
            .values()
            .map(|support| support.post_disruption_stale_rounds(now))
            .sum();
    }
}

fn support_state_rank(state: MercatorSupportState) -> u8 {
    match state {
        MercatorSupportState::Fresh => 5,
        MercatorSupportState::Repairing => 4,
        MercatorSupportState::Suspect => 3,
        MercatorSupportState::CustodyOnly => 2,
        MercatorSupportState::Withdrawn => 1,
    }
}

fn prune_btree_map<K, V, F, O>(values: &mut BTreeMap<K, V>, cap: u32, order: F)
where
    K: Clone + Ord,
    F: Fn(&V) -> O,
    O: Ord,
{
    let cap = usize::try_from(cap).unwrap_or(usize::MAX);
    if values.len() <= cap {
        return;
    }
    let remove_count = values.len().saturating_sub(cap);
    let mut ranked = values
        .iter()
        .map(|(key, value)| (order(value), key.clone()))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let keys = ranked
        .into_iter()
        .take(remove_count)
        .map(|(_, key)| key)
        .collect::<Vec<_>>();
    for key in keys {
        values.remove(&key);
    }
}
