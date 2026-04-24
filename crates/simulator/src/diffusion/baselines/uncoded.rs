//! Uncoded full-message replication baseline.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    BaselineArrivalClassification, BaselineArrivalCounters, BaselineBudgetEvent,
    BaselineContractError, BaselineForwardingEvent, BaselinePayloadDescriptor, BaselinePayloadMode,
    BaselinePolicyClass, BaselinePolicyId, BaselineReceiverEvent, BaselineRunInput, BaselineRunLog,
    BaselineRunSummary, BaselineRunSummaryDraft, BaselineStorageEvent,
};
use crate::diffusion::runtime::execution::generate_contacts;

pub(crate) fn run_uncoded_replication_baseline(
    input: &BaselineRunInput,
) -> Result<BaselineRunLog, BaselineContractError> {
    if input.policy_id.policy_class() != BaselinePolicyClass::Replication {
        return Err(BaselineContractError::PolicyClassMismatch);
    }
    let scenario = &input.scenario.diffusion;
    let mut state = UncodedReplicationState::new(input);
    state.record_storage(scenario.creation_round, scenario.source_node_id);

    for round in scenario.creation_round..scenario.round_count {
        let mut round_spent = 0_u32;
        let contacts = generate_contacts(input.seed, scenario, round);
        for contact in contacts {
            if contact.bandwidth_bytes < state.whole_message_bytes {
                continue;
            }
            let holders_at_contact_start = state.holders.clone();
            for (from, to) in [
                (contact.node_a, contact.node_b),
                (contact.node_b, contact.node_a),
            ] {
                if !holders_at_contact_start.contains(&from) {
                    continue;
                }
                if !state.can_spend_next_copy() {
                    continue;
                }
                if state.should_skip_receiver(to) || state.should_skip_non_receiver(to) {
                    continue;
                }
                let arrival_round = round.saturating_add(contact.connection_delay);
                let bytes_spent = state.forward_copy(round, arrival_round, from, to)?;
                round_spent = round_spent.saturating_add(bytes_spent);
            }
        }
        state.record_budget(round, round_spent);
    }

    Ok(state.log)
}

// long-block-exception: summary assembly mirrors the shared baseline metric schema.
pub(crate) fn summarize_uncoded_replication_baseline(
    input: &BaselineRunInput,
    log: &BaselineRunLog,
) -> Result<BaselineRunSummary, BaselineContractError> {
    let reconstruction_round = log
        .receiver_events
        .iter()
        .find_map(|event| event.reconstruction_round);
    let counters = receiver_arrival_counters(log);
    let receiver_rank = log
        .receiver_events
        .last()
        .map(|event| event.rank_after)
        .unwrap_or(0);
    let bytes_transmitted = log
        .forwarding_events
        .iter()
        .map(|event| event.payload.byte_count)
        .fold(0_u32, u32::saturating_add);
    let (peak_units, peak_bytes) = peak_storage_by_node(log);
    let full_message_recovered = reconstruction_round.is_some();
    let margin = if full_message_recovered {
        input.scenario.coded_inference.decision_margin_threshold
    } else {
        0
    };

    BaselineRunSummary::try_from_draft(BaselineRunSummaryDraft {
        artifact_namespace: log.artifact_namespace.clone(),
        family_id: log.family_id.clone(),
        policy_id: Some(input.policy_id),
        payload_mode: Some(BaselinePayloadMode::UncodedWholeMessage),
        fixed_budget_label: Some(input.fixed_budget.label.clone()),
        fixed_payload_budget_bytes: Some(input.fixed_budget.payload_byte_budget),
        recovery_probability_permille: Some(if full_message_recovered { 1000 } else { 0 }),
        decision_accuracy_permille: Some(if full_message_recovered { 1000 } else { 0 }),
        reconstruction_round: Some(reconstruction_round),
        commitment_round: Some(reconstruction_round),
        receiver_rank: Some(receiver_rank),
        top_hypothesis_margin: Some(margin),
        bytes_transmitted: Some(bytes_transmitted),
        forwarding_events: Some(u32::try_from(log.forwarding_events.len()).unwrap_or(u32::MAX)),
        peak_stored_payload_units_per_node: Some(peak_units),
        peak_stored_payload_bytes_per_node: Some(peak_bytes),
        duplicate_rate_permille: Some(counters.duplicate_rate_permille()),
        innovative_arrival_rate_permille: Some(counters.innovative_arrival_rate_permille()),
        duplicate_arrival_count: Some(counters.duplicate_arrivals),
        innovative_arrival_count: Some(counters.innovative_arrivals),
        target_reproduction_min_permille: None,
        target_reproduction_max_permille: None,
        measured_reproduction_permille: None,
    })
}

