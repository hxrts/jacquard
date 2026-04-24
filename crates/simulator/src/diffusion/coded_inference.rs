//! Deterministic coded-inference readiness logs derived from diffusion scenarios.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    model::{
        CodedEvidenceOriginMode, CodedEvidenceTransformKind, CodedInferenceReadinessScenario,
        DiffusionTransportKind,
    },
    runtime::execution::generate_contacts,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum CodedArrivalClassification {
    Innovative,
    Duplicate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodedContactTraceEvent {
    pub round_index: u32,
    pub contact_id: u32,
    pub node_a: u32,
    pub node_b: u32,
    pub cluster_a: u8,
    pub cluster_b: u8,
    pub transport_kind: DiffusionTransportKind,
    pub bandwidth_bytes: u32,
    pub connection_delay: u32,
    pub energy_cost_per_byte: u32,
    pub contact_window: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodedEvidenceOriginLog {
    pub origin_mode: CodedEvidenceOriginMode,
    pub local_observation_id: Option<u32>,
    pub parent_evidence_ids: Vec<u32>,
    pub transform_kind: CodedEvidenceTransformKind,
    pub contribution_ledger_ids: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodedForwardingEvent {
    pub round_index: u32,
    pub sender_node_id: u32,
    pub receiver_node_id: u32,
    pub target_id: String,
    pub message_id: String,
    pub evidence_id: u32,
    pub fragment_id: Option<u32>,
    pub rank_id: Option<u32>,
    pub byte_count: u32,
    pub classification: CodedArrivalClassification,
    pub arrival_round: u32,
    pub sender_cluster_id: u8,
    pub receiver_cluster_id: u8,
    pub transport_kind: DiffusionTransportKind,
    pub policy_id: String,
    pub origin: CodedEvidenceOriginLog,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodedReceiverEvidenceEvent {
    pub round_index: u32,
    pub receiver_node_id: u32,
    pub evidence_id: u32,
    pub rank_before: u32,
    pub rank_after: u32,
    pub innovative_arrival_count: u32,
    pub duplicate_arrival_count: u32,
    pub reconstruction_event_round: Option<u32>,
    pub decision_event_round: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodedInferenceLandscapeEvent {
    pub round_index: u32,
    pub target_id: String,
    pub hidden_anomaly_cluster_id: u8,
    pub hypothesis_id: u8,
    pub scaled_score: i32,
    pub top_hypothesis_id: u8,
    pub runner_up_hypothesis_id: u8,
    pub margin: i32,
    pub uncertainty_permille: u32,
    pub energy_gap: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodedBudgetEvent {
    pub round_index: u32,
    pub payload_bytes_spent: u32,
    pub whole_message_bytes: u32,
    pub fragment_bytes: u32,
    pub forwarding_opportunities: u32,
    pub retained_bytes: u32,
    pub fixed_budget_label: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodedControllerTelemetryEvent {
    pub round_index: u32,
    pub target_reproduction_min_permille: u32,
    pub target_reproduction_max_permille: u32,
    pub measured_reproduction_permille: u32,
    pub active_forwarding_opportunities: u32,
    pub innovative_successor_opportunities: u32,
    pub duplicate_pressure_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodedInferenceReadinessLog {
    pub artifact_namespace: String,
    pub family_id: String,
    pub contact_events: Vec<CodedContactTraceEvent>,
    pub forwarding_events: Vec<CodedForwardingEvent>,
    pub receiver_events: Vec<CodedReceiverEvidenceEvent>,
    pub landscape_events: Vec<CodedInferenceLandscapeEvent>,
    pub budget_events: Vec<CodedBudgetEvent>,
    pub controller_events: Vec<CodedControllerTelemetryEvent>,
}

struct LogBuildState {
    evidence_id: u32,
    accepted_ledger_ids: BTreeSet<u32>,
    score_vector: Vec<i32>,
    innovative_arrival_count: u32,
    duplicate_arrival_count: u32,
    reconstruction_round: Option<u32>,
    decision_round: Option<u32>,
    recent_evidence_ids: Vec<u32>,
    evidence_ledger_by_id: BTreeMap<u32, Vec<u32>>,
}

pub(crate) fn build_coded_inference_readiness_log(
    seed: u64,
    scenario: &CodedInferenceReadinessScenario,
) -> CodedInferenceReadinessLog {
    let diffusion = &scenario.diffusion;
    let inference = &scenario.coded_inference;
    let observation_by_node = inference
        .local_observations
        .iter()
        .map(|observation| (observation.node_id, observation))
        .collect::<BTreeMap<_, _>>();
    let mut log = CodedInferenceReadinessLog {
        artifact_namespace: scenario.artifact_namespace.clone(),
        family_id: diffusion.family_id.clone(),
        contact_events: Vec::new(),
        forwarding_events: Vec::new(),
        receiver_events: Vec::new(),
        landscape_events: Vec::new(),
        budget_events: Vec::new(),
        controller_events: Vec::new(),
    };
    let mut state = LogBuildState {
        evidence_id: 1,
        accepted_ledger_ids: BTreeSet::new(),
        score_vector: inference.initial_score_vector.clone(),
        innovative_arrival_count: 0,
        duplicate_arrival_count: 0,
        reconstruction_round: None,
        decision_round: None,
        recent_evidence_ids: Vec::new(),
        evidence_ledger_by_id: BTreeMap::new(),
    };

    for round in diffusion.creation_round..diffusion.round_count {
        let contacts = generate_contacts(seed, diffusion, round);
        let mut round_payload_bytes = 0_u32;
        let mut round_forwarding_events = 0_u32;
        let mut round_innovative_events = 0_u32;
        let mut round_duplicate_events = 0_u32;
        for (contact_index, contact) in contacts.iter().enumerate() {
            let Some(contact_event) = contact_trace_event(
                diffusion,
                round,
                u32::try_from(contact_index).unwrap_or(u32::MAX),
                contact,
            ) else {
                continue;
            };
            log.contact_events.push(contact_event);
            let involves_receiver = contact.node_a == inference.receiver_node_id
                || contact.node_b == inference.receiver_node_id;
            if state.evidence_id > 48 && !involves_receiver {
                continue;
            }
            if state.evidence_id > 128 && log.receiver_events.len() >= 16 {
                continue;
            }
            let sender = if contact.node_a == inference.receiver_node_id {
                contact.node_b
            } else {
                contact.node_a
            };
            let receiver = if contact.node_b == inference.receiver_node_id {
                contact.node_b
            } else if contact.node_a == inference.receiver_node_id {
                contact.node_a
            } else {
                contact.node_b
            };
            let origin = evidence_origin_for(
                &mut state,
                inference.fragment_payload_bytes,
                sender,
                &observation_by_node,
            );
            let is_innovative = origin
                .contribution_ledger_ids
                .iter()
                .any(|ledger_id| !state.accepted_ledger_ids.contains(ledger_id));
            let classification = if is_innovative {
                round_innovative_events = round_innovative_events.saturating_add(1);
                CodedArrivalClassification::Innovative
            } else {
                round_duplicate_events = round_duplicate_events.saturating_add(1);
                CodedArrivalClassification::Duplicate
            };
            let sender_cluster_id = cluster_id_for(diffusion, sender).unwrap_or(0);
            let receiver_cluster_id = cluster_id_for(diffusion, receiver).unwrap_or(0);
            let event = CodedForwardingEvent {
                round_index: round,
                sender_node_id: sender,
                receiver_node_id: receiver,
                target_id: inference.target_id.clone(),
                message_id: inference.message_id.clone(),
                evidence_id: state.evidence_id,
                fragment_id: Some(state.evidence_id),
                rank_id: origin.contribution_ledger_ids.first().copied(),
                byte_count: inference.fragment_payload_bytes,
                classification,
                arrival_round: round.saturating_add(contact.connection_delay),
                sender_cluster_id,
                receiver_cluster_id,
                transport_kind: contact.transport_kind,
                policy_id: "coded-inference-readiness".to_string(),
                origin,
            };
            round_payload_bytes =
                round_payload_bytes.saturating_add(inference.fragment_payload_bytes);
            round_forwarding_events = round_forwarding_events.saturating_add(1);
            if event.receiver_node_id == inference.receiver_node_id {
                record_receiver_event(scenario, &event, &mut state, &mut log);
            }
            state.evidence_ledger_by_id.insert(
                event.evidence_id,
                event.origin.contribution_ledger_ids.clone(),
            );
            state.recent_evidence_ids.push(event.evidence_id);
            if state.recent_evidence_ids.len() > 8 {
                state.recent_evidence_ids.remove(0);
            }
            state.evidence_id = state.evidence_id.saturating_add(1);
            log.forwarding_events.push(event);
        }
        let active_forwarding_opportunities = u32::try_from(contacts.len()).unwrap_or(u32::MAX);
        log.budget_events.push(CodedBudgetEvent {
            round_index: round,
            payload_bytes_spent: round_payload_bytes,
            whole_message_bytes: inference.uncoded_message_payload_bytes,
            fragment_bytes: inference.fragment_payload_bytes,
            forwarding_opportunities: active_forwarding_opportunities,
            retained_bytes: u32::try_from(state.accepted_ledger_ids.len())
                .unwrap_or(u32::MAX)
                .saturating_mul(inference.fragment_payload_bytes),
            fixed_budget_label: "equal-payload-bytes".to_string(),
        });
        log.controller_events.push(CodedControllerTelemetryEvent {
            round_index: round,
            target_reproduction_min_permille: 800,
            target_reproduction_max_permille: 1200,
            measured_reproduction_permille: ratio_permille(
                round_innovative_events,
                active_forwarding_opportunities,
            ),
            active_forwarding_opportunities,
            innovative_successor_opportunities: round_innovative_events,
            duplicate_pressure_permille: ratio_permille(
                round_duplicate_events,
                round_forwarding_events,
            ),
        });
    }
    log
}

pub(crate) fn serialize_coded_inference_log(
    log: &CodedInferenceReadinessLog,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(log)
}

fn contact_trace_event(
    scenario: &super::model::DiffusionScenarioSpec,
    round: u32,
    contact_index: u32,
    contact: &super::model::DiffusionContactEvent,
) -> Option<CodedContactTraceEvent> {
    Some(CodedContactTraceEvent {
        round_index: round,
        contact_id: round.saturating_mul(10_000).saturating_add(contact_index),
        node_a: contact.node_a,
        node_b: contact.node_b,
        cluster_a: cluster_id_for(scenario, contact.node_a)?,
        cluster_b: cluster_id_for(scenario, contact.node_b)?,
        transport_kind: contact.transport_kind,
        bandwidth_bytes: contact.bandwidth_bytes,
        connection_delay: contact.connection_delay,
        energy_cost_per_byte: contact.energy_cost_per_byte,
        contact_window: contact.contact_window,
    })
}

fn evidence_origin_for(
    state: &mut LogBuildState,
    fragment_payload_bytes: u32,
    sender: u32,
    observation_by_node: &BTreeMap<u32, &super::model::CodedLocalObservationSpec>,
) -> CodedEvidenceOriginLog {
    match state.evidence_id % 3 {
        1 => CodedEvidenceOriginLog {
            origin_mode: CodedEvidenceOriginMode::SourceCoded,
            local_observation_id: None,
            parent_evidence_ids: Vec::new(),
            transform_kind: CodedEvidenceTransformKind::ForwardOriginal,
            contribution_ledger_ids: vec![1 + (state.evidence_id % 12)],
        },
        2 => {
            let local_observation = observation_by_node.get(&sender).copied();
            CodedEvidenceOriginLog {
                origin_mode: CodedEvidenceOriginMode::LocalObservation,
                local_observation_id: local_observation
                    .map(|observation| observation.observation_id),
                parent_evidence_ids: Vec::new(),
                transform_kind: CodedEvidenceTransformKind::ForwardOriginal,
                contribution_ledger_ids: local_observation
                    .map(|observation| vec![observation.contribution_ledger_id])
                    .unwrap_or_else(|| vec![fragment_payload_bytes.saturating_add(sender)]),
            }
        }
        _ => {
            let parent_evidence_ids = state
                .recent_evidence_ids
                .iter()
                .rev()
                .take(2)
                .copied()
                .collect::<Vec<_>>();
            let mut contribution_ledger_ids = parent_evidence_ids
                .iter()
                .filter_map(|evidence_id| state.evidence_ledger_by_id.get(evidence_id))
                .flatten()
                .copied()
                .collect::<BTreeSet<_>>();
            if let Some(local_observation) = observation_by_node.get(&sender) {
                contribution_ledger_ids.insert(local_observation.contribution_ledger_id);
            }
            CodedEvidenceOriginLog {
                origin_mode: CodedEvidenceOriginMode::RecodedAggregate,
                local_observation_id: observation_by_node
                    .get(&sender)
                    .map(|observation| observation.observation_id),
                parent_evidence_ids,
                transform_kind: CodedEvidenceTransformKind::ContributionLedgerUnion,
                contribution_ledger_ids: contribution_ledger_ids.into_iter().collect(),
            }
        }
    }
}

fn record_receiver_event(
    scenario: &CodedInferenceReadinessScenario,
    event: &CodedForwardingEvent,
    state: &mut LogBuildState,
    log: &mut CodedInferenceReadinessLog,
) {
    let rank_before = u32::try_from(state.accepted_ledger_ids.len()).unwrap_or(u32::MAX);
    let mut new_ledger_ids = Vec::new();
    for ledger_id in &event.origin.contribution_ledger_ids {
        if state.accepted_ledger_ids.insert(*ledger_id) {
            new_ledger_ids.push(*ledger_id);
        }
    }
    if new_ledger_ids.is_empty() {
        state.duplicate_arrival_count = state.duplicate_arrival_count.saturating_add(1);
    } else {
        state.innovative_arrival_count = state.innovative_arrival_count.saturating_add(1);
        apply_score_updates(scenario, &new_ledger_ids, &mut state.score_vector);
    }
    let rank_after = u32::try_from(state.accepted_ledger_ids.len()).unwrap_or(u32::MAX);
    if state.reconstruction_round.is_none()
        && rank_after >= scenario.coded_inference.reconstruction_threshold
    {
        state.reconstruction_round = Some(event.arrival_round);
    }
    let (top, runner_up, margin) = score_summary(&state.score_vector);
    if state.decision_round.is_none()
        && rank_after >= scenario.coded_inference.minimum_decision_evidence_count
        && margin >= scenario.coded_inference.decision_margin_threshold
    {
        state.decision_round = Some(event.arrival_round);
    }
    log.receiver_events.push(CodedReceiverEvidenceEvent {
        round_index: event.arrival_round,
        receiver_node_id: event.receiver_node_id,
        evidence_id: event.evidence_id,
        rank_before,
        rank_after,
        innovative_arrival_count: state.innovative_arrival_count,
        duplicate_arrival_count: state.duplicate_arrival_count,
        reconstruction_event_round: state.reconstruction_round,
        decision_event_round: state.decision_round,
    });
    for (hypothesis_id, score) in state.score_vector.iter().enumerate() {
        log.landscape_events.push(CodedInferenceLandscapeEvent {
            round_index: event.arrival_round,
            target_id: scenario.coded_inference.target_id.clone(),
            hidden_anomaly_cluster_id: scenario.coded_inference.hidden_anomaly_cluster_id,
            hypothesis_id: u8::try_from(hypothesis_id).unwrap_or(u8::MAX),
            scaled_score: *score,
            top_hypothesis_id: top,
            runner_up_hypothesis_id: runner_up,
            margin,
            uncertainty_permille: uncertainty_permille(margin),
            energy_gap: margin,
        });
    }
}

fn apply_score_updates(
    scenario: &CodedInferenceReadinessScenario,
    ledger_ids: &[u32],
    score_vector: &mut [i32],
) {
    let observation_by_ledger = scenario
        .coded_inference
        .local_observations
        .iter()
        .map(|observation| (observation.contribution_ledger_id, observation))
        .collect::<BTreeMap<_, _>>();
    for ledger_id in ledger_ids {
        if let Some(observation) = observation_by_ledger.get(ledger_id) {
            for (index, score) in observation.evidence_vector.iter().enumerate() {
                if let Some(target_score) = score_vector.get_mut(index) {
                    *target_score = target_score.saturating_add(*score);
                }
            }
        }
    }
}

fn cluster_id_for(scenario: &super::model::DiffusionScenarioSpec, node_id: u32) -> Option<u8> {
    scenario
        .node_index_by_id
        .get(&node_id)
        .and_then(|index| scenario.nodes.get(*index))
        .map(|node| node.cluster_id)
}

fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        0
    } else {
        numerator.saturating_mul(1000) / denominator
    }
}

fn score_summary(score_vector: &[i32]) -> (u8, u8, i32) {
    let mut ranked = score_vector
        .iter()
        .enumerate()
        .map(|(index, score)| (u8::try_from(index).unwrap_or(u8::MAX), *score))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let top = ranked.first().copied().unwrap_or((0, 0));
    let runner_up = ranked.get(1).copied().unwrap_or((top.0, top.1));
    (top.0, runner_up.0, top.1.saturating_sub(runner_up.1))
}

fn uncertainty_permille(margin: i32) -> u32 {
    let margin = u32::try_from(margin.max(0)).unwrap_or(u32::MAX);
    1000_u32.saturating_sub(margin.saturating_mul(20))
}

#[cfg(test)]
mod tests {
    use super::{build_coded_inference_readiness_log, serialize_coded_inference_log};
    use crate::diffusion::catalog::scenarios::build_coded_inference_readiness_scenario;

    #[test]
    fn coded_inference_readiness_logs_are_deterministic_and_complete() {
        let scenario = build_coded_inference_readiness_scenario();
        let first = build_coded_inference_readiness_log(41, &scenario);
        let second = build_coded_inference_readiness_log(41, &scenario);

        assert_eq!(first, second);
        assert_eq!(
            serialize_coded_inference_log(&first).expect("first serialization"),
            serialize_coded_inference_log(&second).expect("second serialization")
        );
        assert_eq!(
            first.artifact_namespace,
            "artifacts/coded-inference/readiness"
        );
        assert!(!first.contact_events.is_empty());
        assert!(!first.forwarding_events.is_empty());
        assert!(!first.receiver_events.is_empty());
        assert!(!first.landscape_events.is_empty());
        assert!(!first.budget_events.is_empty());
        assert!(!first.controller_events.is_empty());
        assert!(first.forwarding_events.iter().any(|event| {
            event.origin.origin_mode
                == crate::diffusion::model::CodedEvidenceOriginMode::SourceCoded
        }));
        assert!(first.forwarding_events.iter().any(|event| {
            event.origin.origin_mode
                == crate::diffusion::model::CodedEvidenceOriginMode::LocalObservation
        }));
        assert!(first.forwarding_events.iter().any(|event| {
            event.origin.origin_mode
                == crate::diffusion::model::CodedEvidenceOriginMode::RecodedAggregate
                && !event.origin.parent_evidence_ids.is_empty()
        }));
        assert!(first
            .receiver_events
            .iter()
            .any(|event| event.rank_after >= event.rank_before));
        assert!(first.budget_events.iter().all(|event| {
            event.fixed_budget_label == "equal-payload-bytes"
                && event.whole_message_bytes > 0
                && event.fragment_bytes > 0
        }));
        assert!(first.controller_events.iter().all(|event| {
            event.target_reproduction_min_permille <= event.target_reproduction_max_permille
                && event.measured_reproduction_permille <= 1000
                && event.duplicate_pressure_permille <= 1000
        }));
    }
}
