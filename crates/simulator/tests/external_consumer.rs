use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use jacquard_core::SimulationSeed;
use jacquard_simulator::{
    presets, ArtifactSink, CustomDiffusionRunSpec, CustomDiffusionScenarioSpec,
    DiffusionForwardingStyle, DiffusionMessageMode, DiffusionMobilityProfile, DiffusionNodeSpec,
    DiffusionPolicyConfig, DiffusionRegimeDescriptor, DiffusionSuite, DiffusionTransportKind,
    ExperimentRunner, ExperimentSuiteSpec, RouteVisibleRunSpec,
};

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(1);

fn temp_output_dir(label: &str) -> PathBuf {
    let suffix = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "jacquard-external-{label}-{}-{suffix}",
        std::process::id()
    ))
}

fn remove_temp_output_dir(output_dir: &Path) {
    // allow-ignored-result: cleanup must not hide the external-consumer assertion failure.
    let _ = fs::remove_dir_all(output_dir);
}

#[test]
fn external_route_visible_consumer_writes_artifacts() {
    let (scenario, environment) = presets::batman_line();
    let suite = ExperimentSuiteSpec::route_visible(
        "external-route-smoke",
        vec![RouteVisibleRunSpec::new(
            "external-route-smoke-batman",
            "external-connected-line",
            "batman-bellman",
            SimulationSeed(11),
            scenario,
            environment,
        )],
    );
    let output_dir = temp_output_dir("route");
    let artifacts = ExperimentRunner::default()
        .run_route_visible_suite(&suite, &ArtifactSink::directory(&output_dir))
        .expect("external route-visible suite should run");

    assert_eq!(artifacts.manifest.run_count, 1);
    assert_eq!(artifacts.runs.len(), 1);
    assert!(artifacts.runs[0].round_count > 0);
    assert!(artifacts.runs[0]
        .distinct_engine_ids
        .iter()
        .any(|engine| engine == "batman-bellman"));
    assert!(output_dir.join("external_manifest.json").exists());
    assert!(output_dir.join("external_runs.jsonl").exists());

    remove_temp_output_dir(&output_dir);
}

#[test]
fn external_diffusion_consumer_writes_standard_artifacts() {
    let suite = DiffusionSuite::from_custom_runs(
        "external-diffusion-smoke",
        vec![CustomDiffusionRunSpec {
            family_id: "external-diffusion-family".to_string(),
            seed: 41,
            policy: test_diffusion_policy(),
            scenario: test_diffusion_scenario(),
        }],
    )
    .expect("custom diffusion suite should validate");
    let output_dir = temp_output_dir("diffusion");
    let artifacts = ExperimentRunner::default()
        .run_diffusion_suite(&suite, &ArtifactSink::directory(&output_dir))
        .expect("external diffusion suite should run");

    assert_eq!(artifacts.manifest.schema_version, 1);
    assert_eq!(artifacts.manifest.run_count, 1);
    assert_eq!(artifacts.runs.len(), 1);
    assert!(artifacts.runs[0].coverage_permille > 0);
    assert!(output_dir.join("diffusion_manifest.json").exists());
    assert!(output_dir.join("diffusion_runs.jsonl").exists());

    remove_temp_output_dir(&output_dir);
}

#[test]
fn external_route_visible_consumer_can_run_without_report_or_files() {
    let (scenario, environment) = presets::babel_line();
    let suite = ExperimentSuiteSpec::route_visible(
        "external-no-report",
        vec![RouteVisibleRunSpec::new(
            "external-no-report-babel",
            "external-connected-line",
            "babel",
            SimulationSeed(51),
            scenario,
            environment,
        )],
    );
    let artifacts = ExperimentRunner::default()
        .run_route_visible_suite(&suite, &ArtifactSink::disabled())
        .expect("external route-visible suite should run without an artifact directory");

    assert!(artifacts.output_dir.is_none());
    assert_eq!(artifacts.runs.len(), 1);
    assert!(artifacts.runs[0].round_count > 0);
}

fn test_diffusion_policy() -> DiffusionPolicyConfig {
    DiffusionPolicyConfig {
        config_id: "external-balanced".to_string(),
        replication_budget: 4,
        message_horizon: 8,
        forward_probability_permille: 900,
        bridge_bias_permille: 100,
        target_cluster_bias_permille: 100,
        same_cluster_bias_permille: 0,
        observer_aversion_permille: 0,
        lora_bias_permille: 0,
        spread_restraint_permille: 0,
        energy_guard_permille: 0,
        forwarding_style: DiffusionForwardingStyle::BalancedDistanceVector,
    }
}

fn test_diffusion_scenario() -> CustomDiffusionScenarioSpec {
    CustomDiffusionScenarioSpec {
        family_id: "external-diffusion-family".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "tiny".to_string(),
            mobility_model: "custom-static".to_string(),
            transport_mix: "ble".to_string(),
            pressure: "low".to_string(),
            objective_regime: "single-target".to_string(),
            stress_score: 1,
        },
        round_count: 12,
        creation_round: 0,
        payload_bytes: 128,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(3),
        nodes: vec![
            diffusion_node(1, 0),
            diffusion_node(2, 0),
            diffusion_node(3, 0),
        ],
    }
}

fn diffusion_node(node_id: u32, cluster_id: u8) -> DiffusionNodeSpec {
    DiffusionNodeSpec {
        node_id,
        cluster_id,
        mobility_profile: DiffusionMobilityProfile::Static,
        energy_budget: 16_384,
        storage_capacity: 16_384,
        transport_capabilities: vec![DiffusionTransportKind::Ble],
    }
}
