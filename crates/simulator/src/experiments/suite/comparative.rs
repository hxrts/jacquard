use super::*;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum ComparativeSuiteScale {
    Smoke,
    Full,
}

type ComparativeFamilySpec = (&'static str, RegimeFields<'static>, FamilyBuilder);

const SCATTER_FAMILY_SPECS: [ComparativeFamilySpec; 7] = [
    (
        "scatter-connected-low-loss",
        (
            "medium-ring",
            "low",
            "low",
            "none",
            "static",
            "none",
            "connected-only",
            18,
        ),
        build_comparison_connected_low_loss,
    ),
    (
        "scatter-connected-high-loss",
        (
            "bridge-cluster",
            "high",
            "medium",
            "mild",
            "relink-and-replace",
            "mixed",
            "repairable-connected",
            54,
        ),
        build_comparison_connected_high_loss,
    ),
    (
        "scatter-bridge-transition",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "partial-recovery",
            "none",
            "repairable-connected",
            42,
        ),
        build_comparison_bridge_transition,
    ),
    (
        "scatter-partial-observability-bridge",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            46,
        ),
        build_comparison_partial_observability_bridge,
    ),
    (
        "scatter-concurrent-mixed",
        (
            "medium-mesh",
            "moderate",
            "medium",
            "none",
            "partial-recovery",
            "tight-connection",
            "concurrent-mixed",
            48,
        ),
        build_comparison_concurrent_mixed,
    ),
    (
        "scatter-corridor-continuity-uncertainty",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "intermittent-recovery",
            "none",
            "repairable-connected",
            50,
        ),
        build_comparison_corridor_continuity_uncertainty,
    ),
    (
        "scatter-medium-bridge-repair",
        (
            "medium-bridge-chain",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            58,
        ),
        build_comparison_medium_bridge_repair,
    ),
];

const COMPARISON_FAMILY_SPECS: [ComparativeFamilySpec; 7] = [
    (
        "comparison-connected-low-loss",
        (
            "medium-ring",
            "low",
            "low",
            "none",
            "static",
            "none",
            "connected-only",
            18,
        ),
        build_comparison_connected_low_loss,
    ),
    (
        "comparison-connected-high-loss",
        (
            "bridge-cluster",
            "high",
            "medium",
            "mild",
            "relink-and-replace",
            "mixed",
            "repairable-connected",
            54,
        ),
        build_comparison_connected_high_loss,
    ),
    (
        "comparison-bridge-transition",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "partial-recovery",
            "none",
            "repairable-connected",
            42,
        ),
        build_comparison_bridge_transition,
    ),
    (
        "comparison-partial-observability-bridge",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            46,
        ),
        build_comparison_partial_observability_bridge,
    ),
    (
        "comparison-concurrent-mixed",
        (
            "medium-mesh",
            "moderate",
            "medium",
            "none",
            "partial-recovery",
            "tight-connection",
            "concurrent-mixed",
            48,
        ),
        build_comparison_concurrent_mixed,
    ),
    (
        "comparison-corridor-continuity-uncertainty",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "intermittent-recovery",
            "none",
            "repairable-connected",
            50,
        ),
        build_comparison_corridor_continuity_uncertainty,
    ),
    (
        "comparison-medium-bridge-repair",
        (
            "medium-bridge-chain",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            58,
        ),
        build_comparison_medium_bridge_repair,
    ),
];

