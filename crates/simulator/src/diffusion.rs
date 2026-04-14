use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::experiments::ExperimentError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionPolicyConfig {
    pub config_id: String,
    pub replication_budget: u32,
    pub ttl_rounds: u32,
    pub forward_probability_permille: u32,
    pub bridge_bias_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionRegimeDescriptor {
    pub density: String,
    pub mobility_model: String,
    pub transport_mix: String,
    pub pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DiffusionMobilityProfile {
    Static,
    LocalMover,
    Bridger,
    LongRangeMover,
    Observer,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DiffusionTransportKind {
    Ble,
    WifiAware,
    LoRa,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DiffusionMessageMode {
    Unicast,
    Broadcast,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionNodeSpec {
    pub node_id: u32,
    pub cluster_id: u8,
    pub mobility_profile: DiffusionMobilityProfile,
    pub energy_budget: u32,
    pub storage_capacity: u32,
    pub transport_capabilities: Vec<DiffusionTransportKind>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionScenarioSpec {
    pub family_id: String,
    pub regime: DiffusionRegimeDescriptor,
    pub round_count: u32,
    pub creation_round: u32,
    pub payload_bytes: u32,
    pub message_mode: DiffusionMessageMode,
    pub source_node_id: u32,
    pub destination_node_id: Option<u32>,
    pub nodes: Vec<DiffusionNodeSpec>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionRunSpec {
    pub suite_id: String,
    pub family_id: String,
    pub seed: u64,
    pub policy: DiffusionPolicyConfig,
    pub scenario: DiffusionScenarioSpec,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionContactEvent {
    pub round_index: u32,
    pub node_a: u32,
    pub node_b: u32,
    pub duration_rounds: u32,
    pub bandwidth_bytes: u32,
    pub transport_kind: DiffusionTransportKind,
    pub connection_latency_rounds: u32,
    pub energy_cost_per_byte: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionRunSummary {
    pub suite_id: String,
    pub family_id: String,
    pub config_id: String,
    pub seed: u64,
    pub density: String,
    pub mobility_model: String,
    pub transport_mix: String,
    pub pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
    pub replication_budget: u32,
    pub ttl_rounds: u32,
    pub forward_probability_permille: u32,
    pub bridge_bias_permille: u32,
    pub delivery_probability_permille: u32,
    pub delivery_latency_rounds: Option<u32>,
    pub coverage_permille: u32,
    pub total_transmissions: u32,
    pub energy_spent_units: u32,
    pub energy_per_delivered_message: Option<u32>,
    pub storage_utilization_permille: u32,
    pub estimated_reproduction_permille: u32,
    pub corridor_persistence_permille: u32,
    pub decision_churn_count: u32,
    pub observer_leakage_permille: u32,
    pub bounded_state: String,
    pub message_persistence_rounds: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionAggregateSummary {
    pub suite_id: String,
    pub family_id: String,
    pub config_id: String,
    pub density: String,
    pub mobility_model: String,
    pub transport_mix: String,
    pub pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
    pub replication_budget: u32,
    pub ttl_rounds: u32,
    pub forward_probability_permille: u32,
    pub bridge_bias_permille: u32,
    pub run_count: u32,
    pub delivery_probability_permille_mean: u32,
    pub delivery_latency_rounds_mean: Option<u32>,
    pub coverage_permille_mean: u32,
    pub total_transmissions_mean: u32,
    pub energy_spent_units_mean: u32,
    pub energy_per_delivered_message_mean: Option<u32>,
    pub storage_utilization_permille_mean: u32,
    pub estimated_reproduction_permille_mean: u32,
    pub corridor_persistence_permille_mean: u32,
    pub decision_churn_count_mean: u32,
    pub observer_leakage_permille_mean: u32,
    pub message_persistence_rounds_mean: u32,
    pub bounded_state_mode: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionBoundarySummary {
    pub suite_id: String,
    pub config_id: String,
    pub viable_family_count: u32,
    pub first_collapse_family_id: Option<String>,
    pub first_collapse_stress_score: Option<u32>,
    pub first_explosive_family_id: Option<String>,
    pub first_explosive_stress_score: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionManifest {
    pub suite_id: String,
    pub run_count: u32,
    pub aggregate_count: u32,
    pub boundary_count: u32,
}

#[derive(Clone, Debug)]
pub struct DiffusionArtifacts {
    pub output_dir: PathBuf,
    pub manifest: DiffusionManifest,
    pub runs: Vec<DiffusionRunSummary>,
    pub aggregates: Vec<DiffusionAggregateSummary>,
    pub boundaries: Vec<DiffusionBoundarySummary>,
}

#[derive(Clone, Debug)]
pub struct DiffusionSuite {
    suite_id: String,
    runs: Vec<DiffusionRunSpec>,
}

impl DiffusionSuite {
    #[must_use]
    pub fn suite_id(&self) -> &str {
        &self.suite_id
    }
}

#[derive(Clone, Debug)]
struct PendingTransfer {
    arrival_round: u32,
    target_node_id: u32,
}

#[derive(Clone, Debug)]
struct HolderState {
    first_round: u32,
}

#[must_use]
pub fn diffusion_smoke_suite() -> DiffusionSuite {
    build_diffusion_suite("diffusion-smoke", &[41], true)
}

#[must_use]
pub fn diffusion_local_suite() -> DiffusionSuite {
    build_diffusion_suite("diffusion-local", &[41, 43], false)
}

pub fn run_diffusion_suite(
    suite: &DiffusionSuite,
    output_dir: &Path,
) -> Result<DiffusionArtifacts, ExperimentError> {
    fs::create_dir_all(output_dir)?;
    let mut runs = Vec::new();
    let run_path = output_dir.join("diffusion_runs.jsonl");
    let mut writer = BufWriter::new(File::create(&run_path)?);
    for spec in &suite.runs {
        let summary = simulate_diffusion_run(spec);
        serde_json::to_writer(&mut writer, &summary)?;
        writer.write_all(b"\n")?;
        runs.push(summary);
    }
    writer.flush()?;
    let aggregates = aggregate_diffusion_runs(&runs);
    let boundaries = summarize_diffusion_boundaries(&aggregates);
    let manifest = DiffusionManifest {
        suite_id: suite.suite_id.clone(),
        run_count: u32::try_from(runs.len()).unwrap_or(u32::MAX),
        aggregate_count: u32::try_from(aggregates.len()).unwrap_or(u32::MAX),
        boundary_count: u32::try_from(boundaries.len()).unwrap_or(u32::MAX),
    };
    serde_json::to_writer_pretty(
        File::create(output_dir.join("diffusion_manifest.json"))?,
        &manifest,
    )?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("diffusion_aggregates.json"))?,
        &aggregates,
    )?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("diffusion_boundaries.json"))?,
        &boundaries,
    )?;
    Ok(DiffusionArtifacts {
        output_dir: output_dir.to_path_buf(),
        manifest,
        runs,
        aggregates,
        boundaries,
    })
}

fn build_diffusion_suite(suite_id: &str, seeds: &[u64], smoke: bool) -> DiffusionSuite {
    let configs = if smoke {
        vec![
            DiffusionPolicyConfig {
                config_id: "diffusion-r4-t30-p650-b250".to_string(),
                replication_budget: 4,
                ttl_rounds: 30,
                forward_probability_permille: 650,
                bridge_bias_permille: 250,
            },
            DiffusionPolicyConfig {
                config_id: "diffusion-r8-t42-p800-b350".to_string(),
                replication_budget: 8,
                ttl_rounds: 42,
                forward_probability_permille: 800,
                bridge_bias_permille: 350,
            },
        ]
    } else {
        vec![
            DiffusionPolicyConfig {
                config_id: "diffusion-r2-t18-p350-b0".to_string(),
                replication_budget: 2,
                ttl_rounds: 18,
                forward_probability_permille: 350,
                bridge_bias_permille: 0,
            },
            DiffusionPolicyConfig {
                config_id: "diffusion-r4-t18-p350-b0".to_string(),
                replication_budget: 4,
                ttl_rounds: 18,
                forward_probability_permille: 350,
                bridge_bias_permille: 0,
            },
            DiffusionPolicyConfig {
                config_id: "diffusion-r4-t30-p650-b0".to_string(),
                replication_budget: 4,
                ttl_rounds: 30,
                forward_probability_permille: 650,
                bridge_bias_permille: 0,
            },
            DiffusionPolicyConfig {
                config_id: "diffusion-r4-t30-p650-b250".to_string(),
                replication_budget: 4,
                ttl_rounds: 30,
                forward_probability_permille: 650,
                bridge_bias_permille: 250,
            },
            DiffusionPolicyConfig {
                config_id: "diffusion-r8-t30-p650-b250".to_string(),
                replication_budget: 8,
                ttl_rounds: 30,
                forward_probability_permille: 650,
                bridge_bias_permille: 250,
            },
            DiffusionPolicyConfig {
                config_id: "diffusion-r8-t42-p650-b250".to_string(),
                replication_budget: 8,
                ttl_rounds: 42,
                forward_probability_permille: 650,
                bridge_bias_permille: 250,
            },
            DiffusionPolicyConfig {
                config_id: "diffusion-r8-t42-p800-b350".to_string(),
                replication_budget: 8,
                ttl_rounds: 42,
                forward_probability_permille: 800,
                bridge_bias_permille: 350,
            },
        ]
    };
    let scenarios = vec![
        build_random_waypoint_sanity_scenario(),
        build_partitioned_clusters_scenario(),
        build_disaster_broadcast_scenario(),
        build_sparse_long_delay_scenario(),
        build_high_density_overload_scenario(),
        build_mobility_shift_scenario(),
        build_adversarial_observation_scenario(),
    ];
    let mut runs = Vec::new();
    for seed in seeds {
        for scenario in &scenarios {
            for policy in &configs {
                runs.push(DiffusionRunSpec {
                    suite_id: suite_id.to_string(),
                    family_id: scenario.family_id.clone(),
                    seed: *seed,
                    policy: policy.clone(),
                    scenario: scenario.clone(),
                });
            }
        }
    }
    DiffusionSuite {
        suite_id: suite_id.to_string(),
        runs,
    }
}

fn simulate_diffusion_run(spec: &DiffusionRunSpec) -> DiffusionRunSummary {
    let scenario = &spec.scenario;
    let policy = &spec.policy;
    let target_count = scenario_target_count(scenario);
    let mut holders = BTreeMap::new();
    let mut pending = Vec::<PendingTransfer>::new();
    holders.insert(
        scenario.source_node_id,
        HolderState {
            first_round: scenario.creation_round,
        },
    );
    let mut delivered_targets = BTreeSet::new();
    let mut delivery_rounds = Vec::<u32>::new();
    let mut copy_budget_remaining = policy.replication_budget;
    let mut total_transmissions = 0_u32;
    let mut total_energy = 0_u32;
    let mut peak_holders = 1_u32;
    let mut round_new_copies = Vec::<u32>::new();
    let mut edge_flows = BTreeMap::<(u32, u32), u32>::new();
    let mut dominant_edge_by_round = Vec::<Option<(u32, u32)>>::new();
    let mut observer_touches = 0_u32;

    for round in 0..scenario.round_count {
        let mut arrivals = Vec::new();
        pending.retain(|transfer| {
            if transfer.arrival_round <= round {
                arrivals.push(transfer.target_node_id);
                false
            } else {
                true
            }
        });
        let mut new_copies_this_round = 0_u32;
        for node_id in arrivals {
            if holders.contains_key(&node_id) {
                continue;
            }
            holders.insert(node_id, HolderState { first_round: round });
            new_copies_this_round = new_copies_this_round.saturating_add(1);
            if is_target_node(scenario, node_id) {
                delivered_targets.insert(node_id);
                delivery_rounds.push(round.saturating_sub(scenario.creation_round));
            }
        }

        if round < scenario.creation_round || round > scenario.creation_round.saturating_add(policy.ttl_rounds)
        {
            round_new_copies.push(new_copies_this_round);
            dominant_edge_by_round.push(None);
            continue;
        }

        let contacts = generate_contacts(spec.seed, scenario, round);
        let mut round_edge_counts = BTreeMap::<(u32, u32), u32>::new();
        for contact in contacts {
            for (from, to) in [(contact.node_a, contact.node_b), (contact.node_b, contact.node_a)] {
                if !holders.contains_key(&from) || holders.contains_key(&to) {
                    continue;
                }
                let allow_budget = copy_budget_remaining > 0 || is_target_node(scenario, to);
                if !allow_budget {
                    continue;
                }
                let score = forwarding_score(scenario, policy, from, to, &contact);
                if score <= permille_hash(spec.seed, scenario.family_id.as_str(), round, from, to, 0) {
                    continue;
                }
                if contact.bandwidth_bytes < scenario.payload_bytes {
                    continue;
                }
                total_transmissions = total_transmissions.saturating_add(1);
                total_energy = total_energy.saturating_add(
                    scenario.payload_bytes.saturating_mul(contact.energy_cost_per_byte),
                );
                if is_observer_node(scenario, from) || is_observer_node(scenario, to) {
                    observer_touches = observer_touches.saturating_add(1);
                }
                let edge = normalized_edge(from, to);
                *edge_flows.entry(edge).or_insert(0) += 1;
                *round_edge_counts.entry(edge).or_insert(0) += 1;
                let arrival_round = round.saturating_add(contact.connection_latency_rounds);
                if arrival_round <= scenario.creation_round.saturating_add(policy.ttl_rounds) {
                    pending.push(PendingTransfer {
                        arrival_round,
                        target_node_id: to,
                    });
                }
                if !is_target_node(scenario, to) && copy_budget_remaining > 0 {
                    copy_budget_remaining -= 1;
                }
            }
        }
        dominant_edge_by_round.push(
            round_edge_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(edge, _)| edge),
        );
        peak_holders = peak_holders.max(u32::try_from(holders.len()).unwrap_or(u32::MAX));
        round_new_copies.push(new_copies_this_round);
    }

    let delivery_probability_permille = match scenario.message_mode {
        DiffusionMessageMode::Unicast => {
            if delivered_targets.contains(&scenario.destination_node_id.unwrap_or_default()) {
                1000
            } else {
                0
            }
        }
        DiffusionMessageMode::Broadcast => coverage_permille_for(target_count, delivered_targets.len()),
    };
    let coverage_permille = coverage_permille_for(target_count, delivered_targets.len());
    let energy_per_delivered_message = if delivered_targets.is_empty() {
        None
    } else {
        Some(total_energy / u32::try_from(delivered_targets.len()).unwrap_or(1))
    };
    let delivery_latency_rounds = if delivery_rounds.is_empty() {
        None
    } else {
        Some(
            delivery_rounds.iter().copied().sum::<u32>()
                / u32::try_from(delivery_rounds.len()).unwrap_or(1),
        )
    };
    let total_storage_capacity = scenario.nodes.iter().map(|node| node.storage_capacity).sum::<u32>();
    let storage_utilization_permille = if total_storage_capacity == 0 {
        0
    } else {
        peak_holders
            .saturating_mul(scenario.payload_bytes)
            .saturating_mul(1000)
            / total_storage_capacity
    };
    let holders_before_contact = round_new_copies
        .iter()
        .fold((0_u32, 0_u32), |(holders_so_far, sum), new_copies| {
            let updated_sum = sum.saturating_add(holders_so_far);
            (holders_so_far.saturating_add(*new_copies), updated_sum)
        })
        .1
        .saturating_add(1);
    let estimated_reproduction_permille = if holders_before_contact == 0 {
        0
    } else {
        u32::try_from((u64::from(total_transmissions) * 1000) / u64::from(holders_before_contact))
            .unwrap_or(u32::MAX)
    };
    let corridor_persistence_permille = if total_transmissions == 0 {
        0
    } else {
        edge_flows.values().copied().max().unwrap_or(0).saturating_mul(1000) / total_transmissions
    };
    let decision_churn_count = dominant_edge_by_round
        .windows(2)
        .filter(|window| window[0].is_some() && window[1].is_some() && window[0] != window[1])
        .count() as u32;
    let observer_leakage_permille = if total_transmissions == 0 {
        0
    } else {
        observer_touches.saturating_mul(1000) / total_transmissions
    };
    let bounded_state = bounded_state(
        delivery_probability_permille,
        coverage_permille,
        estimated_reproduction_permille,
        total_transmissions,
    )
    .to_string();
    let message_persistence_rounds = if holders.is_empty() {
        0
    } else {
        scenario
            .round_count
            .saturating_sub(
                holders
                    .values()
                    .map(|holder| holder.first_round)
                    .min()
                    .unwrap_or(scenario.round_count),
            )
    };

    DiffusionRunSummary {
        suite_id: spec.suite_id.clone(),
        family_id: spec.family_id.clone(),
        config_id: policy.config_id.clone(),
        seed: spec.seed,
        density: scenario.regime.density.clone(),
        mobility_model: scenario.regime.mobility_model.clone(),
        transport_mix: scenario.regime.transport_mix.clone(),
        pressure: scenario.regime.pressure.clone(),
        objective_regime: scenario.regime.objective_regime.clone(),
        stress_score: scenario.regime.stress_score,
        replication_budget: policy.replication_budget,
        ttl_rounds: policy.ttl_rounds,
        forward_probability_permille: policy.forward_probability_permille,
        bridge_bias_permille: policy.bridge_bias_permille,
        delivery_probability_permille,
        delivery_latency_rounds,
        coverage_permille,
        total_transmissions,
        energy_spent_units: total_energy,
        energy_per_delivered_message,
        storage_utilization_permille,
        estimated_reproduction_permille,
        corridor_persistence_permille,
        decision_churn_count,
        observer_leakage_permille,
        bounded_state,
        message_persistence_rounds,
    }
}

fn aggregate_diffusion_runs(runs: &[DiffusionRunSummary]) -> Vec<DiffusionAggregateSummary> {
    let mut grouped = BTreeMap::<(String, String), Vec<&DiffusionRunSummary>>::new();
    for run in runs {
        grouped
            .entry((run.family_id.clone(), run.config_id.clone()))
            .or_default()
            .push(run);
    }
    let mut aggregates = Vec::new();
    for ((_family_id, _config_id), group) in grouped {
        let first = group[0];
        let run_count = u32::try_from(group.len()).unwrap_or(u32::MAX);
        let mode = mode_string(group.iter().map(|row| row.bounded_state.clone()));
        aggregates.push(DiffusionAggregateSummary {
            suite_id: first.suite_id.clone(),
            family_id: first.family_id.clone(),
            config_id: first.config_id.clone(),
            density: first.density.clone(),
            mobility_model: first.mobility_model.clone(),
            transport_mix: first.transport_mix.clone(),
            pressure: first.pressure.clone(),
            objective_regime: first.objective_regime.clone(),
            stress_score: first.stress_score,
            replication_budget: first.replication_budget,
            ttl_rounds: first.ttl_rounds,
            forward_probability_permille: first.forward_probability_permille,
            bridge_bias_permille: first.bridge_bias_permille,
            run_count,
            delivery_probability_permille_mean: mean_u32(group.iter().map(|row| row.delivery_probability_permille)),
            delivery_latency_rounds_mean: mean_option_u32(group.iter().map(|row| row.delivery_latency_rounds)),
            coverage_permille_mean: mean_u32(group.iter().map(|row| row.coverage_permille)),
            total_transmissions_mean: mean_u32(group.iter().map(|row| row.total_transmissions)),
            energy_spent_units_mean: mean_u32(group.iter().map(|row| row.energy_spent_units)),
            energy_per_delivered_message_mean: mean_option_u32(
                group.iter().map(|row| row.energy_per_delivered_message),
            ),
            storage_utilization_permille_mean: mean_u32(group.iter().map(|row| row.storage_utilization_permille)),
            estimated_reproduction_permille_mean: mean_u32(
                group.iter().map(|row| row.estimated_reproduction_permille),
            ),
            corridor_persistence_permille_mean: mean_u32(
                group.iter().map(|row| row.corridor_persistence_permille),
            ),
            decision_churn_count_mean: mean_u32(group.iter().map(|row| row.decision_churn_count)),
            observer_leakage_permille_mean: mean_u32(group.iter().map(|row| row.observer_leakage_permille)),
            message_persistence_rounds_mean: mean_u32(group.iter().map(|row| row.message_persistence_rounds)),
            bounded_state_mode: mode,
        });
    }
    aggregates.sort_by(|left, right| {
        left.family_id
            .cmp(&right.family_id)
            .then(left.delivery_probability_permille_mean.cmp(&right.delivery_probability_permille_mean).reverse())
    });
    aggregates
}

fn summarize_diffusion_boundaries(
    aggregates: &[DiffusionAggregateSummary],
) -> Vec<DiffusionBoundarySummary> {
    let mut grouped = BTreeMap::<String, Vec<&DiffusionAggregateSummary>>::new();
    for aggregate in aggregates {
        grouped.entry(aggregate.config_id.clone()).or_default().push(aggregate);
    }
    let mut rows = Vec::new();
    for (config_id, mut group) in grouped {
        group.sort_by_key(|row| row.stress_score);
        let viable_family_count = u32::try_from(
            group
                .iter()
                .filter(|row| row.bounded_state_mode == "viable")
                .count(),
        )
        .unwrap_or(u32::MAX);
        let collapse = group.iter().find(|row| row.bounded_state_mode == "collapse");
        let explosive = group.iter().find(|row| row.bounded_state_mode == "explosive");
        rows.push(DiffusionBoundarySummary {
            suite_id: group[0].suite_id.clone(),
            config_id,
            viable_family_count,
            first_collapse_family_id: collapse.map(|row| row.family_id.clone()),
            first_collapse_stress_score: collapse.map(|row| row.stress_score),
            first_explosive_family_id: explosive.map(|row| row.family_id.clone()),
            first_explosive_stress_score: explosive.map(|row| row.stress_score),
        });
    }
    rows.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    rows
}

fn generate_contacts(seed: u64, scenario: &DiffusionScenarioSpec, round: u32) -> Vec<DiffusionContactEvent> {
    let mut contacts = Vec::new();
    for index in 0..scenario.nodes.len() {
        for peer_index in index + 1..scenario.nodes.len() {
            let left = &scenario.nodes[index];
            let right = &scenario.nodes[peer_index];
            let probability = contact_probability_permille(scenario, left, right, round);
            if probability <= permille_hash(seed, scenario.family_id.as_str(), round, left.node_id, right.node_id, 1) {
                continue;
            }
            let transport_kind = choose_transport(left, right, round);
            let (bandwidth_bytes, energy_cost_per_byte, connection_latency_rounds) =
                transport_properties(transport_kind);
            contacts.push(DiffusionContactEvent {
                round_index: round,
                node_a: left.node_id,
                node_b: right.node_id,
                duration_rounds: 1,
                bandwidth_bytes,
                transport_kind,
                connection_latency_rounds,
                energy_cost_per_byte,
            });
        }
    }
    contacts
}

fn contact_probability_permille(
    scenario: &DiffusionScenarioSpec,
    left: &DiffusionNodeSpec,
    right: &DiffusionNodeSpec,
    round: u32,
) -> u32 {
    let same_cluster = left.cluster_id == right.cluster_id;
    let bridged = matches!(left.mobility_profile, DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover)
        || matches!(right.mobility_profile, DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover);
    match scenario.family_id.as_str() {
        "diffusion-partitioned-clusters" => {
            if same_cluster { 720 } else if bridged { 260 } else { 28 }
        }
        "diffusion-random-waypoint-sanity" => {
            if same_cluster { 440 } else if bridged { 180 } else { 120 }
        }
        "diffusion-disaster-broadcast" => {
            if same_cluster { 660 } else if bridged { 210 } else { 55 }
        }
        "diffusion-sparse-long-delay" => {
            if same_cluster { 140 } else if bridged { 120 } else { 18 }
        }
        "diffusion-high-density-overload" => {
            if same_cluster { 900 } else if bridged { 620 } else { 420 }
        }
        "diffusion-mobility-shift" => {
            if round < scenario.round_count / 2 {
                if same_cluster { 650 } else if bridged { 140 } else { 30 }
            } else if same_cluster {
                380
            } else if bridged {
                460
            } else {
                140
            }
        }
        "diffusion-adversarial-observation" => {
            if same_cluster { 540 } else if bridged { 180 } else { 42 }
        }
        _ => 0,
    }
}

fn choose_transport(
    left: &DiffusionNodeSpec,
    right: &DiffusionNodeSpec,
    round: u32,
) -> DiffusionTransportKind {
    let same_cluster = left.cluster_id == right.cluster_id;
    if same_cluster {
        if matches!(left.mobility_profile, DiffusionMobilityProfile::Observer)
            || matches!(right.mobility_profile, DiffusionMobilityProfile::Observer)
        {
            DiffusionTransportKind::Ble
        } else {
            DiffusionTransportKind::WifiAware
        }
    } else if matches!(left.mobility_profile, DiffusionMobilityProfile::LongRangeMover)
        || matches!(right.mobility_profile, DiffusionMobilityProfile::LongRangeMover)
        || round % 5 == 0
    {
        DiffusionTransportKind::LoRa
    } else {
        DiffusionTransportKind::WifiAware
    }
}

fn transport_properties(kind: DiffusionTransportKind) -> (u32, u32, u32) {
    match kind {
        DiffusionTransportKind::Ble => (192, 4, 0),
        DiffusionTransportKind::WifiAware => (640, 2, 0),
        DiffusionTransportKind::LoRa => (96, 8, 1),
    }
}

fn forwarding_score(
    scenario: &DiffusionScenarioSpec,
    policy: &DiffusionPolicyConfig,
    from: u32,
    to: u32,
    contact: &DiffusionContactEvent,
) -> u32 {
    let mut score = policy.forward_probability_permille;
    if is_target_node(scenario, to) {
        score = score.saturating_add(220);
    }
    if let Some(node) = scenario.nodes.iter().find(|node| node.node_id == to) {
        if matches!(node.mobility_profile, DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover) {
            score = score.saturating_add(policy.bridge_bias_permille);
        }
        if matches!(node.mobility_profile, DiffusionMobilityProfile::Observer) {
            score = score.saturating_sub(120);
        }
    }
    if matches!(scenario.message_mode, DiffusionMessageMode::Broadcast) {
        score = score.saturating_add(60);
    }
    if matches!(contact.transport_kind, DiffusionTransportKind::LoRa) {
        score = score.saturating_sub(80);
    }
    if matches!(scenario.family_id.as_str(), "diffusion-high-density-overload") && from % 2 == 0 {
        score = score.saturating_sub(90);
    }
    score.min(1000)
}

fn bounded_state(
    delivery_probability_permille: u32,
    coverage_permille: u32,
    reproduction_permille: u32,
    transmissions: u32,
) -> &'static str {
    if delivery_probability_permille < 300 || coverage_permille < 350 {
        "collapse"
    } else if reproduction_permille > 1600 || transmissions > 48 {
        "explosive"
    } else {
        "viable"
    }
}

fn coverage_permille_for(target_count: usize, delivered_count: usize) -> u32 {
    if target_count == 0 {
        return 0;
    }
    u32::try_from((u64::try_from(delivered_count).unwrap_or(0) * 1000) / u64::try_from(target_count).unwrap_or(1))
        .unwrap_or(u32::MAX)
}

fn scenario_target_count(scenario: &DiffusionScenarioSpec) -> usize {
    match scenario.message_mode {
        DiffusionMessageMode::Unicast => 1,
        DiffusionMessageMode::Broadcast => scenario.nodes.len().saturating_sub(1),
    }
}

fn is_target_node(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    match scenario.message_mode {
        DiffusionMessageMode::Unicast => scenario.destination_node_id == Some(node_id),
        DiffusionMessageMode::Broadcast => node_id != scenario.source_node_id,
    }
}

fn is_observer_node(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    scenario
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .map(|node| matches!(node.mobility_profile, DiffusionMobilityProfile::Observer))
        .unwrap_or(false)
}

fn normalized_edge(left: u32, right: u32) -> (u32, u32) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn permille_hash(
    seed: u64,
    family_id: &str,
    round: u32,
    left: u32,
    right: u32,
    lane: u64,
) -> u32 {
    let mut value = seed
        ^ u64::from(round).wrapping_mul(0x9E37_79B9)
        ^ u64::from(left).wrapping_mul(0x85EB_CA6B)
        ^ u64::from(right).wrapping_mul(0xC2B2_AE35)
        ^ lane;
    for byte in family_id.as_bytes() {
        value ^= u64::from(*byte);
        value = value.rotate_left(13).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    u32::try_from(value % 1000).unwrap_or(0)
}

fn mean_u32(values: impl Iterator<Item = u32>) -> u32 {
    let collected = values.collect::<Vec<_>>();
    if collected.is_empty() {
        return 0;
    }
    let sum = collected.iter().copied().map(u64::from).sum::<u64>();
    u32::try_from(sum / u64::try_from(collected.len()).unwrap_or(1)).unwrap_or(u32::MAX)
}

fn mean_option_u32(values: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    let collected = values.flatten().collect::<Vec<_>>();
    if collected.is_empty() {
        return None;
    }
    Some(mean_u32(collected.into_iter()))
}

fn mode_string(values: impl Iterator<Item = String>) -> String {
    let mut counts = BTreeMap::<String, u32>::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(&right.0)))
        .map(|(value, _)| value)
        .unwrap_or_else(|| "none".to_string())
}

fn build_partitioned_clusters_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-partitioned-clusters".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "community-bridgers".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "rare-bridges".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 42,
        },
        round_count: 40,
        creation_round: 2,
        payload_bytes: 64,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(12),
        nodes: clustered_nodes(12, 3, false),
    }
}

fn build_random_waypoint_sanity_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-random-waypoint-sanity".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "mobile-mixed".to_string(),
            mobility_model: "random-waypoint".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "sanity-check".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 20,
        },
        round_count: 28,
        creation_round: 2,
        payload_bytes: 48,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(8),
        nodes: clustered_nodes(8, 2, false),
    }
}

