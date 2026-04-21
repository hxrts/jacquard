//! Bounded custody posture helpers for Mercator.

use jacquard_core::{
    Blake3Digest, ContentId, DestinationId, NodeId, OrderStamp, RetentionError, RouteEpoch, Tick,
};
use jacquard_traits::{Blake3Hashing, Hashing, RetentionStore};
use serde::{Deserialize, Serialize};

use crate::{
    evidence::{MercatorCustodyOpportunity, MercatorObjectiveKey},
    MercatorEngine, MercatorEvidenceMeta,
};

const DOMAIN_TAG_CUSTODY_OBJECT: &[u8] = b"mercator-custody-object";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorCustodyForwardingContext {
    pub receiver_same_cluster: bool,
    pub receiver_cluster_has_holder: bool,
    pub receiver_is_terminal_target: bool,
    pub bridge_opportunity: bool,
    pub energy_cost_units: u32,
    pub energy_pressure_permille: u16,
    pub observer_leakage_permille: u16,
    pub decided_at_tick: Tick,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorCustodyForwardingIntent {
    pub object_id: ContentId<Blake3Digest>,
    pub carrier: NodeId,
    pub improvement_score: u16,
    pub protected_budget_used: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MercatorCustodySuppressionReason {
    UnknownObject,
    NoStrictImprovement,
    LowGain,
    SameClusterRedundant,
    CopyBudgetExhausted,
    EnergyPressure,
    LeakageRisk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MercatorCustodyDecision {
    Forward(MercatorCustodyForwardingIntent),
    Suppressed(MercatorCustodySuppressionReason),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MercatorCustodyRecord {
    pub object_id: ContentId<Blake3Digest>,
    pub objective: MercatorObjectiveKey,
    pub payload_bytes: u32,
    pub copy_budget_remaining: u32,
    pub protected_bridge_budget_remaining: u32,
    pub best_improvement_score: u16,
    pub reproduction_count: u32,
    pub transmission_count: u32,
    pub energy_spent_units: u32,
    pub observer_leakage_permille: u16,
    pub retained_at_tick: Tick,
    pub last_progress_at_tick: Option<Tick>,
    pub meta: MercatorEvidenceMeta,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MercatorCustodyStats {
    pub record_count: u32,
    pub reproduction_count: u32,
    pub copy_budget_spent: u32,
    pub copy_budget_remaining: u32,
    pub protected_budget_spent: u32,
    pub protected_budget_remaining: u32,
    pub transmission_count: u32,
    pub storage_bytes: u32,
    pub energy_spent_units: u32,
    pub leakage_risk_permille: u16,
}

impl MercatorEngine {
    pub fn retain_custody_payload<R>(
        &mut self,
        destination: DestinationId,
        payload: Vec<u8>,
        now: Tick,
        retention: &mut R,
    ) -> Result<ContentId<Blake3Digest>, RetentionError>
    where
        R: RetentionStore,
    {
        if self.custody_records.len()
            >= usize::try_from(self.config.evidence.custody_record_count_max).unwrap_or(usize::MAX)
        {
            return Err(RetentionError::Full);
        }
        let payload_bytes = u32::try_from(payload.len()).unwrap_or(u32::MAX);
        if payload_bytes > self.config.bounds.custody_payload_bytes_max {
            return Err(RetentionError::Full);
        }
        let objective = MercatorObjectiveKey::destination(destination);
        let object_id = custody_object_id(self.local_node_id, &objective, &payload);
        retention.retain_payload(object_id, payload)?;
        self.custody_records.insert(
            object_id,
            MercatorCustodyRecord {
                object_id,
                objective,
                payload_bytes,
                copy_budget_remaining: self.config.bounds.custody_copy_budget_max,
                protected_bridge_budget_remaining: self
                    .config
                    .bounds
                    .custody_protected_bridge_budget,
                best_improvement_score: 0,
                reproduction_count: 0,
                transmission_count: 0,
                energy_spent_units: 0,
                observer_leakage_permille: 0,
                retained_at_tick: now,
                last_progress_at_tick: None,
                meta: MercatorEvidenceMeta::new(
                    self.latest_topology_epoch.unwrap_or(RouteEpoch(0)),
                    now,
                    self.config.bounds.evidence_validity,
                    OrderStamp(u64::try_from(self.custody_records.len()).unwrap_or(u64::MAX)),
                ),
            },
        );
        self.refresh_custody_diagnostics();
        Ok(object_id)
    }

    #[must_use]
    pub fn custody_record_count(&self) -> usize {
        self.custody_records.len()
    }

    #[must_use]
    pub fn plan_custody_forwarding(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
        opportunity: &MercatorCustodyOpportunity,
        context: MercatorCustodyForwardingContext,
    ) -> MercatorCustodyDecision {
        if context.bridge_opportunity {
            self.evidence.record_custody_bridge_opportunity();
        }
        let Some(record) = self.custody_records.get(object_id) else {
            self.record_custody_suppression(MercatorCustodySuppressionReason::UnknownObject);
            return MercatorCustodyDecision::Suppressed(
                MercatorCustodySuppressionReason::UnknownObject,
            );
        };
        let suppression = custody_suppression_reason(record, opportunity, &context, &self.config);
        if let Some(reason) = suppression {
            self.record_custody_suppression(reason);
            self.refresh_custody_diagnostics();
            return MercatorCustodyDecision::Suppressed(reason);
        }
        let protected_budget_used = {
            let record = self
                .custody_records
                .get_mut(object_id)
                .expect("custody record checked above");
            let protected_budget_used = record.copy_budget_remaining == 0;
            if protected_budget_used {
                record.protected_bridge_budget_remaining =
                    record.protected_bridge_budget_remaining.saturating_sub(1);
            } else {
                record.copy_budget_remaining = record.copy_budget_remaining.saturating_sub(1);
            }
            if !context.receiver_is_terminal_target {
                record.reproduction_count = record.reproduction_count.saturating_add(1);
            }
            record.transmission_count = record.transmission_count.saturating_add(1);
            record.energy_spent_units = record
                .energy_spent_units
                .saturating_add(context.energy_cost_units);
            record.observer_leakage_permille = record
                .observer_leakage_permille
                .max(context.observer_leakage_permille);
            record.best_improvement_score = record
                .best_improvement_score
                .max(opportunity.improvement_score);
            record.last_progress_at_tick = Some(context.decided_at_tick);
            protected_budget_used
        };
        if protected_budget_used {
            self.evidence.record_custody_protected_bridge_usage();
        }
        self.refresh_custody_diagnostics();
        MercatorCustodyDecision::Forward(MercatorCustodyForwardingIntent {
            object_id: *object_id,
            carrier: opportunity.carrier,
            improvement_score: opportunity.improvement_score,
            protected_budget_used,
        })
    }

    pub(crate) fn refresh_custody_diagnostics(&mut self) {
        let stats = custody_stats(
            self.custody_records.values(),
            self.config.bounds.custody_copy_budget_max,
            self.config.bounds.custody_protected_bridge_budget,
        );
        self.evidence.record_custody_stats(stats);
    }

    fn record_custody_suppression(&mut self, reason: MercatorCustodySuppressionReason) {
        self.evidence.record_custody_suppression(reason);
    }
}

fn custody_suppression_reason(
    record: &MercatorCustodyRecord,
    opportunity: &MercatorCustodyOpportunity,
    context: &MercatorCustodyForwardingContext,
    config: &crate::MercatorEngineConfig,
) -> Option<MercatorCustodySuppressionReason> {
    if opportunity.improvement_score <= record.best_improvement_score {
        return Some(MercatorCustodySuppressionReason::NoStrictImprovement);
    }
    let gain = opportunity
        .improvement_score
        .saturating_sub(record.best_improvement_score);
    if gain < config.bounds.custody_low_gain_floor {
        return Some(MercatorCustodySuppressionReason::LowGain);
    }
    if context.receiver_same_cluster
        && context.receiver_cluster_has_holder
        && !context.receiver_is_terminal_target
    {
        return Some(MercatorCustodySuppressionReason::SameClusterRedundant);
    }
    if context.energy_pressure_permille >= config.bounds.custody_energy_pressure_threshold {
        return Some(MercatorCustodySuppressionReason::EnergyPressure);
    }
    if context.observer_leakage_permille >= config.bounds.custody_leakage_risk_threshold
        && !context.receiver_is_terminal_target
    {
        return Some(MercatorCustodySuppressionReason::LeakageRisk);
    }
    if record.copy_budget_remaining == 0
        && !(context.bridge_opportunity && record.protected_bridge_budget_remaining > 0)
    {
        return Some(MercatorCustodySuppressionReason::CopyBudgetExhausted);
    }
    None
}

fn custody_stats<'a>(
    records: impl Iterator<Item = &'a MercatorCustodyRecord>,
    copy_budget_max: u32,
    protected_budget_max: u32,
) -> MercatorCustodyStats {
    let mut stats = MercatorCustodyStats::default();
    for record in records {
        stats.record_count = stats.record_count.saturating_add(1);
        stats.reproduction_count = stats
            .reproduction_count
            .saturating_add(record.reproduction_count);
        stats.copy_budget_remaining = stats
            .copy_budget_remaining
            .saturating_add(record.copy_budget_remaining);
        stats.protected_budget_remaining = stats
            .protected_budget_remaining
            .saturating_add(record.protected_bridge_budget_remaining);
        stats.transmission_count = stats
            .transmission_count
            .saturating_add(record.transmission_count);
        stats.storage_bytes = stats.storage_bytes.saturating_add(record.payload_bytes);
        stats.energy_spent_units = stats
            .energy_spent_units
            .saturating_add(record.energy_spent_units);
        stats.leakage_risk_permille = stats
            .leakage_risk_permille
            .max(record.observer_leakage_permille);
        stats.copy_budget_spent = stats
            .copy_budget_spent
            .saturating_add(copy_budget_max.saturating_sub(record.copy_budget_remaining));
        stats.protected_budget_spent = stats.protected_budget_spent.saturating_add(
            protected_budget_max.saturating_sub(record.protected_bridge_budget_remaining),
        );
    }
    stats
}

fn custody_object_id(
    local_node_id: NodeId,
    objective: &MercatorObjectiveKey,
    payload: &[u8],
) -> ContentId<Blake3Digest> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&local_node_id.0);
    encode_objective_key(&mut bytes, objective);
    bytes.extend_from_slice(
        &u32::try_from(payload.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
    ContentId {
        digest: Blake3Hashing.hash_tagged(DOMAIN_TAG_CUSTODY_OBJECT, &bytes),
    }
}

fn encode_objective_key(bytes: &mut Vec<u8>, objective: &MercatorObjectiveKey) {
    match objective {
        MercatorObjectiveKey::Node(node) => {
            bytes.push(0);
            bytes.extend_from_slice(&node.0);
        }
        MercatorObjectiveKey::Service(service) => {
            bytes.push(1);
            bytes.extend_from_slice(&service.0);
        }
        MercatorObjectiveKey::Gateway(gateway) => {
            bytes.push(2);
            bytes.extend_from_slice(&gateway.0);
        }
    }
}