struct UncodedReplicationState {
    log: BaselineRunLog,
    holders: BTreeSet<u32>,
    receiver_has_message: bool,
    reconstruction_round: Option<u32>,
    bytes_spent: u32,
    whole_message_bytes: u32,
    budget_bytes: u32,
    receiver_node_id: u32,
    next_message_copy_id: u32,
}

impl UncodedReplicationState {
    fn new(input: &BaselineRunInput) -> Self {
        let scenario = &input.scenario.diffusion;
        let receiver_node_id = input.scenario.coded_inference.receiver_node_id;
        let mut holders = BTreeSet::new();
        holders.insert(scenario.source_node_id);
        Self {
            log: BaselineRunLog {
                artifact_namespace: input.artifact_namespace.clone(),
                family_id: scenario.family_id.clone(),
                policy_id: input.policy_id,
                forwarding_events: Vec::new(),
                receiver_events: Vec::new(),
                storage_events: Vec::new(),
                budget_events: Vec::new(),
            },
            holders,
            receiver_has_message: scenario.source_node_id == receiver_node_id,
            reconstruction_round: None,
            bytes_spent: 0,
            whole_message_bytes: input.fixed_budget.whole_message_payload_bytes,
            budget_bytes: input.fixed_budget.payload_byte_budget,
            receiver_node_id,
            next_message_copy_id: 1,
        }
    }

    fn can_spend_next_copy(&self) -> bool {
        self.bytes_spent.saturating_add(self.whole_message_bytes) <= self.budget_bytes
    }

    fn should_skip_receiver(&self, to: u32) -> bool {
        to == self.receiver_node_id && !self.receiver_has_message && self.holders.contains(&to)
    }

    fn should_skip_non_receiver(&self, to: u32) -> bool {
        to != self.receiver_node_id && self.holders.contains(&to)
    }

    fn forward_copy(
        &mut self,
        round: u32,
        arrival_round: u32,
        from: u32,
        to: u32,
    ) -> Result<u32, BaselineContractError> {
        let classification = self.classify_arrival(to);
        self.record_forwarding(round, from, to, classification)?;
        self.bytes_spent = self.bytes_spent.saturating_add(self.whole_message_bytes);
        if !self.holders.contains(&to) {
            self.holders.insert(to);
            self.record_storage(arrival_round, to);
        }
        if to == self.receiver_node_id {
            self.record_receiver(arrival_round, classification);
        }
        Ok(self.whole_message_bytes)
    }

    fn classify_arrival(&self, to: u32) -> BaselineArrivalClassification {
        if to == self.receiver_node_id && self.receiver_has_message {
            BaselineArrivalClassification::Duplicate
        } else {
            BaselineArrivalClassification::Innovative
        }
    }

    fn record_forwarding(
        &mut self,
        round: u32,
        from: u32,
        to: u32,
        classification: BaselineArrivalClassification,
    ) -> Result<(), BaselineContractError> {
        let copy_id = self.next_message_copy_id;
        self.next_message_copy_id = self.next_message_copy_id.saturating_add(1);
        self.log.forwarding_events.push(BaselineForwardingEvent {
            round_index: round,
            sender_node_id: from,
            receiver_node_id: to,
            policy_id: BaselinePolicyId::UncodedReplication,
            payload: BaselinePayloadDescriptor::try_new(
                BaselinePayloadMode::UncodedWholeMessage,
                None,
                Some(copy_id),
                None,
                self.whole_message_bytes,
                vec![1],
            )?,
            classification,
        });
        Ok(())
    }

    fn record_receiver(
        &mut self,
        arrival_round: u32,
        classification: BaselineArrivalClassification,
    ) {
        let rank_before = if self.receiver_has_message { 1 } else { 0 };
        if classification == BaselineArrivalClassification::Innovative {
            self.receiver_has_message = true;
            self.reconstruction_round.get_or_insert(arrival_round);
        }
        let rank_after = if self.receiver_has_message { 1 } else { 0 };
        self.log.receiver_events.push(BaselineReceiverEvent {
            round_index: arrival_round,
            receiver_node_id: self.receiver_node_id,
            policy_id: BaselinePolicyId::UncodedReplication,
            arrival_classification: classification,
            rank_before,
            rank_after,
            reconstruction_round: self.reconstruction_round,
            commitment_round: self.reconstruction_round,
        });
    }

    fn record_storage(&mut self, round: u32, node_id: u32) {
        self.log.storage_events.push(BaselineStorageEvent {
            round_index: round,
            node_id,
            policy_id: BaselinePolicyId::UncodedReplication,
            stored_payload_units: 1,
            stored_payload_bytes: self.whole_message_bytes,
        });
    }

