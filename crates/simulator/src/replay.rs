//! Simulation replay artifacts: host rounds, route events, checkpoints, and statistics.

use std::collections::BTreeMap;

use jacquard_core::{
    DestinationId, HealthScore, NodeId, Observation, ReachabilityState, RouteEvent,
    RouteEventStamped, RouteId, RouteLifecycleEvent, RouterRoundOutcome, RoutingEngineId, Tick,
};
use jacquard_mem_link_profile::InMemoryRuntimeEffects;

use crate::{
    environment::{AppliedEnvironmentHook, ScriptedEnvironmentModel},
    scenario::JacquardScenario,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostRoundStatus {
    Advanced {
        router_outcome: RouterRoundOutcome,
        ingested_transport_observation_count: usize,
        flushed_transport_commands: usize,
        dropped_transport_observations: usize,
    },
    Waiting {
        next_round_hint: jacquard_core::RoutingTickHint,
        pending_transport_observations: usize,
        pending_transport_commands: usize,
        dropped_transport_observations: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRoundArtifact {
    pub local_node_id: NodeId,
    pub ingress_batch_boundary: IngressBatchBoundary,
    pub status: HostRoundStatus,
    pub active_routes: Vec<ActiveRouteSummary>,
    pub field_replay: Option<FieldReplaySummary>,
    pub mercator_replay: Option<MercatorReplaySummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveRouteSummary {
    pub owner_node_id: NodeId,
    pub route_id: RouteId,
    pub destination: DestinationId,
    pub engine_id: RoutingEngineId,
    pub next_hop_node_id: Option<NodeId>,
    pub hop_count_hint: Option<u8>,
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub reachability_state: ReachabilityState,
    pub stability_score: HealthScore,
    pub commitment_resolution: Option<String>,
    pub field_continuity_band: Option<String>,
    pub field_last_outcome: Option<String>,
    pub field_last_promotion_decision: Option<String>,
    pub field_last_promotion_blocker: Option<String>,
    pub field_continuation_shift_count: Option<u32>,
    pub scatter_current_regime: Option<String>,
    pub scatter_last_action: Option<String>,
    pub scatter_retained_message_count: Option<u32>,
    pub scatter_delivered_message_count: Option<u32>,
    pub scatter_contact_rate: Option<u32>,
    pub scatter_diversity_score: Option<u32>,
    pub scatter_resource_pressure_permille: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReplaySummary {
    pub selected_result_present: bool,
    pub search_reconfiguration_present: bool,
    pub execution_policy: Option<String>,
    pub bootstrap_active: bool,
    pub continuity_band: Option<String>,
    pub last_continuity_transition: Option<String>,
    pub last_promotion_decision: Option<String>,
    pub last_promotion_blocker: Option<String>,
    pub bootstrap_activation_count: u32,
    pub bootstrap_hold_count: u32,
    pub bootstrap_narrow_count: u32,
    pub bootstrap_upgrade_count: u32,
    pub bootstrap_withdraw_count: u32,
    pub degraded_steady_entry_count: u32,
    pub degraded_steady_recovery_count: u32,
    pub degraded_to_bootstrap_count: u32,
    pub degraded_steady_round_count: u32,
    pub service_retention_carry_forward_count: u32,
    pub asymmetric_shift_success_count: u32,
    pub protocol_reconfiguration_count: usize,
    pub route_bound_reconfiguration_count: usize,
    pub continuation_shift_count: u32,
    pub corridor_narrow_count: u32,
    pub checkpoint_capture_count: u32,
    pub checkpoint_restore_count: u32,
    pub reconfiguration_causes: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MercatorReplaySummary {
    pub selected_result_rounds: u32,
    pub no_candidate_attempts: u32,
    pub inadmissible_candidate_attempts: u32,
    pub support_withdrawal_count: u32,
    pub stale_persistence_rounds: u32,
    pub active_route_count: u32,
    pub latest_topology_epoch: Option<jacquard_core::RouteEpoch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriverStatusEvent {
    IngressDropped {
        local_node_id: jacquard_core::NodeId,
        at_tick: Tick,
        dropped_transport_observations: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationFailureSummary {
    pub round_index: Option<u32>,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngressBatchBoundary {
    pub observed_at_tick: Tick,
    pub ingested_transport_observation_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TelltaleNativeArtifactRef {
    PathwayCheckpointRecovery {
        completed_rounds: u32,
        host_count: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostCheckpointSnapshot {
    pub local_node_id: NodeId,
    pub runtime_effects: InMemoryRuntimeEffects,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JacquardRoundArtifact {
    pub round_index: u32,
    pub topology: Observation<jacquard_core::Configuration>,
    pub environment_artifacts: Vec<AppliedEnvironmentHook>,
    pub host_rounds: Vec<HostRoundArtifact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JacquardCheckpointArtifact {
    pub completed_rounds: u32,
    pub topology: Observation<jacquard_core::Configuration>,
    pub host_snapshots: BTreeMap<NodeId, HostCheckpointSnapshot>,
    pub telltale_native: Option<TelltaleNativeArtifactRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JacquardReplayArtifact {
    pub scenario: JacquardScenario,
    pub environment_model: ScriptedEnvironmentModel,
    pub rounds: Vec<JacquardRoundArtifact>,
    pub route_events: Vec<RouteEvent>,
    pub stamped_route_events: Vec<RouteEventStamped>,
    pub driver_status_events: Vec<DriverStatusEvent>,
    pub failure_summaries: Vec<SimulationFailureSummary>,
    pub checkpoints: Vec<JacquardCheckpointArtifact>,
    pub telltale_native: Option<TelltaleNativeArtifactRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JacquardSimulationStats {
    pub executed_round_count: u32,
    pub advanced_round_count: u32,
    pub waiting_round_count: u32,
    pub route_event_count: usize,
    pub checkpoint_count: usize,
    pub driver_status_event_count: usize,
    pub failure_summary_count: usize,
}
