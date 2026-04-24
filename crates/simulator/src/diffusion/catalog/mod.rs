//! Pre-configured diffusion scenario families and policy parameter variations.

use super::model::{
    DiffusionForwardingStyle, DiffusionPolicyConfig, DiffusionRunSpec, DiffusionScenarioSpec,
    DiffusionSuite,
};

pub(super) use super::model::{
    CodedContributionValidityRule, CodedEvidenceOriginMode, CodedEvidenceTransformKind,
    CodedInferenceReadinessScenario, CodedInferenceSpec, CodedLocalObservationSpec,
    CodedRecodingRuleSpec, DiffusionMessageMode, DiffusionMobilityProfile, DiffusionNodeSpec,
    DiffusionRegimeDescriptor, DiffusionTransportKind,
};

pub(super) mod scenarios;

use scenarios::{
    build_adversarial_observation_scenario, build_bridge_drought_scenario,
    build_congestion_cascade_scenario, build_disaster_broadcast_scenario,
    build_energy_starved_relay_scenario, build_high_density_overload_scenario,
    build_large_congestion_threshold_high_scenario,
    build_large_congestion_threshold_moderate_scenario, build_large_regional_shift_high_scenario,
    build_large_regional_shift_moderate_scenario, build_large_sparse_threshold_high_scenario,
    build_large_sparse_threshold_moderate_scenario, build_mobility_shift_scenario,
    build_partitioned_clusters_scenario, build_random_waypoint_sanity_scenario,
    build_sparse_long_delay_scenario,
};

type FieldProfileOverrides = (u32, u32, u32, u32, i32, i32, i32, i32, u32, u32);
type DiffusionPolicySpec = (
    u32,
    u32,
    u32,
    u32,
    i32,
    i32,
    i32,
    i32,
    u32,
    u32,
    DiffusionForwardingStyle,
);

#[must_use]
pub fn diffusion_smoke_suite() -> DiffusionSuite {
    build_diffusion_suite("diffusion-smoke", &[41], true)
}

#[must_use]
pub fn diffusion_local_suite() -> DiffusionSuite {
    build_diffusion_suite("diffusion-local", &[41, 43, 47, 53], false)
}

#[must_use]
pub fn diffusion_local_stage_suite(stage_id: &str) -> Option<DiffusionSuite> {
    let family_ids: &[&str] = match stage_id {
        "diffusion-local-stage-1" => &[
            "diffusion-random-waypoint-sanity",
            "diffusion-partitioned-clusters",
            "diffusion-disaster-broadcast",
            "diffusion-sparse-long-delay",
        ],
        "diffusion-local-stage-2" => &[
            "diffusion-high-density-overload",
            "diffusion-mobility-shift",
            "diffusion-adversarial-observation",
            "diffusion-bridge-drought",
        ],
        "diffusion-local-stage-3" => &[
            "diffusion-energy-starved-relay",
            "diffusion-congestion-cascade",
            "diffusion-large-sparse-threshold-moderate",
            "diffusion-large-congestion-threshold-moderate",
        ],
        "diffusion-local-stage-4" => &[
            "diffusion-large-regional-shift-moderate",
            "diffusion-large-sparse-threshold-high",
            "diffusion-large-congestion-threshold-high",
            "diffusion-large-regional-shift-high",
        ],
        _ => return None,
    };
    Some(build_diffusion_suite_filtered(
        "diffusion-local",
        &[41, 43, 47, 53],
        false,
        family_ids,
    ))
}

fn build_diffusion_suite(suite_id: &str, seeds: &[u64], smoke: bool) -> DiffusionSuite {
    build_diffusion_suite_filtered(suite_id, seeds, smoke, &[])
}

