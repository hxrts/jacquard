//! Deterministic spray-and-wait baseline.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    summarize_store_forward_baseline, BaselineArrivalClassification, BaselineBudgetEvent,
    BaselineContractError, BaselineForwardingEvent, BaselinePayloadDescriptor, BaselinePayloadMode,
    BaselinePolicyClass, BaselinePolicyId, BaselineReceiverEvent, BaselineRunInput, BaselineRunLog,
    BaselineRunSummary, BaselineStorageEvent,
};
use crate::diffusion::runtime::execution::generate_contacts;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SpraySplitRule {
    BinaryHalve,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SprayDirectDeliveryRule {
    ReceiverOnlyAfterSpray,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SprayAndWaitParams {
    pub payload_mode: BaselinePayloadMode,
    pub initial_copy_count: u32,
    pub split_rule: SpraySplitRule,
    pub direct_delivery_rule: SprayDirectDeliveryRule,
    pub ttl_rounds: u32,
    pub storage_cap_payload_units: u32,
}

pub(crate) fn run_spray_and_wait_baseline(
    input: &BaselineRunInput,
    params: SprayAndWaitParams,
) -> Result<BaselineRunLog, BaselineContractError> {
    if input.policy_id.policy_class() != BaselinePolicyClass::BoundedCopy {
        return Err(BaselineContractError::PolicyClassMismatch);
    }
    let scenario = &input.scenario.diffusion;
    let mut state = SprayState::new(input, params);
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
            let snapshot = state.copy_budgets.clone();
            for (from, to) in [
                (contact.node_a, contact.node_b),
                (contact.node_b, contact.node_a),
            ] {
                let Some(sender_budgets) = snapshot.get(&from) else {
                    continue;
                };
                let Some(ledger_id) = sender_budgets.iter().find_map(|(ledger_id, copies)| {
                    state.transferable_ledger(from, to, *ledger_id, *copies)
                }) else {
                    continue;
                };
                if !state.can_spend_next_payload() {
                    continue;
                }
                let bytes_spent = state.forward(round, from, to, ledger_id)?;
                round_spent = round_spent.saturating_add(bytes_spent);
            }
        }
        state.record_budget(round, round_spent);
    }

    Ok(state.log)
}

// long-block-exception: summary assembly mirrors the shared baseline metric schema.
pub(crate) fn summarize_spray_and_wait_baseline(
    input: &BaselineRunInput,
    log: &BaselineRunLog,
    payload_mode: BaselinePayloadMode,
) -> Result<BaselineRunSummary, BaselineContractError> {
    summarize_store_forward_baseline(input, log, payload_mode)
}

struct SprayState {
    log: BaselineRunLog,
    params: SprayAndWaitParams,
    copy_budgets: BTreeMap<u32, BTreeMap<u32, u32>>,
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

impl SprayState {
    fn new(input: &BaselineRunInput, params: SprayAndWaitParams) -> Self {
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
            copy_budgets: BTreeMap::new(),
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
        let mut budget_by_ledger = BTreeMap::new();
        let mut ledgers = BTreeSet::new();
        for ledger_id in 1..=self.max_ledger_id {
            budget_by_ledger.insert(ledger_id, self.params.initial_copy_count);
            ledgers.insert(ledger_id);
        }
        self.copy_budgets
            .insert(self.source_node_id, budget_by_ledger);
        self.holdings.insert(self.source_node_id, ledgers);
        self.record_storage(0, self.source_node_id);
    }

    fn transferable_ledger(&self, from: u32, to: u32, ledger_id: u32, copies: u32) -> Option<u32> {
        if to == self.receiver_node_id {
            if self.accepted_receiver_ledgers.contains(&ledger_id) {
                return None;
            }
            return (copies > 0).then_some(ledger_id);
        }
        if self
            .holdings
            .get(&to)
            .is_some_and(|held| held.contains(&ledger_id))
        {
            return None;
        }
        let held_count = self
            .holdings
            .get(&to)
            .map(|held| u32::try_from(held.len()).unwrap_or(u32::MAX))
            .unwrap_or(0);
        if held_count >= self.params.storage_cap_payload_units {
            return None;
        }
        if self.in_wait_phase(from, ledger_id) {
            return None;
        }
        (copies > 1).then_some(ledger_id)
    }

    fn in_wait_phase(&self, node_id: u32, ledger_id: u32) -> bool {
        let copies = self
            .copy_budgets
            .get(&node_id)
            .and_then(|budgets| budgets.get(&ledger_id))
            .copied()
            .unwrap_or(0);
        matches!(
            self.params.direct_delivery_rule,
            SprayDirectDeliveryRule::ReceiverOnlyAfterSpray
        ) && copies <= 1
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
        let transferred_copies = self.transfer_copy_budget(from, to, ledger_id);
        if transferred_copies == 0 && to != self.receiver_node_id {
            return Ok(0);
        }
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

    fn transfer_copy_budget(&mut self, from: u32, to: u32, ledger_id: u32) -> u32 {
        let Some(from_budgets) = self.copy_budgets.get_mut(&from) else {
            return 0;
        };
        let Some(from_copies) = from_budgets.get_mut(&ledger_id) else {
            return 0;
        };
        if to == self.receiver_node_id {
            return 1;
        }
        let transfer = match self.params.split_rule {
            SpraySplitRule::BinaryHalve => *from_copies / 2,
        };
        if transfer == 0 {
            return 0;
        }
        *from_copies = from_copies.saturating_sub(transfer);
        self.copy_budgets
            .entry(to)
            .or_default()
            .insert(ledger_id, transfer);
        transfer
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
            policy_id: BaselinePolicyId::SprayAndWait,
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
            policy_id: BaselinePolicyId::SprayAndWait,
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
        self.reconstruction_threshold()
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
            policy_id: BaselinePolicyId::SprayAndWait,
            stored_payload_units,
            stored_payload_bytes: stored_payload_units.saturating_mul(self.payload_bytes),
        });
    }

