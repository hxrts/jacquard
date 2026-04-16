use super::{materialize_families, FamilyBuilder, FamilyDescriptor};
use crate::experiments::{
    build_comparison_bridge_transition, build_comparison_concurrent_mixed,
    build_comparison_connected_high_loss, build_comparison_connected_low_loss,
    build_comparison_corridor_continuity_uncertainty, build_comparison_large_core_periphery_high,
    build_comparison_large_core_periphery_moderate, build_comparison_large_multi_bottleneck_high,
    build_comparison_large_multi_bottleneck_moderate, build_comparison_medium_bridge_repair,
    build_comparison_partial_observability_bridge, RegimeDescriptor,
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::experiments) enum ComparativeSuiteScale {
    Smoke,
    Full,
}

const SCATTER_FAMILIES: [FamilyDescriptor; 11] = [
    FamilyDescriptor {
        family_id: "scatter-connected-low-loss",
        regime: (
            "medium-ring",
            "low",
            "low",
            "none",
            "static",
            "none",
            "connected-only",
            18,
        ),
        builder: build_comparison_connected_low_loss,
    },
    FamilyDescriptor {
        family_id: "scatter-connected-high-loss",
        regime: (
            "bridge-cluster",
            "high",
            "medium",
            "mild",
            "relink-and-replace",
            "mixed",
            "repairable-connected",
            54,
        ),
        builder: build_comparison_connected_high_loss,
    },
    FamilyDescriptor {
        family_id: "scatter-bridge-transition",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "partial-recovery",
            "none",
            "repairable-connected",
            42,
        ),
        builder: build_comparison_bridge_transition,
    },
    FamilyDescriptor {
        family_id: "scatter-partial-observability-bridge",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            46,
        ),
        builder: build_comparison_partial_observability_bridge,
    },
    FamilyDescriptor {
        family_id: "scatter-concurrent-mixed",
        regime: (
            "medium-mesh",
            "moderate",
            "medium",
            "none",
            "partial-recovery",
            "tight-connection",
            "concurrent-mixed",
            48,
        ),
        builder: build_comparison_concurrent_mixed,
    },
    FamilyDescriptor {
        family_id: "scatter-corridor-continuity-uncertainty",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "intermittent-recovery",
            "none",
            "repairable-connected",
            50,
        ),
        builder: build_comparison_corridor_continuity_uncertainty,
    },
    FamilyDescriptor {
        family_id: "scatter-medium-bridge-repair",
        regime: (
            "medium-bridge-chain",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            58,
        ),
        builder: build_comparison_medium_bridge_repair,
    },
    FamilyDescriptor {
        family_id: "scatter-large-core-periphery-moderate",
        regime: (
            "large-core-periphery-moderate",
            "moderate",
            "high",
            "mild",
            "reroute-shift",
            "moderate",
            "repairable-connected",
            66,
        ),
        builder: build_comparison_large_core_periphery_moderate,
    },
    FamilyDescriptor {
        family_id: "scatter-large-core-periphery-high",
        regime: (
            "large-core-periphery-high",
            "moderate",
            "high",
            "moderate",
            "reroute-shift",
            "high",
            "repairable-connected",
            76,
        ),
        builder: build_comparison_large_core_periphery_high,
    },
    FamilyDescriptor {
        family_id: "scatter-large-multi-bottleneck-moderate",
        regime: (
            "large-multi-bottleneck-moderate",
            "high",
            "high",
            "moderate",
            "staggered-bottlenecks",
            "high",
            "repairable-connected",
            82,
        ),
        builder: build_comparison_large_multi_bottleneck_moderate,
    },
    FamilyDescriptor {
        family_id: "scatter-large-multi-bottleneck-high",
        regime: (
            "large-multi-bottleneck-high",
            "high",
            "high",
            "severe",
            "staggered-bottlenecks",
            "high",
            "repairable-connected",
            90,
        ),
        builder: build_comparison_large_multi_bottleneck_high,
    },
];

