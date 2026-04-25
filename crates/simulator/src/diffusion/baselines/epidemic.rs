//! Deterministic epidemic forwarding baseline.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    summarize_store_forward_baseline, BaselineArrivalClassification, BaselineBudgetEvent,
    BaselineContractError, BaselineForwardingEvent, BaselinePayloadDescriptor, BaselinePayloadMode,
    BaselinePolicyClass, BaselinePolicyId, BaselineReceiverEvent, BaselineRunInput, BaselineRunLog,
    BaselineRunSummary, BaselineStorageEvent,
};
use crate::diffusion::runtime::execution::generate_contacts;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EpidemicForwardingParams {
    pub payload_mode: BaselinePayloadMode,
    pub ttl_rounds: u32,
    pub storage_cap_payload_units: u32,
    pub per_contact_capacity: u32,
}

pub(crate) fn run_epidemic_forwarding_baseline(
    input: &BaselineRunInput,
    params: EpidemicForwardingParams,
) -> Result<BaselineRunLog, BaselineContractError> {
    if input.policy_id.policy_class() != BaselinePolicyClass::Flooding {
        return Err(BaselineContractError::PolicyClassMismatch);
    }
    let scenario = &input.scenario.diffusion;
    let mut state = EpidemicState::new(input, params);
    state.seed_source_holdings();

    let last_round = scenario
        .creation_round
        .saturating_add(params.ttl_rounds)
        .min(scenario.round_count);
    for round in scenario.creation_round..last_round {
        let contacts = generate_contacts(input.seed, scenario, round);
        let mut round_spent = 0_u32;
        for contact in contacts {
            if contact.bandwidth_bytes < state.payload_bytes {
                continue;
            }
            let snapshot = state.holdings.clone();
            for (from, to) in [
                (contact.node_a, contact.node_b),
                (contact.node_b, contact.node_a),
            ] {
                let Some(sender_holding) = snapshot.get(&from) else {
                    continue;
                };
                let mut sent_on_contact = 0_u32;
                for ledger_id in sender_holding {
                    if sent_on_contact >= params.per_contact_capacity {
                        break;
                    }
                    if !state.can_forward_to(to, *ledger_id) {
                        continue;
                    }
                    if !state.can_spend_next_payload() {
                        break;
                    }
                    let bytes_spent = state.forward(round, from, to, *ledger_id)?;
                    round_spent = round_spent.saturating_add(bytes_spent);
                    sent_on_contact = sent_on_contact.saturating_add(1);
                }
            }
        }
        state.record_budget(round, round_spent);
    }

    Ok(state.log)
}

// long-block-exception: summary assembly mirrors the shared baseline metric schema.
pub(crate) fn summarize_epidemic_forwarding_baseline(
    input: &BaselineRunInput,
    log: &BaselineRunLog,
    payload_mode: BaselinePayloadMode,
) -> Result<BaselineRunSummary, BaselineContractError> {
    summarize_store_forward_baseline(input, log, payload_mode)
}

struct EpidemicState {
    log: BaselineRunLog,
    params: EpidemicForwardingParams,
    holdings: BTreeMap<u32, BTreeSet<u32>>,
    accepted_receiver_ledgers: BTreeSet<u32>,
    reconstruction_round: Option<u32>,
    commitment_round: Option<u32>,
    bytes_spent: u32,
    payload_bytes: u32,
    budget_bytes: u32,
    receiver_node_id: u32,
    source_node_id: u32,
    max_ledger_id: u32,
}