const HEAD_TO_HEAD_FAMILY_SPECS: [ComparativeFamilySpec; 7] = [
    (
        "head-to-head-connected-low-loss",
        (
            "medium-ring",
            "low",
            "low",
            "none",
            "static",
            "none",
            "connected-only",
            18,
        ),
        build_comparison_connected_low_loss,
    ),
    (
        "head-to-head-connected-high-loss",
        (
            "bridge-cluster",
            "high",
            "medium",
            "mild",
            "relink-and-replace",
            "mixed",
            "repairable-connected",
            54,
        ),
        build_comparison_connected_high_loss,
    ),
    (
        "head-to-head-bridge-transition",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "partial-recovery",
            "none",
            "repairable-connected",
            42,
        ),
        build_comparison_bridge_transition,
    ),
    (
        "head-to-head-partial-observability-bridge",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            46,
        ),
        build_comparison_partial_observability_bridge,
    ),
    (
        "head-to-head-concurrent-mixed",
        (
            "medium-mesh",
            "moderate",
            "medium",
            "none",
            "partial-recovery",
            "tight-connection",
            "concurrent-mixed",
            48,
        ),
        build_comparison_concurrent_mixed,
    ),
    (
        "head-to-head-corridor-continuity-uncertainty",
        (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "intermittent-recovery",
            "none",
            "repairable-connected",
            50,
        ),
        build_comparison_corridor_continuity_uncertainty,
    ),
    (
        "head-to-head-medium-bridge-repair",
        (
            "medium-bridge-chain",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            58,
        ),
        build_comparison_medium_bridge_repair,
    ),
];

fn family_descriptors(
    specs: &[ComparativeFamilySpec],
) -> Vec<(&'static str, RegimeDescriptor, FamilyBuilder)> {
    specs
        .iter()
        .map(|(family_id, fields, builder)| (*family_id, regime(*fields), *builder))
        .collect()
}

fn scatter_parameter_sets(scale: ComparativeSuiteScale) -> Vec<ExperimentParameterSet> {
    match scale {
        ComparativeSuiteScale::Smoke => vec![
            ExperimentParameterSet::scatter("balanced"),
            ExperimentParameterSet::scatter("degraded-network"),
        ],
        ComparativeSuiteScale::Full => vec![
            ExperimentParameterSet::scatter("balanced"),
            ExperimentParameterSet::scatter("conservative"),
            ExperimentParameterSet::scatter("degraded-network"),
        ],
    }
}

fn comparison_configs(scale: ComparativeSuiteScale) -> Vec<ExperimentParameterSet> {
    match scale {
        ComparativeSuiteScale::Smoke => vec![ExperimentParameterSet::comparison(
            4,
            2,
            3,
            PathwaySearchHeuristicMode::Zero,
        )],
        ComparativeSuiteScale::Full => vec![
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero),
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound),
        ],
    }
}

fn head_to_head_configs() -> Vec<ExperimentParameterSet> {
    vec![
        ExperimentParameterSet::head_to_head("batman-bellman", Some((1, 1)), None, None),
        ExperimentParameterSet::head_to_head("batman-classic", Some((4, 2)), None, None),
        ExperimentParameterSet::head_to_head("babel", Some((4, 2)), None, None),
        ExperimentParameterSet::head_to_head("olsrv2", Some((4, 2)), None, None),
        ExperimentParameterSet::head_to_head("scatter", None, None, None),
        ExperimentParameterSet::head_to_head(
            "pathway",
            None,
            Some((6, PathwaySearchHeuristicMode::HopLowerBound)),
            None,
        ),
        ExperimentParameterSet::head_to_head_field_low_churn(),
        ExperimentParameterSet::head_to_head(
            "pathway-batman-bellman",
            Some((6, 3)),
            Some((6, PathwaySearchHeuristicMode::HopLowerBound)),
            None,
        ),
    ]
}

pub(super) fn build_scatter_runs(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
) -> Vec<ExperimentRunSpec> {
    let parameter_sets = scatter_parameter_sets(scale);
    let families = family_descriptors(&SCATTER_FAMILY_SPECS);
    expand_runs(suite_id, "scatter", seeds, &parameter_sets, &families)
}

pub(super) fn build_comparison_runs(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
) -> Vec<ExperimentRunSpec> {
    let configs = comparison_configs(scale);
    let families = family_descriptors(&COMPARISON_FAMILY_SPECS);
    expand_runs(suite_id, "comparison", seeds, &configs, &families)
}

pub(super) fn build_head_to_head_runs(
    suite_id: &str,
    seeds: &[u64],
    _scale: ComparativeSuiteScale,
) -> Vec<ExperimentRunSpec> {
    let configs = head_to_head_configs();
    let families = family_descriptors(&HEAD_TO_HEAD_FAMILY_SPECS);
    expand_runs(suite_id, "head-to-head", seeds, &configs, &families)
}