    fn record_budget(&mut self, round: u32, round_spent: u32) {
        self.log.budget_events.push(BaselineBudgetEvent {
            round_index: round,
            policy_id: BaselinePolicyId::SprayAndWait,
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
        run_spray_and_wait_baseline, summarize_spray_and_wait_baseline, BaselinePayloadMode,
        BaselinePolicyId, SprayAndWaitParams, SprayDirectDeliveryRule, SpraySplitRule,
    };
    use crate::diffusion::{
        baselines::{
            epidemic::{
                run_epidemic_forwarding_baseline, summarize_epidemic_forwarding_baseline,
                EpidemicForwardingParams,
            },
            BaselineFixedBudget, BaselineRunInput, EQUAL_PAYLOAD_BYTES_LABEL,
        },
        catalog::scenarios::build_coded_inference_readiness_scenario,
    };

    fn spray_input(payload_byte_budget: u32) -> BaselineRunInput {
        let scenario = build_coded_inference_readiness_scenario();
        let budget = BaselineFixedBudget::try_new(
            EQUAL_PAYLOAD_BYTES_LABEL,
            payload_byte_budget,
            scenario.coded_inference.uncoded_message_payload_bytes,
            scenario.coded_inference.fragment_payload_bytes,
        )
        .expect("budget");
        BaselineRunInput::try_new(41, scenario, BaselinePolicyId::SprayAndWait, budget)
            .expect("input")
    }

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

    fn spray_params(initial_copy_count: u32) -> SprayAndWaitParams {
        SprayAndWaitParams {
            payload_mode: BaselinePayloadMode::CodedFragment,
            initial_copy_count,
            split_rule: SpraySplitRule::BinaryHalve,
            direct_delivery_rule: SprayDirectDeliveryRule::ReceiverOnlyAfterSpray,
            ttl_rounds: 32,
            storage_cap_payload_units: 12,
        }
    }

    #[test]
    fn spray_and_wait_copy_budget_splitting_is_deterministic() {
        let input = spray_input(1_536);
        let first = run_spray_and_wait_baseline(&input, spray_params(8)).expect("first");
        let second = run_spray_and_wait_baseline(&input, spray_params(8)).expect("second");

        assert_eq!(first, second);
        assert!(!first.forwarding_events.is_empty());
    }

    #[test]
    fn spray_and_wait_reports_shared_schema_and_budget() {
        let input = spray_input(1_536);
        let log = run_spray_and_wait_baseline(&input, spray_params(8)).expect("log");
        let summary =
            summarize_spray_and_wait_baseline(&input, &log, BaselinePayloadMode::CodedFragment)
                .expect("summary");

        assert_eq!(summary.policy_id, BaselinePolicyId::SprayAndWait);
        assert_eq!(summary.payload_mode, BaselinePayloadMode::CodedFragment);
        assert!(summary.bytes_transmitted <= input.fixed_budget.payload_byte_budget);
        assert!(summary.peak_stored_payload_units_per_node > 0);
    }

    #[test]
    fn exhausted_spray_budget_prevents_broad_forwarding() {
        let input = spray_input(1_536);
        let broad = run_spray_and_wait_baseline(&input, spray_params(8)).expect("broad");
        let exhausted = run_spray_and_wait_baseline(&input, spray_params(1)).expect("exhausted");

        assert!(exhausted.forwarding_events.len() < broad.forwarding_events.len());
        assert!(
            exhausted
                .forwarding_events
                .iter()
                .all(|event| event.receiver_node_id
                    == input.scenario.coded_inference.receiver_node_id)
        );
    }

    #[test]
    fn spray_and_wait_duplicate_pressure_is_bounded_against_epidemic() {
        let spray_input = spray_input(4_096);
        let spray_log = run_spray_and_wait_baseline(&spray_input, spray_params(8)).expect("spray");
        let spray_summary = summarize_spray_and_wait_baseline(
            &spray_input,
            &spray_log,
            BaselinePayloadMode::CodedFragment,
        )
        .expect("spray summary");

        let epidemic_input = epidemic_input(4_096);
        let epidemic_params = EpidemicForwardingParams {
            payload_mode: BaselinePayloadMode::CodedFragment,
            ttl_rounds: 32,
            storage_cap_payload_units: 12,
            per_contact_capacity: 3,
        };
        let epidemic_log =
            run_epidemic_forwarding_baseline(&epidemic_input, epidemic_params).expect("epidemic");
        let epidemic_summary = summarize_epidemic_forwarding_baseline(
            &epidemic_input,
            &epidemic_log,
            BaselinePayloadMode::CodedFragment,
        )
        .expect("epidemic summary");

        assert!(spray_summary.forwarding_events <= epidemic_summary.forwarding_events);
        assert!(spray_summary.duplicate_arrival_count <= epidemic_summary.duplicate_arrival_count);
    }
}
// proc-macro-scope: spray baseline rows are artifact schema, not shared model vocabulary.