impl EpidemicState {
    fn new(input: &BaselineRunInput, params: EpidemicForwardingParams) -> Self {
        let scenario = &input.scenario.diffusion;
        let inference = &input.scenario.coded_inference;
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
            params,
            holdings: BTreeMap::new(),
            accepted_receiver_ledgers: BTreeSet::new(),
            reconstruction_round: None,
            commitment_round: None,
            bytes_spent: 0,
            payload_bytes: params.payload_mode.byte_count(&input.fixed_budget),
            budget_bytes: input.fixed_budget.payload_byte_budget,
            receiver_node_id: inference.receiver_node_id,
            source_node_id: scenario.source_node_id,
            max_ledger_id: max_ledger_id(params.payload_mode, input),
        }
    }

    fn seed_source_holdings(&mut self) {
        let mut ledgers = BTreeSet::new();
        for ledger_id in 1..=self.max_ledger_id {
            ledgers.insert(ledger_id);
        }
        self.holdings.insert(self.source_node_id, ledgers);
        self.record_storage(0, self.source_node_id);
    }

    fn can_forward_to(&self, node_id: u32, ledger_id: u32) -> bool {
        if node_id == self.receiver_node_id {
            return true;
        }
        if self
            .holdings
            .get(&node_id)
            .is_some_and(|held| held.contains(&ledger_id))
        {
            return false;
        }
        let held_count = self
            .holdings
            .get(&node_id)
            .map(|held| u32::try_from(held.len()).unwrap_or(u32::MAX))
            .unwrap_or(0);
        held_count < self.params.storage_cap_payload_units
    }

    fn can_spend_next_payload(&self) -> bool {
        self.bytes_spent.saturating_add(self.payload_bytes) <= self.budget_bytes
    }

    fn forward(
        &mut self,
        round: u32,
        from: u32,
        to: u32,
        ledger_id: u32,
    ) -> Result<u32, BaselineContractError> {
        let classification = self.classification_for(to, ledger_id);
        self.record_forwarding(round, from, to, ledger_id, classification)?;
        self.bytes_spent = self.bytes_spent.saturating_add(self.payload_bytes);
        if to != self.receiver_node_id {
            self.holdings.entry(to).or_default().insert(ledger_id);
            self.record_storage(round, to);
        } else {
            self.record_receiver(round, ledger_id, classification);
        }
        Ok(self.payload_bytes)
    }

    fn classification_for(&self, to: u32, ledger_id: u32) -> BaselineArrivalClassification {
        if to == self.receiver_node_id && self.accepted_receiver_ledgers.contains(&ledger_id) {
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
        ledger_id: u32,
        classification: BaselineArrivalClassification,
    ) -> Result<(), BaselineContractError> {
        self.log.forwarding_events.push(BaselineForwardingEvent {
            round_index: round,
            sender_node_id: from,
            receiver_node_id: to,
            policy_id: BaselinePolicyId::EpidemicForwarding,
            payload: BaselinePayloadDescriptor::try_new(
                self.params.payload_mode,
                (self.params.payload_mode == BaselinePayloadMode::CodedFragment)
                    .then_some(ledger_id),
                (self.params.payload_mode == BaselinePayloadMode::UncodedWholeMessage)
                    .then_some(ledger_id),
                (self.params.payload_mode == BaselinePayloadMode::CodedFragment)
                    .then_some(ledger_id),
                self.payload_bytes,
                vec![ledger_id],
            )?,
            classification,
        });
        Ok(())
    }

    fn record_receiver(
        &mut self,
        round: u32,
        ledger_id: u32,
        classification: BaselineArrivalClassification,
    ) {
        let rank_before = u32::try_from(self.accepted_receiver_ledgers.len()).unwrap_or(u32::MAX);
        if classification == BaselineArrivalClassification::Innovative {
            self.accepted_receiver_ledgers.insert(ledger_id);
        }
        let rank_after = u32::try_from(self.accepted_receiver_ledgers.len()).unwrap_or(u32::MAX);
        if self.reconstruction_round.is_none() && rank_after >= self.reconstruction_threshold() {
            self.reconstruction_round = Some(round);
        }
        if self.commitment_round.is_none() && rank_after >= self.decision_threshold() {
            self.commitment_round = Some(round);
        }
        self.log.receiver_events.push(BaselineReceiverEvent {
            round_index: round,
            receiver_node_id: self.receiver_node_id,
            policy_id: BaselinePolicyId::EpidemicForwarding,
            arrival_classification: classification,
            rank_before,
            rank_after,
            reconstruction_round: self.reconstruction_round,
            commitment_round: self.commitment_round,
        });
    }

    fn reconstruction_threshold(&self) -> u32 {
        match self.params.payload_mode {
            BaselinePayloadMode::UncodedWholeMessage => 1,
            BaselinePayloadMode::CodedFragment => self.max_ledger_id.min(8),
        }
    }

    fn decision_threshold(&self) -> u32 {
        match self.params.payload_mode {
            BaselinePayloadMode::UncodedWholeMessage => 1,
            BaselinePayloadMode::CodedFragment => self.max_ledger_id.min(8),
        }
    }

    fn record_storage(&mut self, round: u32, node_id: u32) {
        let stored_payload_units = self
            .holdings
            .get(&node_id)
            .map(|held| u32::try_from(held.len()).unwrap_or(u32::MAX))
            .unwrap_or(0);
        self.log.storage_events.push(BaselineStorageEvent {
            round_index: round,
            node_id,
            policy_id: BaselinePolicyId::EpidemicForwarding,
            stored_payload_units,
            stored_payload_bytes: stored_payload_units.saturating_mul(self.payload_bytes),
        });
    }

    fn record_budget(&mut self, round: u32, round_spent: u32) {
        self.log.budget_events.push(BaselineBudgetEvent {
            round_index: round,
            policy_id: BaselinePolicyId::EpidemicForwarding,
            payload_bytes_spent: round_spent,
            cumulative_payload_bytes_spent: self.bytes_spent,
            fixed_budget_label: super::EQUAL_PAYLOAD_BYTES_LABEL.to_string(),
            fixed_payload_budget_bytes: self.budget_bytes,
        });
    }
}

