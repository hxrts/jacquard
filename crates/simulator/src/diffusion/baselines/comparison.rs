//! Equal-budget comparison harness for coded-inference baselines.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{
    coded::{
        run_controlled_coded_diffusion_baseline, run_uncontrolled_coded_diffusion_baseline,
        summarize_coded_diffusion_baseline,
    },
    epidemic::{
        run_epidemic_forwarding_baseline, summarize_epidemic_forwarding_baseline,
        EpidemicForwardingParams,
    },
    spray::{
        run_spray_and_wait_baseline, summarize_spray_and_wait_baseline, SprayAndWaitParams,
        SprayDirectDeliveryRule, SpraySplitRule,
    },
    uncoded::{run_uncoded_replication_baseline, summarize_uncoded_replication_baseline},
    BaselineContractError, BaselineFixedBudget, BaselinePayloadMode, BaselinePolicyId,
    BaselineRunInput, BaselineRunSummary, BASELINE_ARTIFACT_NAMESPACE, EQUAL_PAYLOAD_BYTES_LABEL,
};
use crate::diffusion::catalog::scenarios::build_coded_inference_readiness_scenario;

const COMPARISON_PAYLOAD_BYTE_BUDGET: u32 = 4_096;
const REQUIRED_BASELINE_COUNT: u32 = 5;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineComparisonAggregate {
    pub artifact_namespace: String,
    pub family_id: String,
    pub fixed_budget_label: String,
    pub fixed_payload_budget_bytes: u32,
    pub baseline_count: u32,
    pub recovery_probability_permille_sum: u32,
    pub decision_accuracy_permille_sum: u32,
    pub total_bytes_transmitted_sum: u32,
    pub forwarding_events_sum: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BaselineComparisonArtifact {
    pub artifact_namespace: String,
    pub family_id: String,
    pub seed: u64,
    pub fixed_budget_label: String,
    pub fixed_payload_budget_bytes: u32,
    pub summaries: Vec<BaselineRunSummary>,
    pub aggregate: BaselineComparisonAggregate,
}

pub(crate) fn run_equal_budget_baseline_comparison(
    seed: u64,
) -> Result<BaselineComparisonArtifact, BaselineContractError> {
    let scenario = build_coded_inference_readiness_scenario();
    let budget = BaselineFixedBudget::try_new(
        EQUAL_PAYLOAD_BYTES_LABEL,
        COMPARISON_PAYLOAD_BYTE_BUDGET,
        scenario.coded_inference.uncoded_message_payload_bytes,
        scenario.coded_inference.fragment_payload_bytes,
    )?;
    let mut summaries = vec![
        run_uncoded(seed, budget.clone())?,
        run_epidemic(seed, budget.clone())?,
        run_spray(seed, budget.clone())?,
        run_uncontrolled(seed, budget.clone())?,
        run_controlled(seed, budget.clone())?,
    ];
    summaries.sort_by_key(|summary| summary.policy_id);
    validate_required_roster(&summaries)?;
    validate_equal_budget_metadata(&summaries)?;
    let aggregate = aggregate_comparison(&summaries)?;

    Ok(BaselineComparisonArtifact {
        artifact_namespace: BASELINE_ARTIFACT_NAMESPACE.to_string(),
        family_id: scenario.diffusion.family_id,
        seed,
        fixed_budget_label: EQUAL_PAYLOAD_BYTES_LABEL.to_string(),
        fixed_payload_budget_bytes: COMPARISON_PAYLOAD_BYTE_BUDGET,
        summaries,
        aggregate,
    })
}

pub(crate) fn validate_equal_budget_metadata(
    summaries: &[BaselineRunSummary],
) -> Result<(), BaselineContractError> {
    let Some(first) = summaries.first() else {
        return Err(BaselineContractError::MissingRequiredBaseline);
    };
    for summary in summaries {
        if summary.fixed_budget_label != first.fixed_budget_label
            || summary.fixed_payload_budget_bytes != first.fixed_payload_budget_bytes
        {
            return Err(BaselineContractError::UnequalBudgetMetadata);
        }
    }
    Ok(())
}

fn run_uncoded(
    seed: u64,
    budget: BaselineFixedBudget,
) -> Result<BaselineRunSummary, BaselineContractError> {
    let input = comparison_input(seed, BaselinePolicyId::UncodedReplication, budget)?;
    let log = run_uncoded_replication_baseline(&input)?;
    summarize_uncoded_replication_baseline(&input, &log)
}

fn run_epidemic(
    seed: u64,
    budget: BaselineFixedBudget,
) -> Result<BaselineRunSummary, BaselineContractError> {
    let input = comparison_input(seed, BaselinePolicyId::EpidemicForwarding, budget)?;
    let params = EpidemicForwardingParams {
        payload_mode: BaselinePayloadMode::CodedFragment,
        ttl_rounds: 32,
        storage_cap_payload_units: 12,
        per_contact_capacity: 3,
    };
    let log = run_epidemic_forwarding_baseline(&input, params)?;
    summarize_epidemic_forwarding_baseline(&input, &log, BaselinePayloadMode::CodedFragment)
}

fn run_spray(
    seed: u64,
    budget: BaselineFixedBudget,
) -> Result<BaselineRunSummary, BaselineContractError> {
    let input = comparison_input(seed, BaselinePolicyId::SprayAndWait, budget)?;
    let params = SprayAndWaitParams {
        payload_mode: BaselinePayloadMode::CodedFragment,
        initial_copy_count: 8,
        split_rule: SpraySplitRule::BinaryHalve,
        direct_delivery_rule: SprayDirectDeliveryRule::ReceiverOnlyAfterSpray,
        ttl_rounds: 32,
        storage_cap_payload_units: 12,
    };
    let log = run_spray_and_wait_baseline(&input, params)?;
    summarize_spray_and_wait_baseline(&input, &log, BaselinePayloadMode::CodedFragment)
}

fn run_uncontrolled(
    seed: u64,
    budget: BaselineFixedBudget,
) -> Result<BaselineRunSummary, BaselineContractError> {
    let input = comparison_input(seed, BaselinePolicyId::UncontrolledCodedDiffusion, budget)?;
    let (log, readiness_log) = run_uncontrolled_coded_diffusion_baseline(&input)?;
    summarize_coded_diffusion_baseline(&input, &log, &readiness_log, false)
}

fn run_controlled(
    seed: u64,
    budget: BaselineFixedBudget,
) -> Result<BaselineRunSummary, BaselineContractError> {
    let input = comparison_input(seed, BaselinePolicyId::ControlledCodedDiffusion, budget)?;
    let (log, readiness_log) = run_controlled_coded_diffusion_baseline(&input)?;
    summarize_coded_diffusion_baseline(&input, &log, &readiness_log, true)
}

fn comparison_input(
    seed: u64,
    policy_id: BaselinePolicyId,
    budget: BaselineFixedBudget,
) -> Result<BaselineRunInput, BaselineContractError> {
    BaselineRunInput::try_new(
        seed,
        build_coded_inference_readiness_scenario(),
        policy_id,
        budget,
    )
}

fn validate_required_roster(summaries: &[BaselineRunSummary]) -> Result<(), BaselineContractError> {
    let present = summaries
        .iter()
        .map(|summary| summary.policy_id)
        .collect::<BTreeSet<_>>();
    for required in [
        BaselinePolicyId::UncodedReplication,
        BaselinePolicyId::EpidemicForwarding,
        BaselinePolicyId::SprayAndWait,
        BaselinePolicyId::UncontrolledCodedDiffusion,
        BaselinePolicyId::ControlledCodedDiffusion,
    ] {
        if !present.contains(&required) {
            return Err(BaselineContractError::MissingRequiredBaseline);
        }
    }
    Ok(())
}

fn aggregate_comparison(
    summaries: &[BaselineRunSummary],
) -> Result<BaselineComparisonAggregate, BaselineContractError> {
    validate_equal_budget_metadata(summaries)?;
    let Some(first) = summaries.first() else {
        return Err(BaselineContractError::MissingRequiredBaseline);
    };
    Ok(BaselineComparisonAggregate {
        artifact_namespace: BASELINE_ARTIFACT_NAMESPACE.to_string(),
        family_id: first.family_id.clone(),
        fixed_budget_label: first.fixed_budget_label.clone(),
        fixed_payload_budget_bytes: first.fixed_payload_budget_bytes,
        baseline_count: u32::try_from(summaries.len()).unwrap_or(u32::MAX),
        recovery_probability_permille_sum: summaries
            .iter()
            .map(|summary| summary.recovery_probability_permille)
            .fold(0_u32, u32::saturating_add),
        decision_accuracy_permille_sum: summaries
            .iter()
            .map(|summary| summary.decision_accuracy_permille)
            .fold(0_u32, u32::saturating_add),
        total_bytes_transmitted_sum: summaries
            .iter()
            .map(|summary| summary.bytes_transmitted)
            .fold(0_u32, u32::saturating_add),
        forwarding_events_sum: summaries
            .iter()
            .map(|summary| summary.forwarding_events)
            .fold(0_u32, u32::saturating_add),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        run_equal_budget_baseline_comparison, validate_equal_budget_metadata,
        REQUIRED_BASELINE_COUNT,
    };
    use crate::diffusion::baselines::{
        BaselineContractError, BaselinePolicyId, EQUAL_PAYLOAD_BYTES_LABEL,
    };

    #[test]
    fn baseline_comparison_contains_all_required_baselines() {
        let artifact = run_equal_budget_baseline_comparison(41).expect("comparison");
        let policies = artifact
            .summaries
            .iter()
            .map(|summary| summary.policy_id)
            .collect::<Vec<_>>();

        assert_eq!(artifact.aggregate.baseline_count, REQUIRED_BASELINE_COUNT);
        assert_eq!(
            policies,
            vec![
                BaselinePolicyId::UncodedReplication,
                BaselinePolicyId::EpidemicForwarding,
                BaselinePolicyId::SprayAndWait,
                BaselinePolicyId::UncontrolledCodedDiffusion,
                BaselinePolicyId::ControlledCodedDiffusion,
            ]
        );
    }

    #[test]
    fn baseline_comparison_enforces_equal_payload_budget_metadata() {
        let mut artifact = run_equal_budget_baseline_comparison(41).expect("comparison");
        assert!(artifact.summaries.iter().all(|summary| {
            summary.fixed_budget_label == EQUAL_PAYLOAD_BYTES_LABEL
                && summary.fixed_payload_budget_bytes == artifact.fixed_payload_budget_bytes
        }));

        artifact.summaries[0].fixed_payload_budget_bytes = artifact.summaries[0]
            .fixed_payload_budget_bytes
            .saturating_add(1);
        assert_eq!(
            validate_equal_budget_metadata(&artifact.summaries),
            Err(BaselineContractError::UnequalBudgetMetadata)
        );
    }

    #[test]
    fn baseline_comparison_output_is_deterministic_across_replay() {
        let first = run_equal_budget_baseline_comparison(41).expect("first");
        let second = run_equal_budget_baseline_comparison(41).expect("second");

        assert_eq!(first, second);
        assert_eq!(
            first.artifact_namespace,
            "artifacts/coded-inference/baselines"
        );
        assert_eq!(first.fixed_budget_label, EQUAL_PAYLOAD_BYTES_LABEL);
        assert!(first
            .summaries
            .iter()
            .all(|summary| summary.recovery_probability_permille <= 1000
                && summary.decision_accuracy_permille <= 1000));
    }

    #[test]
    fn baseline_comparison_does_not_write_route_analysis_fields() {
        let artifact = run_equal_budget_baseline_comparison(41).expect("comparison");
        let serialized = serde_json::to_string(&artifact).expect("artifact json");

        assert!(!serialized.contains("field_corridor_publication_dependency"));
        assert!(!serialized.contains("private_route_witness_dependency"));
        assert!(!serialized.contains("route_quality_ranking_dependency"));
        assert!(!serialized.contains("routing_analysis_filter_id"));
    }
}