fn build_diffusion_suite_filtered(
    suite_id: &str,
    seeds: &[u64],
    smoke: bool,
    family_ids: &[&str],
) -> DiffusionSuite {
    let mut configs = vec![
        diffusion_engine_profile("batman-bellman"),
        diffusion_engine_profile("batman-classic"),
        diffusion_engine_profile("babel"),
        diffusion_engine_profile("olsrv2"),
        diffusion_engine_profile("scatter"),
        diffusion_engine_profile("mercator"),
        diffusion_engine_profile("pathway"),
        diffusion_engine_profile("pathway-batman-bellman"),
    ];
    configs.extend(transition_diffusion_profiles(smoke));
    configs.extend(field_diffusion_profiles());
    let scenarios = diffusion_scenarios(smoke)
        .into_iter()
        .filter(|scenario| {
            family_ids.is_empty() || family_ids.contains(&scenario.family_id.as_str())
        })
        .collect::<Vec<_>>();
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

fn diffusion_scenarios(smoke: bool) -> Vec<DiffusionScenarioSpec> {
    let mut scenarios = vec![
        build_random_waypoint_sanity_scenario(),
        build_partitioned_clusters_scenario(),
        build_disaster_broadcast_scenario(),
        build_sparse_long_delay_scenario(),
        build_high_density_overload_scenario(),
        build_mobility_shift_scenario(),
        build_adversarial_observation_scenario(),
        build_bridge_drought_scenario(),
        build_energy_starved_relay_scenario(),
        build_congestion_cascade_scenario(),
        build_large_sparse_threshold_moderate_scenario(),
        build_large_congestion_threshold_moderate_scenario(),
        build_large_regional_shift_moderate_scenario(),
    ];
    if !smoke {
        scenarios.extend([
            build_large_sparse_threshold_high_scenario(),
            build_large_congestion_threshold_high_scenario(),
            build_large_regional_shift_high_scenario(),
        ]);
    }
    scenarios
}

// long-block-exception: profile roster is intentionally listed in one place so
// the maintained transition presets remain auditable as a single table.
fn transition_diffusion_profiles(smoke: bool) -> Vec<DiffusionPolicyConfig> {
    let mut profiles = vec![
        diffusion_policy_profile(
            "transition-tight",
            (
                1,
                14,
                140,
                40,
                40,
                180,
                180,
                -220,
                520,
                420,
                DiffusionForwardingStyle::ConservativeLocal,
            ),
        ),
        diffusion_policy_profile(
            "transition-balanced",
            (
                3,
                24,
                420,
                160,
                110,
                20,
                130,
                -40,
                160,
                140,
                DiffusionForwardingStyle::BalancedDistanceVector,
            ),
        ),
    ];
    if !smoke {
        profiles.extend([
            diffusion_policy_profile(
                "transition-broad",
                (
                    10,
                    44,
                    900,
                    260,
                    180,
                    -80,
                    60,
                    120,
                    10,
                    10,
                    DiffusionForwardingStyle::ServiceDirected,
                ),
            ),
            diffusion_policy_profile(
                "transition-bridge-biased",
                (
                    4,
                    30,
                    520,
                    360,
                    180,
                    -20,
                    140,
                    40,
                    120,
                    110,
                    DiffusionForwardingStyle::ContinuityBiased,
                ),
            ),
        ]);
    }
    profiles
}

fn field_diffusion_profiles() -> Vec<DiffusionPolicyConfig> {
    let mut variants = vec![("field".to_string(), "field".to_string(), None)];
    let search_templates: [(&str, &str, [FieldProfileOverrides; 4]); 4] = [
        (
            "field-continuity",
            "field-continuity",
            [
                (4, 34, 455, 360, 190, -10, 180, 140, 140, 120),
                (3, 28, 420, 340, 180, 30, 210, 80, 190, 150),
                (5, 38, 470, 390, 210, -30, 170, 160, 110, 110),
                (4, 32, 440, 350, 210, 0, 190, 120, 150, 130),
            ],
        ),
        (
            "field-scarcity",
            "field-scarcity",
            [
                (2, 20, 320, 210, 190, 95, 220, -90, 320, 260),
                (2, 18, 290, 230, 210, 110, 240, -120, 360, 300),
                (1, 16, 260, 220, 220, 130, 250, -150, 420, 340),
                (2, 18, 300, 250, 230, 120, 220, -140, 390, 320),
            ],
        ),
        (
            "field-congestion",
            "field-congestion",
            [
                (2, 18, 300, 150, 120, 140, 210, -120, 360, 240),
                (8, 26, 520, 180, 150, 20, 200, 0, 140, 120),
                (6, 22, 380, 170, 170, 120, 210, -80, 280, 190),
                (7, 24, 440, 210, 210, 50, 190, -20, 200, 130),
            ],
        ),
        (
            "field-privacy",
            "field-privacy",
            [
                (2, 22, 320, 210, 160, 90, 360, -40, 260, 220),
                (2, 22, 310, 200, 160, 100, 360, -40, 260, 220),
                (2, 24, 330, 240, 190, 70, 420, -80, 280, 240),
                (3, 24, 360, 250, 200, 40, 440, -120, 260, 210),
            ],
        ),
    ];
    for (base_id, prefix, overrides) in search_templates {
        append_field_profile_variants(&mut variants, base_id, prefix, &overrides);
    }
    variants
        .into_iter()
        .map(field_profile_from_variant)
        .collect()
}

fn append_field_profile_variants(
    variants: &mut Vec<(String, String, Option<FieldProfileOverrides>)>,
    base_id: &str,
    prefix: &str,
    overrides: &[FieldProfileOverrides],
) {
    variants.push((base_id.to_string(), prefix.to_string(), None));
    for (index, override_set) in overrides.iter().copied().enumerate() {
        variants.push((
            base_id.to_string(),
            format!("{prefix}-search-{}", index + 1),
            Some(override_set),
        ));
    }
}

fn field_profile_from_variant(
    (base_id, config_id, overrides): (String, String, Option<FieldProfileOverrides>),
) -> DiffusionPolicyConfig {
    let mut profile = diffusion_engine_profile(&base_id);
    profile.config_id = config_id;
    if let Some(overrides) = overrides {
        apply_field_profile_overrides(&mut profile, overrides);
    }
    profile
}

fn apply_field_profile_overrides(
    profile: &mut DiffusionPolicyConfig,
    (
        replication_budget,
        message_horizon,
        forward_probability_permille,
        bridge_bias_permille,
        target_cluster_bias_permille,
        same_cluster_bias_permille,
        observer_aversion_permille,
        lora_bias_permille,
        spread_restraint_permille,
        energy_guard_permille,
    ): FieldProfileOverrides,
) {
    profile.replication_budget = replication_budget;
    profile.message_horizon = message_horizon;
    profile.forward_probability_permille = forward_probability_permille;
    profile.bridge_bias_permille = bridge_bias_permille;
    profile.target_cluster_bias_permille = target_cluster_bias_permille;
    profile.same_cluster_bias_permille = same_cluster_bias_permille;
    profile.observer_aversion_permille = observer_aversion_permille;
    profile.lora_bias_permille = lora_bias_permille;
    profile.spread_restraint_permille = spread_restraint_permille;
    profile.energy_guard_permille = energy_guard_permille;
}

fn diffusion_policy_profile(
    config_id: &str,
    (
        replication_budget,
        message_horizon,
        forward_probability_permille,
        bridge_bias_permille,
        target_cluster_bias_permille,
        same_cluster_bias_permille,
        observer_aversion_permille,
        lora_bias_permille,
        spread_restraint_permille,
        energy_guard_permille,
        forwarding_style,
    ): DiffusionPolicySpec,
) -> DiffusionPolicyConfig {
    DiffusionPolicyConfig {
        config_id: config_id.to_string(),
        replication_budget,
        message_horizon,
        forward_probability_permille,
        bridge_bias_permille,
        target_cluster_bias_permille,
        same_cluster_bias_permille,
        observer_aversion_permille,
        lora_bias_permille,
        spread_restraint_permille,
        energy_guard_permille,
        forwarding_style,
    }
}

// long-block-exception: the diffusion engine profile catalog is maintained as a
// single tuning surface so per-engine defaults remain auditable in one place.
fn diffusion_engine_profile(engine_set: &str) -> DiffusionPolicyConfig {
    match engine_set {
        "batman-bellman" => diffusion_policy_profile(
            "batman-bellman",
            (
                3,
                20,
                380,
                80,
                90,
                45,
                130,
                -80,
                180,
                140,
                DiffusionForwardingStyle::BalancedDistanceVector,
            ),
        ),
        "batman-classic" => diffusion_policy_profile(
            "batman-classic",
            (
                2,
                24,
                320,
                60,
                80,
                90,
                150,
                -120,
                240,
                190,
                DiffusionForwardingStyle::ConservativeLocal,
            ),
        ),
        "babel" => diffusion_policy_profile(
            "babel",
            (
                3,
                22,
                430,
                90,
                105,
                25,
                120,
                -40,
                140,
                120,
                DiffusionForwardingStyle::FreshnessAware,
            ),
        ),
        "olsrv2" => diffusion_policy_profile(
            "olsrv2",
            (
                3,
                24,
                400,
                110,
                120,
                20,
                130,
                0,
                150,
                130,
                DiffusionForwardingStyle::FreshnessAware,
            ),
        ),
        "pathway" => diffusion_policy_profile(
            "pathway",
            (
                5,
                20,
                540,
                180,
                170,
                -50,
                90,
                40,
                90,
                80,
                DiffusionForwardingStyle::ServiceDirected,
            ),
        ),
        "scatter" => diffusion_policy_profile(
            "scatter",
            (
                4,
                28,
                470,
                260,
                150,
                -20,
                180,
                80,
                170,
                140,
                DiffusionForwardingStyle::ConservativeLocal,
            ),
        ),
        "mercator" => diffusion_policy_profile(
            "mercator",
            (
                10,
                40,
                620,
                420,
                180,
                -90,
                180,
                80,
                110,
                130,
                DiffusionForwardingStyle::ContinuityBiased,
            ),
        ),
        "field" => diffusion_policy_profile(
            "field",
            (
                3,
                26,
                430,
                240,
                150,
                35,
                190,
                40,
                180,
                150,
                DiffusionForwardingStyle::ContinuityBiased,
            ),
        ),
        "field-continuity" => diffusion_policy_profile(
            "field-continuity",
            (
                4,
                34,
                460,
                360,
                190,
                -10,
                180,
                140,
                140,
                120,
                DiffusionForwardingStyle::ContinuityBiased,
            ),
        ),
        "field-scarcity" => diffusion_policy_profile(
            "field-scarcity",
            (
                2,
                20,
                330,
                220,
                200,
                100,
                220,
                -90,
                320,
                260,
                DiffusionForwardingStyle::ContinuityBiased,
            ),
        ),
        "field-congestion" => diffusion_policy_profile(
            "field-congestion",
            (
                2,
                18,
                300,
                160,
                130,
                140,
                200,
                -120,
                360,
                240,
                DiffusionForwardingStyle::ContinuityBiased,
            ),
        ),
        "field-privacy" => diffusion_policy_profile(
            "field-privacy",
            (
                2,
                22,
                320,
                210,
                160,
                90,
                360,
                -40,
                260,
                220,
                DiffusionForwardingStyle::ContinuityBiased,
            ),
        ),
        "pathway-batman-bellman" => diffusion_policy_profile(
            "pathway-batman-bellman",
            (
                6,
                24,
                560,
                180,
                150,
                10,
                100,
                20,
                70,
                60,
                DiffusionForwardingStyle::Composite,
            ),
        ),
        _ => diffusion_policy_profile(
            engine_set,
            (
                4,
                24,
                450,
                120,
                100,
                0,
                100,
                0,
                120,
                120,
                DiffusionForwardingStyle::BalancedDistanceVector,
            ),
        ),
    }
}