fn build_disaster_broadcast_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes(16, 4, false);
    nodes[0].mobility_profile = DiffusionMobilityProfile::Static;
    DiffusionScenarioSpec {
        family_id: "diffusion-disaster-broadcast".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "community-bridgers".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "urgent-broadcast".to_string(),
            objective_regime: "broadcast".to_string(),
            stress_score: 48,
        },
        round_count: 36,
        creation_round: 1,
        payload_bytes: 72,
        message_mode: DiffusionMessageMode::Broadcast,
        source_node_id: 1,
        destination_node_id: None,
        nodes,
    }
}

fn build_sparse_long_delay_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes(10, 2, false);
    if let Some(node) = nodes.get_mut(8) {
        node.mobility_profile = DiffusionMobilityProfile::LongRangeMover;
    }
    if let Some(node) = nodes.get_mut(9) {
        node.mobility_profile = DiffusionMobilityProfile::LongRangeMover;
    }
    DiffusionScenarioSpec {
        family_id: "diffusion-sparse-long-delay".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "sparse".to_string(),
            mobility_model: "long-delay".to_string(),
            transport_mix: "ble-lora".to_string(),
            pressure: "permanent-partition-risk".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 56,
        },
        round_count: 48,
        creation_round: 2,
        payload_bytes: 56,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(10),
        nodes,
    }
}