const COMPARISON_FAMILIES: [FamilyDescriptor; 11] = [
    FamilyDescriptor {
        family_id: "comparison-connected-low-loss",
        regime: (
            "medium-ring",
            "low",
            "low",
            "none",
            "static",
            "none",
            "connected-only",
            18,
        ),
        builder: build_comparison_connected_low_loss,
    },
    FamilyDescriptor {
        family_id: "comparison-connected-high-loss",
        regime: (
            "bridge-cluster",
            "high",
            "medium",
            "mild",
            "relink-and-replace",
            "mixed",
            "repairable-connected",
            54,
        ),
        builder: build_comparison_connected_high_loss,
    },
    FamilyDescriptor {
        family_id: "comparison-bridge-transition",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "partial-recovery",
            "none",
            "repairable-connected",
            42,
        ),
        builder: build_comparison_bridge_transition,
    },
    FamilyDescriptor {
        family_id: "comparison-partial-observability-bridge",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            46,
        ),
        builder: build_comparison_partial_observability_bridge,
    },
    FamilyDescriptor {
        family_id: "comparison-concurrent-mixed",
        regime: (
            "medium-mesh",
            "moderate",
            "medium",
            "none",
            "partial-recovery",
            "tight-connection",
            "concurrent-mixed",
            48,
        ),
        builder: build_comparison_concurrent_mixed,
    },
    FamilyDescriptor {
        family_id: "comparison-corridor-continuity-uncertainty",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "intermittent-recovery",
            "none",
            "repairable-connected",
            50,
        ),
        builder: build_comparison_corridor_continuity_uncertainty,
    },
    FamilyDescriptor {
        family_id: "comparison-medium-bridge-repair",
        regime: (
            "medium-bridge-chain",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            58,
        ),
        builder: build_comparison_medium_bridge_repair,
    },
    FamilyDescriptor {
        family_id: "comparison-large-core-periphery-moderate",
        regime: (
            "large-core-periphery-moderate",
            "moderate",
            "high",
            "mild",
            "reroute-shift",
            "moderate",
            "repairable-connected",
            66,
        ),
        builder: build_comparison_large_core_periphery_moderate,
    },
    FamilyDescriptor {
        family_id: "comparison-large-core-periphery-high",
        regime: (
            "large-core-periphery-high",
            "moderate",
            "high",
            "moderate",
            "reroute-shift",
            "high",
            "repairable-connected",
            76,
        ),
        builder: build_comparison_large_core_periphery_high,
    },
    FamilyDescriptor {
        family_id: "comparison-large-multi-bottleneck-moderate",
        regime: (
            "large-multi-bottleneck-moderate",
            "high",
            "high",
            "moderate",
            "staggered-bottlenecks",
            "high",
            "repairable-connected",
            82,
        ),
        builder: build_comparison_large_multi_bottleneck_moderate,
    },
    FamilyDescriptor {
        family_id: "comparison-large-multi-bottleneck-high",
        regime: (
            "large-multi-bottleneck-high",
            "high",
            "high",
            "severe",
            "staggered-bottlenecks",
            "high",
            "repairable-connected",
            90,
        ),
        builder: build_comparison_large_multi_bottleneck_high,
    },
];

