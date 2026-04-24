//! Coded diffusion baselines over the readiness trace.

use super::{
    BaselineArrivalClassification, BaselineArrivalCounters, BaselineBudgetEvent,
    BaselineContractError, BaselineForwardingEvent, BaselinePayloadDescriptor, BaselinePayloadMode,
    BaselinePolicyClass, BaselineReceiverEvent, BaselineRunInput, BaselineRunLog,
    BaselineRunSummary, BaselineRunSummaryDraft, BaselineStorageEvent,
};
use crate::diffusion::coded_inference::{
    build_coded_inference_readiness_log, summarize_coded_inference_readiness_log,
    CodedArrivalClassification, CodedInferenceReadinessLog,
};

pub(crate) fn run_uncontrolled_coded_diffusion_baseline(
    input: &BaselineRunInput,
) -> Result<(BaselineRunLog, CodedInferenceReadinessLog), BaselineContractError> {
    if input.policy_id.policy_class() != BaselinePolicyClass::CodedUncontrolled {
        return Err(BaselineContractError::PolicyClassMismatch);
    }
    let readiness_log = build_coded_inference_readiness_log(input.seed, &input.scenario);
    let baseline_log = convert_coded_readiness_log(input, &readiness_log)?;
    Ok((baseline_log, readiness_log))
}

pub(crate) fn run_controlled_coded_diffusion_baseline(
    input: &BaselineRunInput,
) -> Result<(BaselineRunLog, CodedInferenceReadinessLog), BaselineContractError> {
    if input.policy_id.policy_class() != BaselinePolicyClass::CodedControlled {
        return Err(BaselineContractError::PolicyClassMismatch);
    }
    let readiness_log = build_coded_inference_readiness_log(input.seed, &input.scenario);
    let baseline_log = convert_coded_readiness_log(input, &readiness_log)?;
    Ok((baseline_log, readiness_log))
}

// long-block-exception: summary assembly mirrors the shared baseline metric schema.
pub(crate) fn summarize_coded_diffusion_baseline(
    input: &BaselineRunInput,
    log: &BaselineRunLog,
    readiness_log: &CodedInferenceReadinessLog,
    controlled: bool,
) -> Result<BaselineRunSummary, BaselineContractError> {
    let readiness_summary = summarize_coded_inference_readiness_log(&input.scenario, readiness_log);
    let counters = receiver_arrival_counters(log);
    let measured_reproduction_permille = Some(readiness_summary.effective_reproduction_permille);
    let target_reproduction_min_permille =
        controlled.then_some(readiness_summary.target_reproduction_min_permille);
    let target_reproduction_max_permille =
        controlled.then_some(readiness_summary.target_reproduction_max_permille);

    BaselineRunSummary::try_from_draft(BaselineRunSummaryDraft {
        artifact_namespace: log.artifact_namespace.clone(),
        family_id: log.family_id.clone(),
        policy_id: Some(input.policy_id),
        payload_mode: Some(BaselinePayloadMode::CodedFragment),
        fixed_budget_label: Some(input.fixed_budget.label.clone()),
        fixed_payload_budget_bytes: Some(input.fixed_budget.payload_byte_budget),
        recovery_probability_permille: Some(readiness_summary.recovery_probability_permille),
        decision_accuracy_permille: Some(readiness_summary.decision_accuracy_permille),
        reconstruction_round: Some(readiness_summary.reconstruction_round),
        commitment_round: Some(readiness_summary.decision_event_round),
        receiver_rank: Some(readiness_summary.receiver_rank),
        top_hypothesis_margin: Some(readiness_summary.top_hypothesis_margin),
        bytes_transmitted: Some(readiness_summary.total_bytes_transmitted),
        forwarding_events: Some(readiness_summary.forwarding_event_count),
        peak_stored_payload_units_per_node: Some(
            readiness_summary
                .peak_storage_pressure_bytes
                .checked_div(input.fixed_budget.fragment_payload_bytes)
                .unwrap_or(0),
        ),
        peak_stored_payload_bytes_per_node: Some(readiness_summary.peak_storage_pressure_bytes),
        duplicate_rate_permille: Some(counters.duplicate_rate_permille()),
        innovative_arrival_rate_permille: Some(counters.innovative_arrival_rate_permille()),
        duplicate_arrival_count: Some(counters.duplicate_arrivals),
        innovative_arrival_count: Some(counters.innovative_arrivals),
        target_reproduction_min_permille,
        target_reproduction_max_permille,
        measured_reproduction_permille,
    })
}

