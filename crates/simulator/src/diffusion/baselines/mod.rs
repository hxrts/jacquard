//! Shared deterministic baseline contract for coded-inference comparisons.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::model::CodedInferenceReadinessScenario;

mod coded;
mod comparison;
mod epidemic;
mod spray;
mod uncoded;

pub(crate) const BASELINE_ARTIFACT_NAMESPACE: &str = "artifacts/coded-inference/baselines";
pub(crate) const EQUAL_PAYLOAD_BYTES_LABEL: &str = "equal-payload-bytes";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum BaselinePolicyId {
    UncodedReplication,
    EpidemicForwarding,
    SprayAndWait,
    UncontrolledCodedDiffusion,
    ControlledCodedDiffusion,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum BaselinePolicyClass {
    Replication,
    Flooding,
    BoundedCopy,
    CodedUncontrolled,
    CodedControlled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum BaselinePayloadMode {
    UncodedWholeMessage,
    CodedFragment,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum BaselineArrivalClassification {
    Innovative,
    Duplicate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BaselineContractError {
    EmptyBudgetLabel,
    UnknownBaselineId,
    ZeroPayloadByteBudget,
    ZeroWholeMessageBytes,
    ZeroFragmentBytes,
    MalformedBudgetMetadata,
    EmptyContributionLedger,
    MissingMetric(BaselineMetricField),
    PolicyClassMismatch,
    MissingRequiredBaseline,
    UnequalBudgetMetadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BaselineMetricField {
    PolicyId,
    PayloadMode,
    FixedBudgetLabel,
    FixedPayloadBudgetBytes,
    RecoveryProbabilityPermille,
    DecisionAccuracyPermille,
    ReconstructionRound,
    CommitmentRound,
    ReceiverRank,
    TopHypothesisMargin,
    BytesTransmitted,
    ForwardingEvents,
    PeakStoredPayloadUnitsPerNode,
    PeakStoredPayloadBytesPerNode,
    DuplicateRatePermille,
    InnovativeArrivalRatePermille,
    DuplicateArrivalCount,
    InnovativeArrivalCount,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineFixedBudget {
    pub label: String,
    pub payload_byte_budget: u32,
    pub whole_message_payload_bytes: u32,
    pub fragment_payload_bytes: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BaselineRunInput {
    pub artifact_namespace: String,
    pub seed: u64,
    pub scenario: CodedInferenceReadinessScenario,
    pub policy_id: BaselinePolicyId,
    pub fixed_budget: BaselineFixedBudget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselinePayloadDescriptor {
    pub payload_mode: BaselinePayloadMode,
    pub evidence_id: Option<u32>,
    pub message_copy_id: Option<u32>,
    pub fragment_id: Option<u32>,
    pub byte_count: u32,
    pub contribution_ledger_ids: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineForwardingEvent {
    pub round_index: u32,
    pub sender_node_id: u32,
    pub receiver_node_id: u32,
    pub policy_id: BaselinePolicyId,
    pub payload: BaselinePayloadDescriptor,
    pub classification: BaselineArrivalClassification,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineReceiverEvent {
    pub round_index: u32,
    pub receiver_node_id: u32,
    pub policy_id: BaselinePolicyId,
    pub arrival_classification: BaselineArrivalClassification,
    pub rank_before: u32,
    pub rank_after: u32,
    pub reconstruction_round: Option<u32>,
    pub commitment_round: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineStorageEvent {
    pub round_index: u32,
    pub node_id: u32,
    pub policy_id: BaselinePolicyId,
    pub stored_payload_units: u32,
    pub stored_payload_bytes: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineBudgetEvent {
    pub round_index: u32,
    pub policy_id: BaselinePolicyId,
    pub payload_bytes_spent: u32,
    pub cumulative_payload_bytes_spent: u32,
    pub fixed_budget_label: String,
    pub fixed_payload_budget_bytes: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineRunLog {
    pub artifact_namespace: String,
    pub family_id: String,
    pub policy_id: BaselinePolicyId,
    pub forwarding_events: Vec<BaselineForwardingEvent>,
    pub receiver_events: Vec<BaselineReceiverEvent>,
    pub storage_events: Vec<BaselineStorageEvent>,
    pub budget_events: Vec<BaselineBudgetEvent>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineRunSummary {
    pub artifact_namespace: String,
    pub family_id: String,
    pub policy_id: BaselinePolicyId,
    pub policy_class: BaselinePolicyClass,
    pub payload_mode: BaselinePayloadMode,
    pub fixed_budget_label: String,
    pub fixed_payload_budget_bytes: u32,
    pub recovery_probability_permille: u32,
    pub decision_accuracy_permille: u32,
    pub reconstruction_round: Option<u32>,
    pub commitment_round: Option<u32>,
    pub receiver_rank: u32,
    pub top_hypothesis_margin: i32,
    pub bytes_transmitted: u32,
    pub forwarding_events: u32,
    pub peak_stored_payload_units_per_node: u32,
    pub peak_stored_payload_bytes_per_node: u32,
    pub duplicate_rate_permille: u32,
    pub innovative_arrival_rate_permille: u32,
    pub duplicate_arrival_count: u32,
    pub innovative_arrival_count: u32,
    pub target_reproduction_min_permille: Option<u32>,
    pub target_reproduction_max_permille: Option<u32>,
    pub measured_reproduction_permille: Option<u32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BaselineRunSummaryDraft {
    pub artifact_namespace: String,
    pub family_id: String,
    pub policy_id: Option<BaselinePolicyId>,
    pub payload_mode: Option<BaselinePayloadMode>,
    pub fixed_budget_label: Option<String>,
    pub fixed_payload_budget_bytes: Option<u32>,
    pub recovery_probability_permille: Option<u32>,
    pub decision_accuracy_permille: Option<u32>,
    pub reconstruction_round: Option<Option<u32>>,
    pub commitment_round: Option<Option<u32>>,
    pub receiver_rank: Option<u32>,
    pub top_hypothesis_margin: Option<i32>,
    pub bytes_transmitted: Option<u32>,
    pub forwarding_events: Option<u32>,
    pub peak_stored_payload_units_per_node: Option<u32>,
    pub peak_stored_payload_bytes_per_node: Option<u32>,
    pub duplicate_rate_permille: Option<u32>,
    pub innovative_arrival_rate_permille: Option<u32>,
    pub duplicate_arrival_count: Option<u32>,
    pub innovative_arrival_count: Option<u32>,
    pub target_reproduction_min_permille: Option<u32>,
    pub target_reproduction_max_permille: Option<u32>,
    pub measured_reproduction_permille: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BaselineArrivalCounters {
    pub innovative_arrivals: u32,
    pub duplicate_arrivals: u32,
}

impl BaselinePolicyId {
    pub(crate) fn try_from_str(value: &str) -> Result<Self, BaselineContractError> {
        match value {
            "uncoded-replication" => Ok(Self::UncodedReplication),
            "epidemic-forwarding" => Ok(Self::EpidemicForwarding),
            "spray-and-wait" => Ok(Self::SprayAndWait),
            "uncontrolled-coded-diffusion" => Ok(Self::UncontrolledCodedDiffusion),
            "controlled-coded-diffusion" => Ok(Self::ControlledCodedDiffusion),
            _ => Err(BaselineContractError::UnknownBaselineId),
        }
    }

    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::UncodedReplication => "uncoded-replication",
            Self::EpidemicForwarding => "epidemic-forwarding",
            Self::SprayAndWait => "spray-and-wait",
            Self::UncontrolledCodedDiffusion => "uncontrolled-coded-diffusion",
            Self::ControlledCodedDiffusion => "controlled-coded-diffusion",
        }
    }

    #[must_use]
    pub(crate) fn policy_class(self) -> BaselinePolicyClass {
        match self {
            Self::UncodedReplication => BaselinePolicyClass::Replication,
            Self::EpidemicForwarding => BaselinePolicyClass::Flooding,
            Self::SprayAndWait => BaselinePolicyClass::BoundedCopy,
            Self::UncontrolledCodedDiffusion => BaselinePolicyClass::CodedUncontrolled,
            Self::ControlledCodedDiffusion => BaselinePolicyClass::CodedControlled,
        }
    }
}

impl BaselinePayloadMode {
    #[must_use]
    pub(crate) fn byte_count(self, budget: &BaselineFixedBudget) -> u32 {
        match self {
            Self::UncodedWholeMessage => budget.whole_message_payload_bytes,
            Self::CodedFragment => budget.fragment_payload_bytes,
        }
    }
}

impl BaselineFixedBudget {
    pub(crate) fn try_new(
        label: impl Into<String>,
        payload_byte_budget: u32,
        whole_message_payload_bytes: u32,
        fragment_payload_bytes: u32,
    ) -> Result<Self, BaselineContractError> {
        let label = label.into();
        if label.is_empty() {
            return Err(BaselineContractError::EmptyBudgetLabel);
        }
        if payload_byte_budget == 0 {
            return Err(BaselineContractError::ZeroPayloadByteBudget);
        }
        if whole_message_payload_bytes == 0 {
            return Err(BaselineContractError::ZeroWholeMessageBytes);
        }
        if fragment_payload_bytes == 0 {
            return Err(BaselineContractError::ZeroFragmentBytes);
        }
        Ok(Self {
            label,
            payload_byte_budget,
            whole_message_payload_bytes,
            fragment_payload_bytes,
        })
    }
}

impl BaselineRunInput {
    pub(crate) fn try_new(
        seed: u64,
        scenario: CodedInferenceReadinessScenario,
        policy_id: BaselinePolicyId,
        fixed_budget: BaselineFixedBudget,
    ) -> Result<Self, BaselineContractError> {
        validate_budget_for_scenario(&scenario, &fixed_budget)?;
        Ok(Self {
            artifact_namespace: BASELINE_ARTIFACT_NAMESPACE.to_string(),
            seed,
            scenario,
            policy_id,
            fixed_budget,
        })
    }
}

impl BaselinePayloadDescriptor {
    pub(crate) fn try_new(
        payload_mode: BaselinePayloadMode,
        evidence_id: Option<u32>,
        message_copy_id: Option<u32>,
        fragment_id: Option<u32>,
        byte_count: u32,
        contribution_ledger_ids: Vec<u32>,
    ) -> Result<Self, BaselineContractError> {
        if byte_count == 0 {
            return Err(BaselineContractError::ZeroPayloadByteBudget);
        }
        if contribution_ledger_ids.is_empty() {
            return Err(BaselineContractError::EmptyContributionLedger);
        }
        Ok(Self {
            payload_mode,
            evidence_id,
            message_copy_id,
            fragment_id,
            byte_count,
            contribution_ledger_ids,
        })
    }
}

impl BaselineArrivalCounters {
    #[must_use]
    pub(crate) fn total_arrivals(self) -> u32 {
        self.innovative_arrivals
            .saturating_add(self.duplicate_arrivals)
    }

    #[must_use]
    pub(crate) fn duplicate_rate_permille(self) -> u32 {
        ratio_permille(self.duplicate_arrivals, self.total_arrivals())
    }

    #[must_use]
    pub(crate) fn innovative_arrival_rate_permille(self) -> u32 {
        ratio_permille(self.innovative_arrivals, self.total_arrivals())
    }
}

impl BaselineRunSummary {
    // long-block-exception: summary validation mirrors the required baseline metric schema.
    pub(crate) fn try_from_draft(
        draft: BaselineRunSummaryDraft,
    ) -> Result<Self, BaselineContractError> {
        let policy_id = require(draft.policy_id, BaselineMetricField::PolicyId)?;
        let fixed_budget_label = require(
            draft.fixed_budget_label,
            BaselineMetricField::FixedBudgetLabel,
        )?;
        if fixed_budget_label.is_empty() {
            return Err(BaselineContractError::EmptyBudgetLabel);
        }
        Ok(Self {
            artifact_namespace: draft.artifact_namespace,
            family_id: draft.family_id,
            policy_id,
            policy_class: policy_id.policy_class(),
            payload_mode: require(draft.payload_mode, BaselineMetricField::PayloadMode)?,
            fixed_budget_label,
            fixed_payload_budget_bytes: require_positive(
                draft.fixed_payload_budget_bytes,
                BaselineMetricField::FixedPayloadBudgetBytes,
            )?,
            recovery_probability_permille: require(
                draft.recovery_probability_permille,
                BaselineMetricField::RecoveryProbabilityPermille,
            )?,
            decision_accuracy_permille: require(
                draft.decision_accuracy_permille,
                BaselineMetricField::DecisionAccuracyPermille,
            )?,
            reconstruction_round: require(
                draft.reconstruction_round,
                BaselineMetricField::ReconstructionRound,
            )?,
            commitment_round: require(
                draft.commitment_round,
                BaselineMetricField::CommitmentRound,
            )?,
            receiver_rank: require(draft.receiver_rank, BaselineMetricField::ReceiverRank)?,
            top_hypothesis_margin: require(
                draft.top_hypothesis_margin,
                BaselineMetricField::TopHypothesisMargin,
            )?,
            bytes_transmitted: require(
                draft.bytes_transmitted,
                BaselineMetricField::BytesTransmitted,
            )?,
            forwarding_events: require(
                draft.forwarding_events,
                BaselineMetricField::ForwardingEvents,
            )?,
            peak_stored_payload_units_per_node: require(
                draft.peak_stored_payload_units_per_node,
                BaselineMetricField::PeakStoredPayloadUnitsPerNode,
            )?,
            peak_stored_payload_bytes_per_node: require(
                draft.peak_stored_payload_bytes_per_node,
                BaselineMetricField::PeakStoredPayloadBytesPerNode,
            )?,
            duplicate_rate_permille: require(
                draft.duplicate_rate_permille,
                BaselineMetricField::DuplicateRatePermille,
            )?,
            innovative_arrival_rate_permille: require(
                draft.innovative_arrival_rate_permille,
                BaselineMetricField::InnovativeArrivalRatePermille,
            )?,
            duplicate_arrival_count: require(
                draft.duplicate_arrival_count,
                BaselineMetricField::DuplicateArrivalCount,
            )?,
            innovative_arrival_count: require(
                draft.innovative_arrival_count,
                BaselineMetricField::InnovativeArrivalCount,
            )?,
            target_reproduction_min_permille: draft.target_reproduction_min_permille,
            target_reproduction_max_permille: draft.target_reproduction_max_permille,
            measured_reproduction_permille: draft.measured_reproduction_permille,
        })
    }
}

fn validate_budget_for_scenario(
    scenario: &CodedInferenceReadinessScenario,
    budget: &BaselineFixedBudget,
) -> Result<(), BaselineContractError> {
    let inference = &scenario.coded_inference;
    if budget.label.is_empty() {
        return Err(BaselineContractError::EmptyBudgetLabel);
    }
    if budget.payload_byte_budget == 0 {
        return Err(BaselineContractError::ZeroPayloadByteBudget);
    }
    if budget.whole_message_payload_bytes != inference.uncoded_message_payload_bytes {
        return Err(BaselineContractError::MalformedBudgetMetadata);
    }
    if budget.fragment_payload_bytes != inference.fragment_payload_bytes {
        return Err(BaselineContractError::MalformedBudgetMetadata);
    }
    Ok(())
}

fn require<T>(value: Option<T>, field: BaselineMetricField) -> Result<T, BaselineContractError> {
    value.ok_or(BaselineContractError::MissingMetric(field))
}

fn require_positive(
    value: Option<u32>,
    field: BaselineMetricField,
) -> Result<u32, BaselineContractError> {
    let value = require(value, field)?;
    if value == 0 {
        return Err(BaselineContractError::ZeroPayloadByteBudget);
    }
    Ok(value)
}

fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        0
    } else {
        numerator.saturating_mul(1000) / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BaselineArrivalCounters, BaselineContractError, BaselineFixedBudget, BaselineMetricField,
        BaselinePayloadDescriptor, BaselinePayloadMode, BaselinePolicyClass, BaselinePolicyId,
        BaselineRunInput, BaselineRunSummary, BaselineRunSummaryDraft, BASELINE_ARTIFACT_NAMESPACE,
        EQUAL_PAYLOAD_BYTES_LABEL,
    };
    use crate::diffusion::catalog::scenarios::build_coded_inference_readiness_scenario;

    fn readiness_budget() -> BaselineFixedBudget {
        let scenario = build_coded_inference_readiness_scenario();
        BaselineFixedBudget::try_new(
            EQUAL_PAYLOAD_BYTES_LABEL,
            scenario
                .coded_inference
                .source_fragment_count
                .saturating_mul(scenario.coded_inference.fragment_payload_bytes),
            scenario.coded_inference.uncoded_message_payload_bytes,
            scenario.coded_inference.fragment_payload_bytes,
        )
        .expect("baseline budget")
    }

    fn complete_summary_draft() -> BaselineRunSummaryDraft {
        BaselineRunSummaryDraft {
            artifact_namespace: BASELINE_ARTIFACT_NAMESPACE.to_string(),
            family_id: "coded-inference-100-node-readiness".to_string(),
            policy_id: Some(BaselinePolicyId::ControlledCodedDiffusion),
            payload_mode: Some(BaselinePayloadMode::CodedFragment),
            fixed_budget_label: Some(EQUAL_PAYLOAD_BYTES_LABEL.to_string()),
            fixed_payload_budget_bytes: Some(384),
            recovery_probability_permille: Some(1000),
            decision_accuracy_permille: Some(1000),
            reconstruction_round: Some(Some(18)),
            commitment_round: Some(Some(16)),
            receiver_rank: Some(8),
            top_hypothesis_margin: Some(24),
            bytes_transmitted: Some(384),
            forwarding_events: Some(12),
            peak_stored_payload_units_per_node: Some(3),
            peak_stored_payload_bytes_per_node: Some(96),
            duplicate_rate_permille: Some(250),
            innovative_arrival_rate_permille: Some(750),
            duplicate_arrival_count: Some(2),
            innovative_arrival_count: Some(6),
            target_reproduction_min_permille: Some(800),
            target_reproduction_max_permille: Some(1200),
            measured_reproduction_permille: Some(950),
        }
    }

    #[test]
    fn baseline_contract_identifies_required_policy_roster() {
        let roster = [
            (
                "uncoded-replication",
                BaselinePolicyId::UncodedReplication,
                BaselinePolicyClass::Replication,
            ),
            (
                "epidemic-forwarding",
                BaselinePolicyId::EpidemicForwarding,
                BaselinePolicyClass::Flooding,
            ),
            (
                "spray-and-wait",
                BaselinePolicyId::SprayAndWait,
                BaselinePolicyClass::BoundedCopy,
            ),
            (
                "uncontrolled-coded-diffusion",
                BaselinePolicyId::UncontrolledCodedDiffusion,
                BaselinePolicyClass::CodedUncontrolled,
            ),
            (
                "controlled-coded-diffusion",
                BaselinePolicyId::ControlledCodedDiffusion,
                BaselinePolicyClass::CodedControlled,
            ),
        ];

        for (policy_name, expected_id, expected_class) in roster {
            let policy_id = BaselinePolicyId::try_from_str(policy_name).expect("policy id");
            assert_eq!(policy_id, expected_id);
            assert_eq!(policy_id.as_str(), policy_name);
            assert_eq!(policy_id.policy_class(), expected_class);
        }
        assert_eq!(
            BaselinePolicyId::try_from_str("direct-delivery"),
            Err(BaselineContractError::UnknownBaselineId)
        );
    }

    #[test]
    fn baseline_contract_validates_equal_payload_byte_budget() {
        let scenario = build_coded_inference_readiness_scenario();
        let budget = readiness_budget();
        let input = BaselineRunInput::try_new(
            41,
            scenario,
            BaselinePolicyId::ControlledCodedDiffusion,
            budget.clone(),
        )
        .expect("baseline input");

        assert_eq!(input.artifact_namespace, BASELINE_ARTIFACT_NAMESPACE);
        assert_eq!(input.fixed_budget.label, EQUAL_PAYLOAD_BYTES_LABEL);
        assert_eq!(input.fixed_budget.payload_byte_budget, 384);
        assert_eq!(input.fixed_budget.whole_message_payload_bytes, 384);
        assert_eq!(input.fixed_budget.fragment_payload_bytes, 32);
        assert_eq!(
            BaselineFixedBudget::try_new("", 384, 384, 32),
            Err(BaselineContractError::EmptyBudgetLabel)
        );
        assert_eq!(
            BaselineFixedBudget::try_new(EQUAL_PAYLOAD_BYTES_LABEL, 0, 384, 32),
            Err(BaselineContractError::ZeroPayloadByteBudget)
        );
        let mismatched_budget =
            BaselineFixedBudget::try_new(EQUAL_PAYLOAD_BYTES_LABEL, 384, 128, 32)
                .expect("mismatched budget shape");
        assert_eq!(
            BaselineRunInput::try_new(
                41,
                build_coded_inference_readiness_scenario(),
                BaselinePolicyId::ControlledCodedDiffusion,
                mismatched_budget,
            ),
            Err(BaselineContractError::MalformedBudgetMetadata)
        );
    }

    #[test]
    fn baseline_contract_accounts_for_uncoded_and_coded_payload_bytes() {
        let budget = readiness_budget();
        assert_eq!(
            BaselinePayloadMode::UncodedWholeMessage.byte_count(&budget),
            384
        );
        assert_eq!(BaselinePayloadMode::CodedFragment.byte_count(&budget), 32);

        let whole_message = BaselinePayloadDescriptor::try_new(
            BaselinePayloadMode::UncodedWholeMessage,
            None,
            Some(1),
            None,
            BaselinePayloadMode::UncodedWholeMessage.byte_count(&budget),
            vec![1],
        )
        .expect("whole-message payload");
        let coded_fragment = BaselinePayloadDescriptor::try_new(
            BaselinePayloadMode::CodedFragment,
            Some(7),
            None,
            Some(7),
            BaselinePayloadMode::CodedFragment.byte_count(&budget),
            vec![7],
        )
        .expect("fragment payload");

        assert!(whole_message.byte_count > coded_fragment.byte_count);
        assert_eq!(
            BaselinePayloadDescriptor::try_new(
                BaselinePayloadMode::CodedFragment,
                Some(1),
                None,
                Some(1),
                32,
                Vec::new(),
            ),
            Err(BaselineContractError::EmptyContributionLedger)
        );
    }

    #[test]
    fn baseline_contract_defines_duplicate_and_innovative_rates() {
        let counters = BaselineArrivalCounters {
            innovative_arrivals: 6,
            duplicate_arrivals: 2,
        };

        assert_eq!(counters.total_arrivals(), 8);
        assert_eq!(counters.duplicate_rate_permille(), 250);
        assert_eq!(counters.innovative_arrival_rate_permille(), 750);
        assert_eq!(
            BaselineArrivalCounters::default().duplicate_rate_permille(),
            0
        );
    }

    #[test]
    fn baseline_contract_summary_schema_is_deterministic_and_complete() {
        let first = BaselineRunSummary::try_from_draft(complete_summary_draft()).expect("summary");
        let second = BaselineRunSummary::try_from_draft(complete_summary_draft()).expect("summary");

        assert_eq!(first, second);
        assert_eq!(first.artifact_namespace, BASELINE_ARTIFACT_NAMESPACE);
        assert_eq!(first.policy_id, BaselinePolicyId::ControlledCodedDiffusion);
        assert_eq!(first.policy_class, BaselinePolicyClass::CodedControlled);
        assert_eq!(first.payload_mode, BaselinePayloadMode::CodedFragment);
        assert_eq!(first.fixed_budget_label, EQUAL_PAYLOAD_BYTES_LABEL);
        assert_eq!(first.recovery_probability_permille, 1000);
        assert_eq!(first.decision_accuracy_permille, 1000);
        assert_eq!(first.reconstruction_round, Some(18));
        assert_eq!(first.commitment_round, Some(16));
        assert_eq!(first.receiver_rank, 8);
        assert_eq!(first.top_hypothesis_margin, 24);
        assert_eq!(first.bytes_transmitted, 384);
        assert_eq!(first.forwarding_events, 12);
        assert_eq!(first.peak_stored_payload_units_per_node, 3);
        assert_eq!(first.peak_stored_payload_bytes_per_node, 96);
        assert_eq!(first.duplicate_rate_permille, 250);
        assert_eq!(first.innovative_arrival_rate_permille, 750);
        assert_eq!(first.duplicate_arrival_count, 2);
        assert_eq!(first.innovative_arrival_count, 6);
        assert_eq!(first.target_reproduction_min_permille, Some(800));
        assert_eq!(first.target_reproduction_max_permille, Some(1200));
        assert_eq!(first.measured_reproduction_permille, Some(950));
    }

    #[test]
    fn baseline_contract_rejects_missing_or_malformed_summary_metrics() {
        let mut missing = complete_summary_draft();
        missing.receiver_rank = None;
        assert_eq!(
            BaselineRunSummary::try_from_draft(missing),
            Err(BaselineContractError::MissingMetric(
                BaselineMetricField::ReceiverRank
            ))
        );

        let mut malformed = complete_summary_draft();
        malformed.fixed_budget_label = Some(String::new());
        assert_eq!(
            BaselineRunSummary::try_from_draft(malformed),
            Err(BaselineContractError::EmptyBudgetLabel)
        );

        let mut zero_budget = complete_summary_draft();
        zero_budget.fixed_payload_budget_bytes = Some(0);
        assert_eq!(
            BaselineRunSummary::try_from_draft(zero_budget),
            Err(BaselineContractError::ZeroPayloadByteBudget)
        );
    }
}