const HEAD_TO_HEAD_FAMILIES: [FamilyDescriptor; 11] = [
    FamilyDescriptor {
        family_id: "head-to-head-connected-low-loss",
        regime: (
            "medium-ring",
            "low",
            "low",
            "none",
            "static",
            "none",
            "connected-only",
            18,
        ),
        builder: build_comparison_connected_low_loss,
    },
    FamilyDescriptor {
        family_id: "head-to-head-connected-high-loss",
        regime: (
            "bridge-cluster",
            "high",
            "medium",
            "mild",
            "relink-and-replace",
            "mixed",
            "repairable-connected",
            54,
        ),
        builder: build_comparison_connected_high_loss,
    },
    FamilyDescriptor {
        family_id: "head-to-head-bridge-transition",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "partial-recovery",
            "none",
            "repairable-connected",
            42,
        ),
        builder: build_comparison_bridge_transition,
    },
    FamilyDescriptor {
        family_id: "head-to-head-partial-observability-bridge",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            46,
        ),
        builder: build_comparison_partial_observability_bridge,
    },
    FamilyDescriptor {
        family_id: "head-to-head-concurrent-mixed",
        regime: (
            "medium-mesh",
            "moderate",
            "medium",
            "none",
            "partial-recovery",
            "tight-connection",
            "concurrent-mixed",
            48,
        ),
        builder: build_comparison_concurrent_mixed,
    },
    FamilyDescriptor {
        family_id: "head-to-head-corridor-continuity-uncertainty",
        regime: (
            "bridge-cluster",
            "moderate",
            "medium",
            "moderate",
            "intermittent-recovery",
            "none",
            "repairable-connected",
            50,
        ),
        builder: build_comparison_corridor_continuity_uncertainty,
    },
    FamilyDescriptor {
        family_id: "head-to-head-medium-bridge-repair",
        regime: (
            "medium-bridge-chain",
            "moderate",
            "medium",
            "mild",
            "partial-recovery",
            "none",
            "repairable-connected",
            58,
        ),
        builder: build_comparison_medium_bridge_repair,
    },
    FamilyDescriptor {
        family_id: "head-to-head-large-core-periphery-moderate",
        regime: (
            "large-core-periphery-moderate",
            "moderate",
            "high",
            "mild",
            "reroute-shift",
            "moderate",
            "repairable-connected",
            66,
        ),
        builder: build_comparison_large_core_periphery_moderate,
    },
    FamilyDescriptor {
        family_id: "head-to-head-large-core-periphery-high",
        regime: (
            "large-core-periphery-high",
            "moderate",
            "high",
            "moderate",
            "reroute-shift",
            "high",
            "repairable-connected",
            76,
        ),
        builder: build_comparison_large_core_periphery_high,
    },
    FamilyDescriptor {
        family_id: "head-to-head-large-multi-bottleneck-moderate",
        regime: (
            "large-multi-bottleneck-moderate",
            "high",
            "high",
            "moderate",
            "staggered-bottlenecks",
            "high",
            "repairable-connected",
            82,
        ),
        builder: build_comparison_large_multi_bottleneck_moderate,
    },
    FamilyDescriptor {
        family_id: "head-to-head-large-multi-bottleneck-high",
        regime: (
            "large-multi-bottleneck-high",
            "high",
            "high",
            "severe",
            "staggered-bottlenecks",
            "high",
            "repairable-connected",
            90,
        ),
        builder: build_comparison_large_multi_bottleneck_high,
    },
];

fn scaled_families(
    descriptors: &[FamilyDescriptor],
    scale: ComparativeSuiteScale,
) -> Vec<(&'static str, RegimeDescriptor, FamilyBuilder)> {
    let filtered = descriptors
        .iter()
        .copied()
        .filter(|descriptor| {
            scale == ComparativeSuiteScale::Full || !descriptor.family_id.ends_with("-high")
        })
        .collect::<Vec<_>>();
    materialize_families(&filtered)
}

pub(in crate::experiments) fn scatter_family_descriptors(
    scale: ComparativeSuiteScale,
) -> Vec<(&'static str, RegimeDescriptor, FamilyBuilder)> {
    scaled_families(&SCATTER_FAMILIES, scale)
}

pub(in crate::experiments) fn comparison_family_descriptors(
    scale: ComparativeSuiteScale,
) -> Vec<(&'static str, RegimeDescriptor, FamilyBuilder)> {
    scaled_families(&COMPARISON_FAMILIES, scale)
}

pub(in crate::experiments) fn head_to_head_family_descriptors(
    scale: ComparativeSuiteScale,
) -> Vec<(&'static str, RegimeDescriptor, FamilyBuilder)> {
    scaled_families(&HEAD_TO_HEAD_FAMILIES, scale)
}