fn max_ledger_id(payload_mode: BaselinePayloadMode, input: &BaselineRunInput) -> u32 {
    match payload_mode {
        BaselinePayloadMode::UncodedWholeMessage => 1,
        BaselinePayloadMode::CodedFragment => input.scenario.coded_inference.source_fragment_count,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        run_epidemic_forwarding_baseline, summarize_epidemic_forwarding_baseline,
        BaselinePayloadMode, BaselinePolicyId, EpidemicForwardingParams,
    };
    use crate::diffusion::{
        baselines::{
            uncoded::{run_uncoded_replication_baseline, summarize_uncoded_replication_baseline},
            BaselineFixedBudget, BaselineRunInput, EQUAL_PAYLOAD_BYTES_LABEL,
        },
        catalog::scenarios::build_coded_inference_readiness_scenario,
    };

    fn epidemic_input(payload_byte_budget: u32) -> BaselineRunInput {
        let scenario = build_coded_inference_readiness_scenario();
        let budget = BaselineFixedBudget::try_new(
            EQUAL_PAYLOAD_BYTES_LABEL,
            payload_byte_budget,
            scenario.coded_inference.uncoded_message_payload_bytes,
            scenario.coded_inference.fragment_payload_bytes,
        )
        .expect("budget");
        BaselineRunInput::try_new(41, scenario, BaselinePolicyId::EpidemicForwarding, budget)
            .expect("input")
    }

    fn uncoded_input(payload_byte_budget: u32) -> BaselineRunInput {
        let scenario = build_coded_inference_readiness_scenario();
        let budget = BaselineFixedBudget::try_new(
            EQUAL_PAYLOAD_BYTES_LABEL,
            payload_byte_budget,
            scenario.coded_inference.uncoded_message_payload_bytes,
            scenario.coded_inference.fragment_payload_bytes,
        )
        .expect("budget");
        BaselineRunInput::try_new(41, scenario, BaselinePolicyId::UncodedReplication, budget)
            .expect("input")
    }

    fn broad_params() -> EpidemicForwardingParams {
        EpidemicForwardingParams {
            payload_mode: BaselinePayloadMode::CodedFragment,
            ttl_rounds: 32,
            storage_cap_payload_units: 12,
            per_contact_capacity: 3,
        }
    }

    #[test]
    fn epidemic_forwarding_is_deterministic_under_contact_order() {
        let input = epidemic_input(1_536);
        let first = run_epidemic_forwarding_baseline(&input, broad_params()).expect("first");
        let second = run_epidemic_forwarding_baseline(&input, broad_params()).expect("second");

        assert_eq!(first, second);
        assert!(!first.forwarding_events.is_empty());
    }

    #[test]
    fn epidemic_forwarding_reports_payload_mode_and_shared_metrics() {
        let input = epidemic_input(1_536);
        let log = run_epidemic_forwarding_baseline(&input, broad_params()).expect("log");
        let summary = summarize_epidemic_forwarding_baseline(
            &input,
            &log,
            BaselinePayloadMode::CodedFragment,
        )
        .expect("summary");

        assert_eq!(summary.policy_id, BaselinePolicyId::EpidemicForwarding);
        assert_eq!(summary.payload_mode, BaselinePayloadMode::CodedFragment);
        assert!(summary.forwarding_events > 0);
        assert!(summary.bytes_transmitted > 0);
        assert!(summary.peak_stored_payload_units_per_node > 0);
    }

    #[test]
    fn epidemic_forwarding_exposes_duplicate_pressure_relative_to_bounded_replication() {
        let epidemic_input = epidemic_input(4_096);
        let epidemic_log =
            run_epidemic_forwarding_baseline(&epidemic_input, broad_params()).expect("epidemic");
        let epidemic_summary = summarize_epidemic_forwarding_baseline(
            &epidemic_input,
            &epidemic_log,
            BaselinePayloadMode::CodedFragment,
        )
        .expect("epidemic summary");

        let uncoded_input = uncoded_input(384);
        let uncoded_log = run_uncoded_replication_baseline(&uncoded_input).expect("uncoded");
        let uncoded_summary = summarize_uncoded_replication_baseline(&uncoded_input, &uncoded_log)
            .expect("uncoded summary");

        assert!(
            epidemic_summary.duplicate_arrival_count >= uncoded_summary.duplicate_arrival_count
        );
        assert!(epidemic_summary.forwarding_events > uncoded_summary.forwarding_events);
    }

    #[test]
    fn epidemic_forwarding_caps_stop_further_spread() {
        let input = epidemic_input(96);
        let params = EpidemicForwardingParams {
            payload_mode: BaselinePayloadMode::CodedFragment,
            ttl_rounds: 4,
            storage_cap_payload_units: 1,
            per_contact_capacity: 1,
        };
        let log = run_epidemic_forwarding_baseline(&input, params).expect("log");
        let summary = summarize_epidemic_forwarding_baseline(
            &input,
            &log,
            BaselinePayloadMode::CodedFragment,
        )
        .expect("summary");

        assert!(summary.bytes_transmitted <= input.fixed_budget.payload_byte_budget);
        assert!(summary.peak_stored_payload_units_per_node <= 12);
        assert!(summary.forwarding_events <= 3);
    }
}
// proc-macro-scope: epidemic baseline rows are artifact schema, not shared model vocabulary.