fn convert_coded_readiness_log(
    input: &BaselineRunInput,
    readiness_log: &CodedInferenceReadinessLog,
) -> Result<BaselineRunLog, BaselineContractError> {
    let forwarding_events = readiness_log
        .forwarding_events
        .iter()
        .map(|event| {
            Ok(BaselineForwardingEvent {
                round_index: event.round_index,
                sender_node_id: event.sender_node_id,
                receiver_node_id: event.receiver_node_id,
                policy_id: input.policy_id,
                payload: BaselinePayloadDescriptor::try_new(
                    BaselinePayloadMode::CodedFragment,
                    Some(event.evidence_id),
                    None,
                    event.fragment_id,
                    event.byte_count,
                    event.origin.contribution_ledger_ids.clone(),
                )?,
                classification: convert_arrival_class(event.classification),
            })
        })
        .collect::<Result<Vec<_>, BaselineContractError>>()?;
    let receiver_events = readiness_log
        .receiver_events
        .iter()
        .map(|event| BaselineReceiverEvent {
            round_index: event.round_index,
            receiver_node_id: event.receiver_node_id,
            policy_id: input.policy_id,
            arrival_classification: if event.rank_after > event.rank_before {
                BaselineArrivalClassification::Innovative
            } else {
                BaselineArrivalClassification::Duplicate
            },
            rank_before: event.rank_before,
            rank_after: event.rank_after,
            reconstruction_round: event.reconstruction_event_round,
            commitment_round: event.decision_event_round,
        })
        .collect();
    let storage_events = readiness_log
        .budget_events
        .iter()
        .map(|event| BaselineStorageEvent {
            round_index: event.round_index,
            node_id: input.scenario.coded_inference.receiver_node_id,
            policy_id: input.policy_id,
            stored_payload_units: event
                .retained_bytes
                .checked_div(input.fixed_budget.fragment_payload_bytes)
                .unwrap_or(0),
            stored_payload_bytes: event.retained_bytes,
        })
        .collect();
    let budget_events = readiness_log
        .budget_events
        .iter()
        .scan(0_u32, |cumulative, event| {
            *cumulative = cumulative.saturating_add(event.payload_bytes_spent);
            Some(BaselineBudgetEvent {
                round_index: event.round_index,
                policy_id: input.policy_id,
                payload_bytes_spent: event.payload_bytes_spent,
                cumulative_payload_bytes_spent: *cumulative,
                fixed_budget_label: event.fixed_budget_label.clone(),
                fixed_payload_budget_bytes: input.fixed_budget.payload_byte_budget,
            })
        })
        .collect();

    Ok(BaselineRunLog {
        artifact_namespace: input.artifact_namespace.clone(),
        family_id: input.scenario.diffusion.family_id.clone(),
        policy_id: input.policy_id,
        forwarding_events,
        receiver_events,
        storage_events,
        budget_events,
    })
}

