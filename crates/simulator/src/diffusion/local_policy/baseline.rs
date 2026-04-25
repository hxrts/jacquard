//! Local evidence policy adapter for the baseline comparison harness.

use super::{
    local_policy_state_from_trace, reduce_local_policy_forwarding, LocalPolicyArrivalKind,
    LocalPolicyDecisionRecord, LocalPolicyFragmentCandidate, LocalPolicyPeerCandidate,
    LocalPolicyReducerBudget, LocalPolicyStateTraceEvent,
};
use crate::diffusion::{
    baselines::{
        BaselineArrivalCounters, BaselineContractError, BaselinePayloadMode, BaselinePolicyId,
        BaselineRunInput, BaselineRunSummary, BaselineRunSummaryDraft,
    },
    coded_inference::{
        build_coded_inference_readiness_log, summarize_coded_inference_readiness_log,
        CodedArrivalClassification,
    },
};

pub(crate) fn run_local_evidence_policy_baseline(
    input: &BaselineRunInput,
) -> Result<(BaselineRunSummary, Vec<LocalPolicyDecisionRecord>), BaselineContractError> {
    let readiness_log = build_coded_inference_readiness_log(input.seed, &input.scenario);
    let readiness_summary =
        summarize_coded_inference_readiness_log(&input.scenario, &readiness_log);
    let trace = state_trace(input, &readiness_log);
    let state = local_policy_state_from_trace(
        input.scenario.coded_inference.receiver_node_id,
        input.scenario.diffusion.payload_bytes.saturating_mul(16),
        &trace,
    )
    .map_err(|_| BaselineContractError::MalformedBudgetMetadata)?;
    let peer_candidates = peer_candidates(&state);
    let fragment_candidates = fragment_candidates(input, &readiness_log);
    let decisions = reduce_local_policy_forwarding(
        &state,
        &peer_candidates,
        &fragment_candidates,
        LocalPolicyReducerBudget {
            payload_byte_budget_remaining: input.fixed_budget.payload_byte_budget,
            storage_payload_units_remaining: 12,
            reproduction_target_max_permille: 1_200,
            max_forwarding_decisions: 12,
        },
    );
    let summary = summarize_policy_decisions(input, &readiness_summary, &decisions)?;
    Ok((summary, decisions))
}

fn state_trace(
    input: &BaselineRunInput,
    readiness_log: &crate::diffusion::coded_inference::CodedInferenceReadinessLog,
) -> Vec<LocalPolicyStateTraceEvent> {
    let receiver = input.scenario.coded_inference.receiver_node_id;
    let mut trace = Vec::new();
    for contact in &readiness_log.contact_events {
        if contact.node_a == receiver || contact.node_b == receiver {
            let (peer_node_id, peer_cluster_id) = if contact.node_a == receiver {
                (contact.node_b, contact.cluster_b)
            } else {
                (contact.node_a, contact.cluster_a)
            };
            trace.push(LocalPolicyStateTraceEvent::Contact {
                round_index: contact.round_index,
                peer_node_id,
                peer_cluster_id,
                bridge_contact: contact.cluster_a != contact.cluster_b,
            });
        }
    }
    append_receiver_and_pressure_trace(&mut trace, readiness_log);
    trace
}

fn append_receiver_and_pressure_trace(
    trace: &mut Vec<LocalPolicyStateTraceEvent>,
    readiness_log: &crate::diffusion::coded_inference::CodedInferenceReadinessLog,
) {
    for event in &readiness_log.forwarding_events {
        trace.push(LocalPolicyStateTraceEvent::Arrival {
            arrival_kind: LocalPolicyArrivalKind::from(event.classification),
        });
    }
    for event in &readiness_log.budget_events {
        trace.push(LocalPolicyStateTraceEvent::Storage {
            retained_payload_bytes: event.retained_bytes,
            storage_capacity_bytes: event.retained_bytes.saturating_add(512).max(1),
        });
    }
    for event in &readiness_log.controller_events {
        trace.push(LocalPolicyStateTraceEvent::Reproduction {
            active_forwarding_opportunities: event.active_forwarding_opportunities,
            innovative_successor_opportunities: event.innovative_successor_opportunities,
        });
    }
}

fn peer_candidates(state: &super::LocalPolicyState) -> Vec<LocalPolicyPeerCandidate> {
    state
        .peers
        .keys()
        .copied()
        .map(|peer_node_id| LocalPolicyPeerCandidate { peer_node_id })
        .collect()
}

fn fragment_candidates(
    input: &BaselineRunInput,
    readiness_log: &crate::diffusion::coded_inference::CodedInferenceReadinessLog,
) -> Vec<LocalPolicyFragmentCandidate> {
    readiness_log
        .forwarding_events
        .iter()
        .take(16)
        .map(|event| LocalPolicyFragmentCandidate {
            fragment_id: event.fragment_id.unwrap_or(event.evidence_id),
            payload_bytes: input.fixed_budget.fragment_payload_bytes,
            expected_innovation_gain: innovation_gain(event.classification),
            landscape_value: if event.receiver_node_id
                == input.scenario.coded_inference.receiver_node_id
            {
                500
            } else {
                150
            },
            demand_value: demand_value(readiness_log, event.evidence_id),
            duplicate_risk_hint: duplicate_risk(event.classification),
        })
        .collect()
}

fn demand_value(
    readiness_log: &crate::diffusion::coded_inference::CodedInferenceReadinessLog,
    evidence_id: u32,
) -> u32 {
    readiness_log
        .demand_events
        .iter()
        .find(|event| event.satisfied_by_evidence_id == Some(evidence_id))
        .map(|event| event.uncertainty_permille.min(1_000))
        .unwrap_or(0)
}