    fn record_budget(&mut self, round: u32, round_spent: u32) {
        self.log.budget_events.push(BaselineBudgetEvent {
            round_index: round,
            policy_id: BaselinePolicyId::UncodedReplication,
            payload_bytes_spent: round_spent,
            cumulative_payload_bytes_spent: self.bytes_spent,
            fixed_budget_label: super::EQUAL_PAYLOAD_BYTES_LABEL.to_string(),
            fixed_payload_budget_bytes: self.budget_bytes,
        });
    }
}

fn receiver_arrival_counters(log: &BaselineRunLog) -> BaselineArrivalCounters {
    let mut counters = BaselineArrivalCounters::default();
    for event in &log.receiver_events {
        match event.arrival_classification {
            BaselineArrivalClassification::Innovative => {
                counters.innovative_arrivals = counters.innovative_arrivals.saturating_add(1);
            }
            BaselineArrivalClassification::Duplicate => {
                counters.duplicate_arrivals = counters.duplicate_arrivals.saturating_add(1);
            }
        }
    }
    counters
}

fn peak_storage_by_node(log: &BaselineRunLog) -> (u32, u32) {
    let mut unit_peak_by_node = BTreeMap::<u32, u32>::new();
    let mut byte_peak_by_node = BTreeMap::<u32, u32>::new();
    for event in &log.storage_events {
        let unit_peak = unit_peak_by_node.entry(event.node_id).or_default();
        *unit_peak = (*unit_peak).max(event.stored_payload_units);
        let byte_peak = byte_peak_by_node.entry(event.node_id).or_default();
        *byte_peak = (*byte_peak).max(event.stored_payload_bytes);
    }
    (
        unit_peak_by_node.values().copied().max().unwrap_or(0),
        byte_peak_by_node.values().copied().max().unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        run_uncoded_replication_baseline, summarize_uncoded_replication_baseline,
        BaselineArrivalClassification, BaselineForwardingEvent, BaselinePayloadDescriptor,
        BaselinePayloadMode, BaselinePolicyId, BaselineReceiverEvent, BaselineRunInput,
        BaselineRunLog, BaselineStorageEvent,
    };
    use crate::diffusion::{
        baselines::{
            BaselineBudgetEvent, BaselineFixedBudget, BASELINE_ARTIFACT_NAMESPACE,
            EQUAL_PAYLOAD_BYTES_LABEL,
        },
        catalog::scenarios::build_coded_inference_readiness_scenario,
    };

    fn uncoded_input_with_budget(payload_byte_budget: u32) -> BaselineRunInput {
        let scenario = build_coded_inference_readiness_scenario();
        let budget = BaselineFixedBudget::try_new(
            EQUAL_PAYLOAD_BYTES_LABEL,
            payload_byte_budget,
            scenario.coded_inference.uncoded_message_payload_bytes,
            scenario.coded_inference.fragment_payload_bytes,
        )
        .expect("fixed budget");
        BaselineRunInput::try_new(41, scenario, BaselinePolicyId::UncodedReplication, budget)
            .expect("baseline input")
    }

    #[test]
    fn uncoded_replication_equal_byte_budget_limits_whole_message_replicas() {
        let input = uncoded_input_with_budget(384);
        let log = run_uncoded_replication_baseline(&input).expect("uncoded baseline");
        let summary =
            summarize_uncoded_replication_baseline(&input, &log).expect("uncoded summary");

        assert_eq!(
            summary.payload_mode,
            BaselinePayloadMode::UncodedWholeMessage
        );
        assert_eq!(summary.fixed_budget_label, EQUAL_PAYLOAD_BYTES_LABEL);
        assert!(summary.forwarding_events <= 1);
        assert!(summary.bytes_transmitted <= 384);
        assert_eq!(
            input.fixed_budget.payload_byte_budget / input.fixed_budget.whole_message_payload_bytes,
            1
        );
        assert_eq!(
            input.fixed_budget.payload_byte_budget / input.fixed_budget.fragment_payload_bytes,
            12
        );
    }

    #[test]
    fn uncoded_replication_runs_on_coded_inference_trace_format() {
        let input = uncoded_input_with_budget(768);
        let log = run_uncoded_replication_baseline(&input).expect("uncoded baseline");

        assert_eq!(log.artifact_namespace, BASELINE_ARTIFACT_NAMESPACE);
        assert_eq!(log.family_id, "coded-inference-100-node-readiness");
        assert_eq!(log.policy_id, BaselinePolicyId::UncodedReplication);
        assert!(!log.storage_events.is_empty());
        assert!(log.budget_events.iter().all(|event| {
            event.fixed_budget_label == EQUAL_PAYLOAD_BYTES_LABEL
                && event.fixed_payload_budget_bytes == input.fixed_budget.payload_byte_budget
        }));
    }

    #[test]
    fn uncoded_replication_duplicate_full_messages_do_not_inflate_rank_or_quality() {
        let input = uncoded_input_with_budget(1_152);
        let log = synthetic_duplicate_receiver_log(&input);
        let summary =
            summarize_uncoded_replication_baseline(&input, &log).expect("uncoded summary");

        assert_eq!(summary.receiver_rank, 1);
        assert_eq!(summary.innovative_arrival_count, 1);
        assert_eq!(summary.duplicate_arrival_count, 1);
        assert_eq!(summary.duplicate_rate_permille, 500);
        assert_eq!(summary.innovative_arrival_rate_permille, 500);
        assert_eq!(
            summary.top_hypothesis_margin,
            input.scenario.coded_inference.decision_margin_threshold
        );
    }

    #[test]
    fn uncoded_replication_artifacts_carry_fixed_fairness_budget_label() {
        let input = uncoded_input_with_budget(384);
        let log = run_uncoded_replication_baseline(&input).expect("uncoded baseline");
        let summary =
            summarize_uncoded_replication_baseline(&input, &log).expect("uncoded summary");

        assert_eq!(summary.fixed_budget_label, EQUAL_PAYLOAD_BYTES_LABEL);
        assert_eq!(summary.fixed_payload_budget_bytes, 384);
        assert!(log
            .budget_events
            .iter()
            .all(|event| event.fixed_budget_label == EQUAL_PAYLOAD_BYTES_LABEL));
    }

    fn synthetic_duplicate_receiver_log(input: &BaselineRunInput) -> BaselineRunLog {
        let receiver_node_id = input.scenario.coded_inference.receiver_node_id;
        let byte_count = input.fixed_budget.whole_message_payload_bytes;
        BaselineRunLog {
            artifact_namespace: BASELINE_ARTIFACT_NAMESPACE.to_string(),
            family_id: input.scenario.diffusion.family_id.clone(),
            policy_id: BaselinePolicyId::UncodedReplication,
            forwarding_events: vec![
                forwarding_event(8, 1, receiver_node_id, 1, byte_count),
                forwarding_event(9, 2, receiver_node_id, 2, byte_count),
            ],
            receiver_events: vec![
                receiver_event(
                    8,
                    receiver_node_id,
                    BaselineArrivalClassification::Innovative,
                    0,
                    1,
                ),
                receiver_event(
                    9,
                    receiver_node_id,
                    BaselineArrivalClassification::Duplicate,
                    1,
                    1,
                ),
            ],
            storage_events: vec![
                BaselineStorageEvent {
                    round_index: 4,
                    node_id: 1,
                    policy_id: BaselinePolicyId::UncodedReplication,
                    stored_payload_units: 1,
                    stored_payload_bytes: byte_count,
                },
                BaselineStorageEvent {
                    round_index: 8,
                    node_id: receiver_node_id,
                    policy_id: BaselinePolicyId::UncodedReplication,
                    stored_payload_units: 1,
                    stored_payload_bytes: byte_count,
                },
            ],
            budget_events: vec![BaselineBudgetEvent {
                round_index: 8,
                policy_id: BaselinePolicyId::UncodedReplication,
                payload_bytes_spent: byte_count,
                cumulative_payload_bytes_spent: byte_count,
                fixed_budget_label: EQUAL_PAYLOAD_BYTES_LABEL.to_string(),
                fixed_payload_budget_bytes: input.fixed_budget.payload_byte_budget,
            }],
        }
    }

    fn forwarding_event(
        round: u32,
        sender_node_id: u32,
        receiver_node_id: u32,
        copy_id: u32,
        byte_count: u32,
    ) -> BaselineForwardingEvent {
        BaselineForwardingEvent {
            round_index: round,
            sender_node_id,
            receiver_node_id,
            policy_id: BaselinePolicyId::UncodedReplication,
            payload: BaselinePayloadDescriptor::try_new(
                BaselinePayloadMode::UncodedWholeMessage,
                None,
                Some(copy_id),
                None,
                byte_count,
                vec![1],
            )
            .expect("payload"),
            classification: if copy_id == 1 {
                BaselineArrivalClassification::Innovative
            } else {
                BaselineArrivalClassification::Duplicate
            },
        }
    }

    fn receiver_event(
        round: u32,
        receiver_node_id: u32,
        classification: BaselineArrivalClassification,
        rank_before: u32,
        rank_after: u32,
    ) -> BaselineReceiverEvent {
        BaselineReceiverEvent {
            round_index: round,
            receiver_node_id,
            policy_id: BaselinePolicyId::UncodedReplication,
            arrival_classification: classification,
            rank_before,
            rank_after,
            reconstruction_round: Some(8),
            commitment_round: Some(8),
        }
    }
}