fn convert_arrival_class(
    classification: CodedArrivalClassification,
) -> BaselineArrivalClassification {
    match classification {
        CodedArrivalClassification::Innovative => BaselineArrivalClassification::Innovative,
        CodedArrivalClassification::Duplicate => BaselineArrivalClassification::Duplicate,
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

#[cfg(test)]
mod tests {
    use super::{
        run_controlled_coded_diffusion_baseline, run_uncontrolled_coded_diffusion_baseline,
        summarize_coded_diffusion_baseline, BaselinePayloadMode,
    };
    use crate::diffusion::{
        baselines::{
            BaselineFixedBudget, BaselinePolicyId, BaselineRunInput, EQUAL_PAYLOAD_BYTES_LABEL,
        },
        catalog::scenarios::build_coded_inference_readiness_scenario,
        model::CodedEvidenceOriginMode,
    };

    fn coded_input(policy_id: BaselinePolicyId) -> BaselineRunInput {
        let scenario = build_coded_inference_readiness_scenario();
        let budget = BaselineFixedBudget::try_new(
            EQUAL_PAYLOAD_BYTES_LABEL,
            scenario
                .coded_inference
                .source_fragment_count
                .saturating_mul(scenario.coded_inference.fragment_payload_bytes),
            scenario.coded_inference.uncoded_message_payload_bytes,
            scenario.coded_inference.fragment_payload_bytes,
        )
        .expect("budget");
        BaselineRunInput::try_new(41, scenario, policy_id, budget).expect("input")
    }

    #[test]
    fn uncontrolled_coded_runs_on_same_trace_as_controlled_policy() {
        let uncontrolled_input = coded_input(BaselinePolicyId::UncontrolledCodedDiffusion);
        let controlled_input = coded_input(BaselinePolicyId::ControlledCodedDiffusion);
        let (uncontrolled_log, uncontrolled_readiness) =
            run_uncontrolled_coded_diffusion_baseline(&uncontrolled_input).expect("uncontrolled");
        let (controlled_log, controlled_readiness) =
            run_controlled_coded_diffusion_baseline(&controlled_input).expect("controlled");

        assert_eq!(
            uncontrolled_readiness.contact_events,
            controlled_readiness.contact_events
        );
        assert_eq!(
            uncontrolled_log.forwarding_events.len(),
            controlled_log.forwarding_events.len()
        );
        assert_eq!(
            uncontrolled_readiness.receiver_events,
            controlled_readiness.receiver_events
        );
    }

    #[test]
    fn uncontrolled_coded_logs_measured_reproduction_without_target_control() {
        let input = coded_input(BaselinePolicyId::UncontrolledCodedDiffusion);
        let (log, readiness_log) =
            run_uncontrolled_coded_diffusion_baseline(&input).expect("uncontrolled");
        let summary = summarize_coded_diffusion_baseline(&input, &log, &readiness_log, false)
            .expect("summary");

        assert_eq!(
            summary.policy_id,
            BaselinePolicyId::UncontrolledCodedDiffusion
        );
        assert_eq!(summary.payload_mode, BaselinePayloadMode::CodedFragment);
        assert!(summary.measured_reproduction_permille.is_some());
        assert_eq!(summary.target_reproduction_min_permille, None);
        assert_eq!(summary.target_reproduction_max_permille, None);
    }

    #[test]
    fn uncontrolled_coded_preserves_landscape_and_origin_semantics() {
        let input = coded_input(BaselinePolicyId::UncontrolledCodedDiffusion);
        let (log, readiness_log) =
            run_uncontrolled_coded_diffusion_baseline(&input).expect("uncontrolled");
        let summary = summarize_coded_diffusion_baseline(&input, &log, &readiness_log, false)
            .expect("summary");

        assert!(summary.receiver_rank >= input.scenario.coded_inference.reconstruction_threshold);
        assert!(
            summary.top_hypothesis_margin
                >= input.scenario.coded_inference.decision_margin_threshold
        );
        assert!(readiness_log.forwarding_events.iter().any(|event| {
            event.origin.origin_mode == CodedEvidenceOriginMode::RecodedAggregate
        }));
        assert!(summary.duplicate_arrival_count > 0);
    }

    #[test]
    fn controlled_coded_reports_target_and_measured_reproduction_fields() {
        let input = coded_input(BaselinePolicyId::ControlledCodedDiffusion);
        let (log, readiness_log) =
            run_controlled_coded_diffusion_baseline(&input).expect("controlled");
        let summary = summarize_coded_diffusion_baseline(&input, &log, &readiness_log, true)
            .expect("summary");

        assert_eq!(
            summary.policy_id,
            BaselinePolicyId::ControlledCodedDiffusion
        );
        assert_eq!(summary.target_reproduction_min_permille, Some(800));
        assert_eq!(summary.target_reproduction_max_permille, Some(1200));
        assert!(summary.measured_reproduction_permille.is_some());
        assert_ne!(
            summary.measured_reproduction_permille,
            summary.target_reproduction_min_permille
        );
    }

    #[test]
    fn controlled_coded_replay_is_deterministic() {
        let input = coded_input(BaselinePolicyId::ControlledCodedDiffusion);
        let first = run_controlled_coded_diffusion_baseline(&input).expect("first");
        let second = run_controlled_coded_diffusion_baseline(&input).expect("second");

        assert_eq!(first, second);
    }

    #[test]
    fn controlled_and_uncontrolled_coded_report_comparable_metrics() {
        let controlled_input = coded_input(BaselinePolicyId::ControlledCodedDiffusion);
        let uncontrolled_input = coded_input(BaselinePolicyId::UncontrolledCodedDiffusion);
        let (controlled_log, controlled_readiness) =
            run_controlled_coded_diffusion_baseline(&controlled_input).expect("controlled");
        let (uncontrolled_log, uncontrolled_readiness) =
            run_uncontrolled_coded_diffusion_baseline(&uncontrolled_input).expect("uncontrolled");
        let controlled = summarize_coded_diffusion_baseline(
            &controlled_input,
            &controlled_log,
            &controlled_readiness,
            true,
        )
        .expect("controlled summary");
        let uncontrolled = summarize_coded_diffusion_baseline(
            &uncontrolled_input,
            &uncontrolled_log,
            &uncontrolled_readiness,
            false,
        )
        .expect("uncontrolled summary");

        assert_eq!(controlled.payload_mode, uncontrolled.payload_mode);
        assert_eq!(
            controlled.fixed_payload_budget_bytes,
            uncontrolled.fixed_payload_budget_bytes
        );
        assert_eq!(controlled.receiver_rank, uncontrolled.receiver_rank);
        assert_eq!(
            controlled.top_hypothesis_margin,
            uncontrolled.top_hypothesis_margin
        );
        assert_eq!(
            controlled.measured_reproduction_permille,
            uncontrolled.measured_reproduction_permille
        );
    }

    #[test]
    fn controlled_coded_does_not_write_route_analysis_fields() {
        let input = coded_input(BaselinePolicyId::ControlledCodedDiffusion);
        let (log, readiness_log) =
            run_controlled_coded_diffusion_baseline(&input).expect("controlled");
        let summary = summarize_coded_diffusion_baseline(&input, &log, &readiness_log, true)
            .expect("summary");
        let serialized = serde_json::to_string(&summary).expect("summary json");

        assert!(!serialized.contains("field_corridor_publication_dependency"));
        assert!(!serialized.contains("private_route_witness_dependency"));
        assert!(!serialized.contains("route_quality_ranking_dependency"));
        assert!(!serialized.contains("routing_analysis_filter_id"));
    }
}