fn innovation_gain(classification: CodedArrivalClassification) -> u32 {
    match classification {
        CodedArrivalClassification::Innovative => 800,
        CodedArrivalClassification::Duplicate => 100,
    }
}

fn duplicate_risk(classification: CodedArrivalClassification) -> u32 {
    match classification {
        CodedArrivalClassification::Innovative => 0,
        CodedArrivalClassification::Duplicate => 500,
    }
}

fn summarize_policy_decisions(
    input: &BaselineRunInput,
    readiness_summary: &crate::diffusion::coded_inference::CodedInferenceReadinessSummary,
    decisions: &[LocalPolicyDecisionRecord],
) -> Result<BaselineRunSummary, BaselineContractError> {
    let selected = decisions
        .iter()
        .filter(|decision| decision.selected)
        .collect::<Vec<_>>();
    let bytes_transmitted = selected
        .iter()
        .map(|_| input.fixed_budget.fragment_payload_bytes)
        .fold(0_u32, u32::saturating_add);
    let arrivals = arrival_counters(selected.len(), readiness_summary);
    BaselineRunSummary::try_from_draft(BaselineRunSummaryDraft {
        artifact_namespace: input.artifact_namespace.clone(),
        family_id: input.scenario.diffusion.family_id.clone(),
        policy_id: Some(BaselinePolicyId::LocalEvidencePolicy),
        payload_mode: Some(BaselinePayloadMode::CodedFragment),
        fixed_budget_label: Some(input.fixed_budget.label.clone()),
        fixed_payload_budget_bytes: Some(input.fixed_budget.payload_byte_budget),
        recovery_probability_permille: Some(readiness_summary.recovery_probability_permille),
        decision_accuracy_permille: Some(readiness_summary.decision_accuracy_permille),
        reconstruction_round: Some(readiness_summary.reconstruction_round),
        commitment_round: Some(readiness_summary.decision_event_round),
        receiver_rank: Some(readiness_summary.receiver_rank),
        top_hypothesis_margin: Some(readiness_summary.top_hypothesis_margin),
        bytes_transmitted: Some(bytes_transmitted),
        forwarding_events: Some(u32::try_from(selected.len()).unwrap_or(u32::MAX)),
        peak_stored_payload_units_per_node: Some(u32::try_from(selected.len()).unwrap_or(u32::MAX)),
        peak_stored_payload_bytes_per_node: Some(bytes_transmitted),
        duplicate_rate_permille: Some(arrivals.duplicate_rate_permille()),
        innovative_arrival_rate_permille: Some(arrivals.innovative_arrival_rate_permille()),
        duplicate_arrival_count: Some(arrivals.duplicate_arrivals),
        innovative_arrival_count: Some(arrivals.innovative_arrivals),
        target_reproduction_min_permille: Some(readiness_summary.target_reproduction_min_permille),
        target_reproduction_max_permille: Some(readiness_summary.target_reproduction_max_permille),
        measured_reproduction_permille: Some(readiness_summary.effective_reproduction_permille),
    })
}

fn arrival_counters(
    selected_count: usize,
    readiness_summary: &crate::diffusion::coded_inference::CodedInferenceReadinessSummary,
) -> BaselineArrivalCounters {
    let selected_count_u32 = u32::try_from(selected_count).unwrap_or(u32::MAX);
    let innovative = readiness_summary
        .innovative_arrival_count
        .min(selected_count_u32);
    BaselineArrivalCounters {
        innovative_arrivals: innovative,
        duplicate_arrivals: selected_count_u32.saturating_sub(innovative),
    }
}

#[cfg(test)]
mod tests {
    use super::run_local_evidence_policy_baseline;
    use crate::diffusion::{
        baselines::{BaselineFixedBudget, BaselinePolicyId, BaselineRunInput},
        catalog::scenarios::build_coded_inference_readiness_scenario,
    };

    fn input() -> BaselineRunInput {
        let scenario = build_coded_inference_readiness_scenario();
        let budget = BaselineFixedBudget::try_new(
            "equal-payload-bytes",
            4_096,
            scenario.coded_inference.uncoded_message_payload_bytes,
            scenario.coded_inference.fragment_payload_bytes,
        )
        .expect("budget");
        BaselineRunInput::try_new(41, scenario, BaselinePolicyId::LocalEvidencePolicy, budget)
            .expect("input")
    }

    #[test]
    fn local_policy_baseline_runs_under_shared_metric_schema() {
        let (summary, decisions) = run_local_evidence_policy_baseline(&input()).expect("baseline");

        assert_eq!(summary.policy_id, BaselinePolicyId::LocalEvidencePolicy);
        assert_eq!(summary.fixed_budget_label, "equal-payload-bytes");
        assert!(summary.bytes_transmitted <= summary.fixed_payload_budget_bytes);
        assert!(summary.forwarding_events > 0);
        assert!(decisions.iter().any(|decision| decision.selected));
    }

    #[test]
    fn local_policy_baseline_decision_rows_include_every_score_term() {
        let (_summary, decisions) = run_local_evidence_policy_baseline(&input()).expect("baseline");
        let serialized = serde_json::to_string(&decisions[0]).expect("json");

        assert!(serialized.contains("policy_id"));
        assert!(serialized.contains("local-evidence-policy"));
        for field in [
            "expected_innovation_gain",
            "bridge_value",
            "landscape_value",
            "demand_value",
            "duplicate_risk",
            "byte_cost",
            "storage_pressure_cost",
            "reproduction_pressure_penalty",
            "total_score",
        ] {
            assert!(serialized.contains(field));
        }
    }
}
// proc-macro-scope: local-policy baseline rows are artifact schema, not shared model vocabulary.