fn build_high_density_overload_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-high-density-overload".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "dense-camp".to_string(),
            mobility_model: "clustered".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "overload".to_string(),
            objective_regime: "broadcast".to_string(),
            stress_score: 62,
        },
        round_count: 28,
        creation_round: 1,
        payload_bytes: 64,
        message_mode: DiffusionMessageMode::Broadcast,
        source_node_id: 1,
        destination_node_id: None,
        nodes: clustered_nodes(18, 3, false),
    }
}

fn build_mobility_shift_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-mobility-shift".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "shifted-communities".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "reconfiguration".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 50,
        },
        round_count: 44,
        creation_round: 3,
        payload_bytes: 64,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(12),
        nodes: clustered_nodes(12, 3, false),
    }
}

fn build_adversarial_observation_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-adversarial-observation".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "community-bridgers".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "observer-risk".to_string(),
            objective_regime: "privacy-sensitive-unicast".to_string(),
            stress_score: 46,
        },
        round_count: 38,
        creation_round: 2,
        payload_bytes: 60,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(12),
        nodes: clustered_nodes(12, 3, true),
    }
}

fn clustered_nodes(node_count: u32, cluster_count: u8, with_observers: bool) -> Vec<DiffusionNodeSpec> {
    let mut nodes = Vec::new();
    for node_id in 1..=node_count {
        let cluster_id = u8::try_from((node_id - 1) % u32::from(cluster_count)).unwrap_or(0);
        let mobility_profile = if with_observers && node_id % 6 == 0 {
            DiffusionMobilityProfile::Observer
        } else if node_id % 5 == 0 {
            DiffusionMobilityProfile::Bridger
        } else if node_id % 7 == 0 {
            DiffusionMobilityProfile::LongRangeMover
        } else if node_id % 2 == 0 {
            DiffusionMobilityProfile::LocalMover
        } else {
            DiffusionMobilityProfile::Static
        };
        let transport_capabilities = match mobility_profile {
            DiffusionMobilityProfile::LongRangeMover => {
                vec![DiffusionTransportKind::Ble, DiffusionTransportKind::WifiAware, DiffusionTransportKind::LoRa]
            }
            _ => vec![DiffusionTransportKind::Ble, DiffusionTransportKind::WifiAware],
        };
        nodes.push(DiffusionNodeSpec {
            node_id,
            cluster_id,
            mobility_profile,
            energy_budget: 20_000,
            storage_capacity: 512,
            transport_capabilities,
        });
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_diffusion_runs, build_partitioned_clusters_scenario, bounded_state,
        diffusion_smoke_suite, generate_contacts, simulate_diffusion_run, DiffusionPolicyConfig,
        DiffusionRunSpec,
    };

    #[test]
    fn contact_generation_is_deterministic() {
        let scenario = build_partitioned_clusters_scenario();
        let first = generate_contacts(41, &scenario, 5);
        let second = generate_contacts(41, &scenario, 5);
        assert_eq!(first, second);
    }

    #[test]
    fn message_persists_across_disconnected_intervals() {
        let scenario = build_partitioned_clusters_scenario();
        let policy = DiffusionPolicyConfig {
            config_id: "test".to_string(),
            replication_budget: 4,
            ttl_rounds: 30,
            forward_probability_permille: 650,
            bridge_bias_permille: 250,
        };
        let summary = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy,
            scenario,
        });
        assert!(summary.message_persistence_rounds >= 30);
    }

    #[test]
    fn bounded_replication_stays_out_of_explosive_state_for_smoke_baseline() {
        let suite = diffusion_smoke_suite();
        let summary = simulate_diffusion_run(&suite.runs[0]);
        assert_ne!(summary.bounded_state, "explosive");
    }

    #[test]
    fn aggregate_metrics_are_non_empty() {
        let suite = diffusion_smoke_suite();
        let runs = suite
            .runs
            .iter()
            .take(2)
            .map(simulate_diffusion_run)
            .collect::<Vec<_>>();
        let aggregates = aggregate_diffusion_runs(&runs);
        assert!(!aggregates.is_empty());
    }

    #[test]
    fn bounded_state_classifies_regions() {
        assert_eq!(bounded_state(100, 200, 400, 8), "collapse");
        assert_eq!(bounded_state(800, 850, 1000, 20), "viable");
        assert_eq!(bounded_state(900, 900, 1900, 64), "explosive");
    }
}
