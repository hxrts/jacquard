//! Deterministic local-policy scenario matrix fixtures.

use serde::{Deserialize, Serialize};

use super::{
    local_policy_state_from_trace, run_local_policy_ablation, LocalPolicyAblationDecisionRecord,
    LocalPolicyAblationVariant, LocalPolicyArrivalKind, LocalPolicyFragmentCandidate,
    LocalPolicyPeerCandidate, LocalPolicyReducerBudget, LocalPolicyStateTraceEvent,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum LocalPolicyScenarioKind {
    SparsePressure,
    ClusteredDuplicateHeavy,
    BridgeHeavy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LocalPolicyScenarioArtifact {
    pub scenario: LocalPolicyScenarioKind,
    pub seed: u64,
    pub rows: Vec<LocalPolicyAblationDecisionRecord>,
    pub selected_forwarding_count: u32,
    pub selected_payload_bytes: u32,
    pub selected_quality_permille: u32,
    pub selected_score_sum: i32,
}

pub(crate) fn run_local_policy_scenario_matrix(seed: u64) -> Vec<LocalPolicyScenarioArtifact> {
    [
        LocalPolicyScenarioKind::SparsePressure,
        LocalPolicyScenarioKind::ClusteredDuplicateHeavy,
        LocalPolicyScenarioKind::BridgeHeavy,
    ]
    .into_iter()
    .flat_map(|scenario| run_scenario_variants(seed, scenario))
    .collect()
}

fn run_scenario_variants(
    seed: u64,
    scenario: LocalPolicyScenarioKind,
) -> Vec<LocalPolicyScenarioArtifact> {
    let fixture = scenario_fixture(scenario);
    [
        LocalPolicyAblationVariant::FullPolicy,
        LocalPolicyAblationVariant::NoBridgeScore,
        LocalPolicyAblationVariant::NoDuplicateRisk,
        LocalPolicyAblationVariant::NoLandscapeValue,
        LocalPolicyAblationVariant::NoDemandValue,
        LocalPolicyAblationVariant::NoReproductionControl,
        LocalPolicyAblationVariant::DeterministicRandomForwarding,
    ]
    .into_iter()
    .map(|variant| {
        let rows = run_local_policy_ablation(
            variant,
            seed,
            &fixture.state,
            &fixture.peers,
            &fixture.fragments,
            fixture.budget,
        );
        summarize_scenario(scenario, seed, rows, &fixture.fragments)
    })
    .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScenarioFixture {
    state: super::LocalPolicyState,
    peers: Vec<LocalPolicyPeerCandidate>,
    fragments: Vec<LocalPolicyFragmentCandidate>,
    budget: LocalPolicyReducerBudget,
}

fn scenario_fixture(scenario: LocalPolicyScenarioKind) -> ScenarioFixture {
    match scenario {
        LocalPolicyScenarioKind::SparsePressure => sparse_pressure_fixture(),
        LocalPolicyScenarioKind::ClusteredDuplicateHeavy => clustered_duplicate_fixture(),
        LocalPolicyScenarioKind::BridgeHeavy => bridge_heavy_fixture(),
    }
}

fn sparse_pressure_fixture() -> ScenarioFixture {
    ScenarioFixture {
        state: state_from_trace(&[
            LocalPolicyStateTraceEvent::Contact {
                round_index: 0,
                peer_node_id: 3,
                peer_cluster_id: 0,
                bridge_contact: false,
            },
            LocalPolicyStateTraceEvent::Reproduction {
                active_forwarding_opportunities: 1,
                innovative_successor_opportunities: 1,
            },
        ]),
        peers: vec![LocalPolicyPeerCandidate { peer_node_id: 3 }],
        fragments: vec![fragment(1, 32, 700, 100, 250, 0)],
        budget: budget(64, 1, 800, 1),
    }
}

fn clustered_duplicate_fixture() -> ScenarioFixture {
    ScenarioFixture {
        state: state_from_trace(&[
            LocalPolicyStateTraceEvent::Contact {
                round_index: 0,
                peer_node_id: 4,
                peer_cluster_id: 1,
                bridge_contact: false,
            },
            LocalPolicyStateTraceEvent::Arrival {
                arrival_kind: LocalPolicyArrivalKind::Duplicate,
            },
            LocalPolicyStateTraceEvent::Arrival {
                arrival_kind: LocalPolicyArrivalKind::Duplicate,
            },
            LocalPolicyStateTraceEvent::Reproduction {
                active_forwarding_opportunities: 4,
                innovative_successor_opportunities: 1,
            },
        ]),
        peers: vec![LocalPolicyPeerCandidate { peer_node_id: 4 }],
        fragments: vec![fragment(2, 32, 500, 250, 100, 300)],
        budget: budget(64, 1, 1_000, 1),
    }
}

fn bridge_heavy_fixture() -> ScenarioFixture {
    ScenarioFixture {
        state: state_from_trace(&[
            LocalPolicyStateTraceEvent::Contact {
                round_index: 0,
                peer_node_id: 9,
                peer_cluster_id: 2,
                bridge_contact: true,
            },
            LocalPolicyStateTraceEvent::Contact {
                round_index: 1,
                peer_node_id: 10,
                peer_cluster_id: 3,
                bridge_contact: true,
            },
            LocalPolicyStateTraceEvent::Reproduction {
                active_forwarding_opportunities: 4,
                innovative_successor_opportunities: 2,
            },
        ]),
        peers: vec![
            LocalPolicyPeerCandidate { peer_node_id: 9 },
            LocalPolicyPeerCandidate { peer_node_id: 10 },
        ],
        fragments: vec![fragment(3, 32, 650, 450, 350, 0)],
        budget: budget(64, 1, 1_000, 1),
    }
}

fn state_from_trace(trace: &[LocalPolicyStateTraceEvent]) -> super::LocalPolicyState {
    local_policy_state_from_trace(7, 512, trace).expect("scenario state")
}

fn fragment(
    fragment_id: u32,
    payload_bytes: u32,
    expected_innovation_gain: u32,
    landscape_value: u32,
    demand_value: u32,
    duplicate_risk_hint: u32,
) -> LocalPolicyFragmentCandidate {
    LocalPolicyFragmentCandidate {
        fragment_id,
        payload_bytes,
        expected_innovation_gain,
        landscape_value,
        demand_value,
        duplicate_risk_hint,
    }
}

fn budget(
    payload_byte_budget_remaining: u32,
    storage_payload_units_remaining: u32,
    reproduction_target_max_permille: u32,
    max_forwarding_decisions: u32,
) -> LocalPolicyReducerBudget {
    LocalPolicyReducerBudget {
        payload_byte_budget_remaining,
        storage_payload_units_remaining,
        reproduction_target_max_permille,
        max_forwarding_decisions,
    }
}

fn summarize_scenario(
    scenario: LocalPolicyScenarioKind,
    seed: u64,
    rows: Vec<LocalPolicyAblationDecisionRecord>,
    fragments: &[LocalPolicyFragmentCandidate],
) -> LocalPolicyScenarioArtifact {
    let selected = rows
        .iter()
        .filter(|row| row.decision.selected)
        .collect::<Vec<_>>();
    LocalPolicyScenarioArtifact {
        scenario,
        seed,
        selected_forwarding_count: u32::try_from(selected.len()).unwrap_or(u32::MAX),
        selected_payload_bytes: selected
            .iter()
            .map(|row| payload_bytes(row.decision.fragment_id, fragments))
            .fold(0_u32, u32::saturating_add),
        selected_quality_permille: selected_quality(&selected),
        selected_score_sum: selected
            .iter()
            .map(|row| row.decision.total_score)
            .fold(0_i32, i32::saturating_add),
        rows,
    }
}

fn payload_bytes(fragment_id: u32, fragments: &[LocalPolicyFragmentCandidate]) -> u32 {
    fragments
        .iter()
        .find(|fragment| fragment.fragment_id == fragment_id)
        .map(|fragment| fragment.payload_bytes)
        .unwrap_or(0)
}

fn selected_quality(selected: &[&LocalPolicyAblationDecisionRecord]) -> u32 {
    if selected.is_empty() {
        return 0;
    }
    let total = selected
        .iter()
        .map(|row| {
            row.decision
                .score
                .expected_innovation_gain
                .max(0)
                .cast_unsigned()
        })
        .fold(0_u32, u32::saturating_add);
    total
        .saturating_div(u32::try_from(selected.len()).unwrap_or(u32::MAX))
        .min(1_000)
}

#[cfg(test)]
mod tests {
    use super::{run_local_policy_scenario_matrix, LocalPolicyScenarioKind};
    use crate::diffusion::local_policy::LocalPolicyAblationVariant;

    fn artifact(
        matrix: &[super::LocalPolicyScenarioArtifact],
        scenario: LocalPolicyScenarioKind,
        variant: LocalPolicyAblationVariant,
    ) -> &super::LocalPolicyScenarioArtifact {
        matrix
            .iter()
            .find(|artifact| artifact.scenario == scenario && artifact.rows[0].variant == variant)
            .expect("artifact")
    }

    #[test]
    fn local_policy_scenarios_run_all_variants_on_all_fixtures() {
        let matrix = run_local_policy_scenario_matrix(41);

        assert_eq!(matrix.len(), 21);
        for scenario in [
            LocalPolicyScenarioKind::SparsePressure,
            LocalPolicyScenarioKind::ClusteredDuplicateHeavy,
            LocalPolicyScenarioKind::BridgeHeavy,
        ] {
            assert_eq!(
                matrix
                    .iter()
                    .filter(|artifact| artifact.scenario == scenario)
                    .count(),
                7
            );
        }
    }

    #[test]
    fn local_policy_scenarios_full_policy_beats_random_at_equal_cost() {
        let matrix = run_local_policy_scenario_matrix(41);
        let full = artifact(
            &matrix,
            LocalPolicyScenarioKind::BridgeHeavy,
            LocalPolicyAblationVariant::FullPolicy,
        );
        let random = artifact(
            &matrix,
            LocalPolicyScenarioKind::BridgeHeavy,
            LocalPolicyAblationVariant::DeterministicRandomForwarding,
        );

        assert_eq!(full.selected_payload_bytes, random.selected_payload_bytes);
        assert!(full.selected_quality_permille >= random.selected_quality_permille);
    }

    #[test]
    fn local_policy_scenarios_bridge_score_changes_bridge_heavy_behavior() {
        let matrix = run_local_policy_scenario_matrix(41);
        let full = artifact(
            &matrix,
            LocalPolicyScenarioKind::BridgeHeavy,
            LocalPolicyAblationVariant::FullPolicy,
        );
        let no_bridge = artifact(
            &matrix,
            LocalPolicyScenarioKind::BridgeHeavy,
            LocalPolicyAblationVariant::NoBridgeScore,
        );

        assert!(full.selected_score_sum > no_bridge.selected_score_sum);
    }

    #[test]
    fn local_policy_scenarios_duplicate_risk_changes_clustered_behavior() {
        let matrix = run_local_policy_scenario_matrix(41);
        let full = artifact(
            &matrix,
            LocalPolicyScenarioKind::ClusteredDuplicateHeavy,
            LocalPolicyAblationVariant::FullPolicy,
        );
        let no_duplicate = artifact(
            &matrix,
            LocalPolicyScenarioKind::ClusteredDuplicateHeavy,
            LocalPolicyAblationVariant::NoDuplicateRisk,
        );

        assert!(no_duplicate.selected_score_sum > full.selected_score_sum);
    }

    #[test]
    fn local_policy_scenarios_landscape_value_changes_evidence_behavior() {
        let matrix = run_local_policy_scenario_matrix(41);
        let full = artifact(
            &matrix,
            LocalPolicyScenarioKind::BridgeHeavy,
            LocalPolicyAblationVariant::FullPolicy,
        );
        let no_landscape = artifact(
            &matrix,
            LocalPolicyScenarioKind::BridgeHeavy,
            LocalPolicyAblationVariant::NoLandscapeValue,
        );

        assert!(full.selected_score_sum > no_landscape.selected_score_sum);
    }

    #[test]
    fn local_policy_scenarios_demand_value_changes_active_behavior() {
        let matrix = run_local_policy_scenario_matrix(41);
        let full = artifact(
            &matrix,
            LocalPolicyScenarioKind::BridgeHeavy,
            LocalPolicyAblationVariant::FullPolicy,
        );
        let no_demand = artifact(
            &matrix,
            LocalPolicyScenarioKind::BridgeHeavy,
            LocalPolicyAblationVariant::NoDemandValue,
        );

        assert!(full.selected_score_sum > no_demand.selected_score_sum);
    }

    #[test]
    fn local_policy_scenarios_reproduction_control_changes_pressure_behavior() {
        let matrix = run_local_policy_scenario_matrix(41);
        let full = artifact(
            &matrix,
            LocalPolicyScenarioKind::SparsePressure,
            LocalPolicyAblationVariant::FullPolicy,
        );
        let no_reproduction = artifact(
            &matrix,
            LocalPolicyScenarioKind::SparsePressure,
            LocalPolicyAblationVariant::NoReproductionControl,
        );

        assert_eq!(full.selected_forwarding_count, 0);
        assert!(no_reproduction.selected_forwarding_count > full.selected_forwarding_count);
    }

    #[test]
    fn local_policy_scenarios_artifacts_are_replay_stable() {
        let first = run_local_policy_scenario_matrix(41);
        let second = run_local_policy_scenario_matrix(41);

        assert_eq!(first, second);
    }
}
// proc-macro-scope: local-policy scenario rows are artifact schema, not shared model vocabulary.
