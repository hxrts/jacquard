use std::collections::BTreeMap;

use jacquard_core::{
    NodeId, Observation, RouteEvent, RouteEventStamped, RouterRoundOutcome, Tick,
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
    PathwayCheckpointRecovery { completed_rounds: u32, host_count: usize },
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
