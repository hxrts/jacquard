//! Core `FieldEngine` type, engine identity, and capability advertisement.
//!
//! `FieldEngine<Transport, Effects>` is the facade through which the Jacquard
//! framework interacts with the field routing engine. It owns the local node
//! identity, transport effects, and private engine state, and implements both
//! `RoutingEnginePlanner` (planning surface) and `RoutingEngine` (runtime
//! hooks).
//!
//! `FIELD_ENGINE_ID` is the unique engine identifier derived from the string
//! `"jacquard.field.."`. `FIELD_CAPABILITIES` advertises `LinkProtected`
//! protection, `PartitionTolerant` connectivity, and `CorridorEnvelope` route
//! shape visibility. The field engine makes conservative end-to-end claims
//! rather than asserting explicit hop-by-hop paths.
// long-file-exception: engine.rs intentionally co-locates the versioned replay surfaces, reduced/exported replay projections, and the engine facade so the Rust-to-proof boundary stays auditable in one place.

use std::{cell::RefCell, collections::VecDeque};

use jacquard_core::{
    ConnectivityPosture, DestinationId, MaterializedRoute, NodeId, RouteCommitment, RouteEpoch,
    RouteId, RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteShapeVisibility,
    RoutingEngineCapabilities, RoutingEngineId, Tick,
};
use jacquard_traits::RoutingEngine;
use serde::{Deserialize, Serialize};

use crate::{
    choreography::{
        BlockedReceiveMarker, FieldExecutionPolicyClass, FieldHostWaitStatus, FieldProtocolKind,
        FieldProtocolReconfiguration, FieldProtocolReconfigurationCause, FieldProtocolRuntime,
        FieldRoundDisposition,
    },
    planner::{bootstrap_class_for_state, continuity_band_for_state},
    recovery::FieldRouteRecoveryState,
    route::{ActiveFieldRoute, FieldBootstrapClass, FieldContinuityBand},
    search::{
        FieldPlannerSearchRecord, FieldSearchConfig, FieldSearchEpoch, FieldSearchPlanningFailure,
        FieldSearchReconfiguration, FieldSearchSnapshotState,
    },
    state::{
        DestinationFieldState, DestinationInterestClass, DestinationKey, EntropyBucket,
        FieldEngineState, HopBand, OperatingRegime, RoutingPosture, SupportBucket,
    },
    summary::{
        EvidenceContributionClass, FieldSummary, ForwardPropagatedEvidence,
        ReverseFeedbackEvidence, SummaryDestinationKey, SummaryUncertaintyClass,
        FIELD_SUMMARY_ENCODING_BYTES,
    },
};
use telltale_search::{SearchEffortProfile, SearchQuery, SearchSchedulerProfile};

pub const FIELD_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.field..");

pub const FIELD_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: FIELD_ENGINE_ID,
    max_protection: RouteProtectionClass::LinkProtected,
    max_connectivity: ConnectivityPosture {
        repair: RouteRepairClass::Repairable,
        partition: RoutePartitionClass::PartitionTolerant,
    },
    repair_support: jacquard_core::RepairSupport::Unsupported,
    hold_support: jacquard_core::HoldSupport::Supported,
    decidable_admission: jacquard_core::DecidableSupport::Supported,
    quantitative_bounds: jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
    reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
    route_shape_visibility: RouteShapeVisibility::CorridorEnvelope,
};

pub const FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX: usize = 16;
pub const FIELD_REPLAY_SURFACE_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldReplaySurfaceClass {
    Semantic,
    Reduced,
    Observational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSearchReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub record: Option<FieldPlannerSearchRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProtocolReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub artifacts: Vec<crate::choreography::FieldProtocolArtifact>,
    pub reconfigurations: Vec<crate::choreography::FieldProtocolReconfiguration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub artifacts: Vec<FieldRuntimeRoundArtifact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRecoveryReplayEntry {
    pub route_id: RouteId,
    pub state: FieldRouteRecoveryState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRecoveryReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub entries: Vec<FieldRecoveryReplayEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldCommitmentReplayEntry {
    pub route_id: RouteId,
    pub commitments: Vec<RouteCommitment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldCommitmentReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub entries: Vec<FieldCommitmentReplayEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedReplayBundle {
    pub schema_version: u16,
    pub runtime_search: FieldExportedRuntimeSearchReplay,
    pub protocol: FieldExportedProtocolReplay,
    pub recovery: FieldExportedRecoveryReplay,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRuntimeSearchReplay {
    pub schema_version: u16,
    pub search: Option<FieldExportedSearchProjection>,
    pub runtime_artifacts: Vec<FieldExportedRuntimeRoundArtifact>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchProjection {
    pub objective_class: String,
    pub query: Option<FieldExportedSearchQuery>,
    pub execution_policy: FieldExportedSearchExecutionPolicy,
    pub selected_result: Option<FieldExportedSelectedResult>,
    pub snapshot_epoch: Option<FieldExportedSearchEpoch>,
    pub reconfiguration: Option<FieldExportedSearchReconfiguration>,
    pub planning_failure: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchQuery {
    pub start: NodeId,
    pub kind: String,
    pub accepted_goals: Vec<NodeId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchExecutionPolicy {
    pub scheduler_profile: String,
    pub batch_width: u64,
    pub exact: bool,
    pub run_to_completion: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSelectedResult {
    pub witness: Vec<NodeId>,
    pub selected_neighbor: Option<NodeId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchEpoch {
    pub route_epoch: u64,
    pub snapshot_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchReconfiguration {
    pub from: FieldExportedSearchEpoch,
    pub to: FieldExportedSearchEpoch,
    pub reseeding_policy: String,
    pub transition_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRuntimeRoundArtifact {
    pub protocol: String,
    pub destination: Option<DestinationId>,
    pub destination_class: Option<String>,
    pub blocked_receive: Option<String>,
    pub disposition: String,
    pub host_wait_status: String,
    pub emitted_count: usize,
    pub step_budget_remaining: u8,
    pub execution_policy: String,
    pub search_snapshot_epoch: Option<FieldExportedSearchEpoch>,
    pub search_selected_result_present: bool,
    pub search_reconfiguration_present: bool,
    pub router_artifact: Option<FieldExportedRuntimeRouteArtifact>,
    pub observed_at_tick: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRuntimeRouteArtifact {
    pub destination: DestinationId,
    pub route_shape: String,
    pub bootstrap_class: String,
    pub continuity_band: String,
    pub route_support: u16,
    pub topology_epoch: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedProtocolReplay {
    pub schema_version: u16,
    pub artifacts: Vec<FieldExportedProtocolArtifact>,
    pub reconfigurations: Vec<FieldExportedProtocolReconfiguration>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedProtocolArtifact {
    pub protocol: String,
    pub route_id: Option<RouteId>,
    pub topology_epoch: u64,
    pub destination: Option<DestinationId>,
    pub detail: String,
    pub last_updated_at: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedProtocolReconfiguration {
    pub protocol: String,
    pub route_id: Option<RouteId>,
    pub destination: Option<DestinationId>,
    pub prior_owner_tag: u64,
    pub next_owner_tag: u64,
    pub prior_generation: u32,
    pub next_generation: u32,
    pub cause: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRecoveryReplay {
    pub schema_version: u16,
    pub entries: Vec<FieldExportedRecoveryEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRecoveryEntry {
    pub route_id: RouteId,
    pub checkpoint_available: bool,
    pub last_trigger: Option<String>,
    pub last_outcome: Option<String>,
    pub bootstrap_active: bool,
    pub continuity_band: Option<String>,
    pub last_continuity_transition: Option<String>,
    pub last_bootstrap_transition: Option<String>,
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
    pub checkpoint_capture_count: u32,
    pub checkpoint_restore_count: u32,
    pub continuation_shift_count: u32,
    pub corridor_narrow_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanReplayFixture {
    pub scenario: String,
    pub search: Option<FieldLeanSearchFixture>,
    pub protocol: FieldLeanProtocolFixture,
    pub runtime: FieldLeanRuntimeLinkageFixture,
    pub recovery: Option<FieldLeanRecoveryFixture>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanSearchFixture {
    pub objective_class: String,
    pub query_kind: String,
    pub selected_neighbor_present: bool,
    pub snapshot_epoch_present: bool,
    pub planning_failure: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanProtocolFixture {
    pub reconfiguration_causes: Vec<String>,
    pub route_bound_reconfiguration_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanRuntimeLinkageFixture {
    pub artifact_count: usize,
    pub search_linked_artifact_count: usize,
    pub route_artifact_count: usize,
    pub bootstrap_route_artifact_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanRecoveryFixture {
    pub last_trigger: Option<String>,
    pub last_outcome: Option<String>,
    pub bootstrap_active: bool,
    pub continuity_band: Option<String>,
    pub last_continuity_transition: Option<String>,
    pub last_bootstrap_transition: Option<String>,
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
    pub checkpoint_capture_count: u32,
    pub checkpoint_restore_count: u32,
    pub continuation_shift_count: u32,
    pub corridor_narrow_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReplaySnapshot {
    pub schema_version: u16,
    pub search: FieldSearchReplaySurface,
    pub protocol: FieldProtocolReplaySurface,
    pub runtime: FieldRuntimeReplaySurface,
    pub recovery: FieldRecoveryReplaySurface,
    pub commitments: FieldCommitmentReplaySurface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldReducedObjectiveClass {
    Node,
    Gateway,
    Service,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldReducedQueryKind {
    SingleGoal,
    MultiGoal,
    CandidateSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedSearchQuery {
    pub start: NodeId,
    pub kind: FieldReducedQueryKind,
    pub accepted_goals: Vec<NodeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldReducedSearchExecutionPolicy {
    pub scheduler_profile: SearchSchedulerProfile,
    pub batch_width: u64,
    pub exact: bool,
    pub run_to_completion: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedSelectedResult {
    pub witness: Vec<NodeId>,
    pub selected_neighbor: Option<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedSearchProjection {
    pub objective_class: FieldReducedObjectiveClass,
    pub query: Option<FieldReducedSearchQuery>,
    pub execution_policy: FieldReducedSearchExecutionPolicy,
    pub selected_result: Option<FieldReducedSelectedResult>,
    pub snapshot_epoch: Option<FieldSearchEpoch>,
    pub reconfiguration: Option<FieldSearchReconfiguration>,
    pub planning_failure: Option<FieldSearchPlanningFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedRuntimeSearchReplay {
    pub schema_version: u16,
    pub search: Option<FieldReducedSearchProjection>,
    pub runtime_artifacts: Vec<FieldRuntimeRoundArtifact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedProtocolSession {
    pub protocol: FieldProtocolKind,
    pub route_id: Option<RouteId>,
    pub topology_epoch: RouteEpoch,
    pub destination: Option<DestinationId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedProtocolArtifact {
    pub session: FieldReducedProtocolSession,
    pub detail: String,
    pub last_updated_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedProtocolReconfiguration {
    pub prior_session: FieldReducedProtocolSession,
    pub next_session: FieldReducedProtocolSession,
    pub prior_owner_tag: u64,
    pub next_owner_tag: u64,
    pub prior_generation: u32,
    pub next_generation: u32,
    pub cause: FieldProtocolReconfigurationCause,
    pub recorded_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedProtocolReplay {
    pub schema_version: u16,
    pub artifacts: Vec<FieldReducedProtocolArtifact>,
    pub reconfigurations: Vec<FieldReducedProtocolReconfiguration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeRouteArtifact {
    pub destination: DestinationId,
    pub route_shape: RouteShapeVisibility,
    pub bootstrap_class: FieldBootstrapClass,
    pub continuity_band: FieldContinuityBand,
    pub route_support: u16,
    pub topology_epoch: RouteEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeRoundArtifact {
    pub protocol: FieldProtocolKind,
    pub destination: Option<DestinationId>,
    pub destination_class: Option<FieldReducedObjectiveClass>,
    pub blocked_receive: Option<BlockedReceiveMarker>,
    pub disposition: FieldRoundDisposition,
    pub host_wait_status: FieldHostWaitStatus,
    pub emitted_count: usize,
    pub step_budget_remaining: u8,
    pub execution_policy: FieldExecutionPolicyClass,
    pub search_snapshot_epoch: Option<FieldSearchEpoch>,
    pub search_selected_result_present: bool,
    pub search_reconfiguration_present: bool,
    pub router_artifact: Option<FieldRuntimeRouteArtifact>,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldForwardSummaryObservation {
    pub topology_epoch: RouteEpoch,
    pub observed_at_tick: Tick,
    pub delivery_support: u16,
    pub min_hops: u8,
    pub max_hops: u8,
}

impl FieldForwardSummaryObservation {
    #[must_use]
    pub fn new(
        topology_epoch: RouteEpoch,
        observed_at_tick: Tick,
        delivery_support: u16,
        min_hops: u8,
        max_hops: u8,
    ) -> Self {
        Self {
            topology_epoch,
            observed_at_tick,
            delivery_support,
            min_hops,
            max_hops,
        }
    }
}

pub struct FieldEngine<Transport, Effects> {
    pub(crate) local_node_id: NodeId,
    pub(crate) transport: Transport,
    #[expect(
        dead_code,
        reason = "phase-2 scaffold; observer/control updates use effects in later phases"
    )]
    pub(crate) effects: Effects,
    pub(crate) state: FieldEngineState,
    pub(crate) search_config: FieldSearchConfig,
    pub(crate) search_snapshot_state: RefCell<Option<FieldSearchSnapshotState>>,
    pub(crate) last_search_record: RefCell<Option<FieldPlannerSearchRecord>>,
    pub(crate) runtime_round_artifacts: RefCell<VecDeque<FieldRuntimeRoundArtifact>>,
    pub(crate) protocol_runtime: FieldProtocolRuntime,
    pub(crate) active_routes: std::collections::BTreeMap<RouteId, ActiveFieldRoute>,
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    #[must_use]
    pub fn new(local_node_id: NodeId, transport: Transport, effects: Effects) -> Self {
        Self {
            local_node_id,
            transport,
            effects,
            state: FieldEngineState::new(),
            search_config: FieldSearchConfig::default(),
            search_snapshot_state: RefCell::new(None),
            last_search_record: RefCell::new(None),
            runtime_round_artifacts: RefCell::new(VecDeque::new()),
            protocol_runtime: FieldProtocolRuntime::default(),
            active_routes: std::collections::BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_search_config(mut self, search_config: FieldSearchConfig) -> Self {
        self.search_config = search_config;
        self
    }

    #[must_use]
    pub fn search_config(&self) -> &FieldSearchConfig {
        &self.search_config
    }

    #[must_use]
    pub(crate) fn effective_search_config(&self) -> FieldSearchConfig {
        let desired_scheduler_profile =
            match (self.state.regime.current, self.state.posture.current) {
                (OperatingRegime::Congested, _)
                | (OperatingRegime::Unstable, RoutingPosture::RiskSuppressed)
                | (_, RoutingPosture::RiskSuppressed) => {
                    telltale_search::SearchSchedulerProfile::ThreadedExactSingleLane
                }
                _ => telltale_search::SearchSchedulerProfile::CanonicalSerial,
            };
        self.search_config
            .clone()
            .with_scheduler_profile(desired_scheduler_profile)
            .unwrap_or_else(|_| self.search_config.clone())
    }

    #[must_use]
    pub fn last_search_record(&self) -> Option<FieldPlannerSearchRecord> {
        self.last_search_record.borrow().clone()
    }

    #[must_use]
    pub fn runtime_round_artifacts(&self) -> Vec<FieldRuntimeRoundArtifact> {
        self.runtime_round_artifacts
            .borrow()
            .iter()
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn route_recovery_entries(&self) -> Vec<FieldRecoveryReplayEntry> {
        self.active_routes
            .iter()
            .map(|(route_id, active)| FieldRecoveryReplayEntry {
                route_id: *route_id,
                state: active.recovery.state.clone(),
            })
            .collect()
    }

    pub fn ingest_forward_summary(
        &mut self,
        from_neighbor: NodeId,
        payload: [u8; FIELD_SUMMARY_ENCODING_BYTES],
        observed_at_tick: Tick,
    ) -> Result<(), &'static str> {
        let summary = FieldSummary::decode(payload)?;
        let destination = DestinationId::from(&DestinationKey::from(&summary.destination));
        let state = self.state.upsert_destination_interest(
            &destination,
            DestinationInterestClass::Propagated,
            observed_at_tick,
        );
        state
            .pending_forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor,
                summary,
                observed_at_tick,
            });
        Ok(())
    }

    // long-block-exception: forward summary recording keeps destination
    // upsert and evidence normalization in one ingestion path.
    pub fn record_forward_summary(
        &mut self,
        destination: &DestinationId,
        from_neighbor: NodeId,
        observation: FieldForwardSummaryObservation,
    ) {
        let service_bias = matches!(destination, DestinationId::Service(_));
        let state = self.state.upsert_destination_interest(
            destination,
            DestinationInterestClass::Propagated,
            observation.observed_at_tick,
        );
        state
            .pending_forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor,
                summary: FieldSummary {
                    destination: SummaryDestinationKey::from(destination),
                    topology_epoch: observation.topology_epoch,
                    freshness_tick: observation.observed_at_tick,
                    hop_band: HopBand::new(observation.min_hops, observation.max_hops),
                    delivery_support: SupportBucket::new(observation.delivery_support),
                    congestion_penalty: EntropyBucket::default(),
                    retention_support: SupportBucket::new(if service_bias {
                        observation.delivery_support.saturating_sub(40)
                    } else {
                        0
                    }),
                    uncertainty_penalty: EntropyBucket::default(),
                    evidence_class: EvidenceContributionClass::ForwardPropagated,
                    uncertainty_class: SummaryUncertaintyClass::Low,
                },
                observed_at_tick: observation.observed_at_tick,
            });
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.posterior.top_corridor_mass = SupportBucket::new(
            state
                .posterior
                .top_corridor_mass
                .value()
                .max(observation.delivery_support.saturating_sub(40)),
        );
        state.corridor_belief.delivery_support = SupportBucket::new(
            state
                .corridor_belief
                .delivery_support
                .value()
                .max(observation.delivery_support.saturating_sub(60)),
        );
        state.corridor_belief.retention_affinity = SupportBucket::new(
            state
                .corridor_belief
                .retention_affinity
                .value()
                .max(observation.delivery_support.saturating_sub(80)),
        );
        state.corridor_belief.expected_hop_band = HopBand::new(
            observation.min_hops.saturating_add(1),
            observation.max_hops.saturating_add(1),
        );
        state.frontier = state
            .frontier
            .clone()
            .insert(crate::state::NeighborContinuation {
                neighbor_id: from_neighbor,
                net_value: SupportBucket::new(observation.delivery_support),
                downstream_support: SupportBucket::new(observation.delivery_support),
                expected_hop_band: HopBand::new(
                    observation.min_hops.saturating_add(1),
                    observation.max_hops.saturating_add(1),
                ),
                freshness: observation.observed_at_tick,
            });
        if service_bias {
            reinforce_service_bootstrap_fanout(state, observation.delivery_support);
        }
    }

    pub fn record_reverse_feedback(
        &mut self,
        destination: &DestinationId,
        from_neighbor: NodeId,
        delivery_feedback: u16,
        observed_at_tick: Tick,
    ) {
        let state = self.state.upsert_destination_interest(
            destination,
            DestinationInterestClass::Transit,
            observed_at_tick,
        );
        state
            .pending_reverse_feedback
            .push(ReverseFeedbackEvidence {
                from_neighbor,
                delivery_feedback: SupportBucket::new(delivery_feedback),
                observed_at_tick,
            });
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ReverseValidated;
        state.posterior.usability_entropy = EntropyBucket::new(
            state
                .posterior
                .usability_entropy
                .value()
                .saturating_sub(120),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(
            state
                .posterior
                .top_corridor_mass
                .value()
                .max(delivery_feedback.saturating_sub(20)),
        );
        state.corridor_belief.delivery_support = SupportBucket::new(
            state
                .corridor_belief
                .delivery_support
                .value()
                .max(delivery_feedback.saturating_sub(40)),
        );
        state.corridor_belief.retention_affinity = SupportBucket::new(
            state
                .corridor_belief
                .retention_affinity
                .value()
                .max(delivery_feedback.saturating_sub(60)),
        );
    }

    #[must_use]
    pub fn protocol_artifacts(&self) -> Vec<crate::choreography::FieldProtocolArtifact> {
        self.protocol_runtime.artifacts()
    }

    #[must_use]
    pub fn replay_snapshot(&self, routes: &[MaterializedRoute]) -> FieldReplaySnapshot
    where
        Self: jacquard_traits::RoutingEngine,
    {
        FieldReplaySnapshot {
            schema_version: FIELD_REPLAY_SURFACE_VERSION,
            search: FieldSearchReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Observational,
                record: self.last_search_record(),
            },
            protocol: FieldProtocolReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Observational,
                artifacts: self.protocol_artifacts(),
                reconfigurations: self.protocol_runtime.reconfigurations(),
            },
            runtime: FieldRuntimeReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Reduced,
                artifacts: self.runtime_round_artifacts(),
            },
            recovery: FieldRecoveryReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Reduced,
                entries: self.route_recovery_entries(),
            },
            commitments: FieldCommitmentReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Observational,
                entries: routes
                    .iter()
                    .map(|route| FieldCommitmentReplayEntry {
                        route_id: *route.identity.route_id(),
                        commitments: self.route_commitments(route),
                    })
                    .collect(),
            },
        }
    }

    #[must_use]
    pub fn exported_replay_bundle(&self, routes: &[MaterializedRoute]) -> FieldExportedReplayBundle
    where
        Self: jacquard_traits::RoutingEngine,
    {
        let snapshot = self.replay_snapshot(routes);
        snapshot.exported_bundle()
    }

    pub fn exported_replay_bundle_json(
        &self,
        routes: &[MaterializedRoute],
    ) -> Result<String, serde_json::Error>
    where
        Self: jacquard_traits::RoutingEngine,
    {
        serde_json::to_string_pretty(&self.exported_replay_bundle(routes))
    }

    pub(crate) fn runtime_route_artifact_for_destination(
        &self,
        destination: &DestinationId,
        destination_state: &DestinationFieldState,
        topology_epoch: RouteEpoch,
    ) -> FieldRuntimeRouteArtifact {
        let route_shape = if destination_state.frontier.as_slice().is_empty()
            || destination_state.corridor_belief.delivery_support.value() == 0
        {
            RouteShapeVisibility::Opaque
        } else {
            RouteShapeVisibility::CorridorEnvelope
        };
        FieldRuntimeRouteArtifact {
            destination: destination.clone(),
            route_shape,
            bootstrap_class: bootstrap_class_for_state(destination_state),
            continuity_band: continuity_band_for_state(destination_state),
            route_support: destination_state.corridor_belief.delivery_support.value(),
            topology_epoch,
        }
    }

    pub(crate) fn record_runtime_round_artifact(&self, artifact: FieldRuntimeRoundArtifact) {
        let mut retained = self.runtime_round_artifacts.borrow_mut();
        if retained.len() >= FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX {
            retained.pop_front();
        }
        retained.push_back(artifact);
    }
}

fn reinforce_service_bootstrap_fanout(
    state: &mut crate::state::DestinationFieldState,
    delivery_support: u16,
) {
    let coherent_branch_count = service_bootstrap_branch_count(state);
    if coherent_branch_count < 2 {
        return;
    }
    let corroboration_bonus = u16::try_from(
        coherent_branch_count
            .saturating_sub(1)
            .saturating_mul(70)
            .min(220),
    )
    .expect("bounded corroboration bonus fits u16");
    state.posterior.top_corridor_mass = SupportBucket::new(
        state.posterior.top_corridor_mass.value().max(
            delivery_support
                .saturating_sub(10)
                .saturating_add(corroboration_bonus),
        ),
    );
    state.corridor_belief.delivery_support = SupportBucket::new(
        state.corridor_belief.delivery_support.value().max(
            delivery_support
                .saturating_sub(35)
                .saturating_add(corroboration_bonus / 2),
        ),
    );
    state.corridor_belief.retention_affinity = SupportBucket::new(
        state.corridor_belief.retention_affinity.value().max(
            delivery_support
                .saturating_sub(20)
                .saturating_add(corroboration_bonus),
        ),
    );
}

fn service_bootstrap_branch_count(state: &crate::state::DestinationFieldState) -> usize {
    let mut neighbors = std::collections::BTreeSet::new();
    for entry in state.frontier.as_slice() {
        if entry.downstream_support.value() >= 140 && entry.net_value.value() >= 180 {
            neighbors.insert(entry.neighbor_id);
        }
    }
    for evidence in &state.pending_forward_evidence {
        if evidence.summary.retention_support.value() >= 140
            && evidence.summary.delivery_support.value() >= 120
            && evidence.summary.uncertainty_penalty.value() <= 900
        {
            neighbors.insert(evidence.from_neighbor);
        }
    }
    neighbors.len()
}

fn reduced_objective_class(
    objective: &jacquard_core::RoutingObjective,
) -> FieldReducedObjectiveClass {
    reduced_objective_class_for_destination(&objective.destination)
}

fn reduced_objective_class_for_destination(
    destination: &DestinationId,
) -> FieldReducedObjectiveClass {
    match destination {
        DestinationId::Node(_) => FieldReducedObjectiveClass::Node,
        DestinationId::Gateway(_) => FieldReducedObjectiveClass::Gateway,
        DestinationId::Service(_) => FieldReducedObjectiveClass::Service,
    }
}

fn reduced_query(query: &SearchQuery<NodeId>) -> FieldReducedSearchQuery {
    match query {
        SearchQuery::SingleGoal { start, goal } => FieldReducedSearchQuery {
            start: *start,
            kind: FieldReducedQueryKind::SingleGoal,
            accepted_goals: vec![*goal],
        },
        SearchQuery::MultiGoal { start, goals } => FieldReducedSearchQuery {
            start: *start,
            kind: FieldReducedQueryKind::MultiGoal,
            accepted_goals: goals.clone(),
        },
        SearchQuery::CandidateSet {
            start, candidates, ..
        } => FieldReducedSearchQuery {
            start: *start,
            kind: FieldReducedQueryKind::CandidateSet,
            accepted_goals: candidates.clone(),
        },
    }
}

fn reduced_execution_policy(config: &FieldSearchConfig) -> FieldReducedSearchExecutionPolicy {
    let policy = config.execution_policy();
    FieldReducedSearchExecutionPolicy {
        scheduler_profile: policy.scheduler_profile,
        batch_width: policy.batch_width,
        exact: matches!(
            (policy.scheduler_profile, policy.effort_profile),
            (
                SearchSchedulerProfile::CanonicalSerial
                    | SearchSchedulerProfile::ThreadedExactSingleLane
                    | SearchSchedulerProfile::BatchedParallelExact,
                SearchEffortProfile::RunToCompletion
            )
        ),
        run_to_completion: matches!(policy.effort_profile, SearchEffortProfile::RunToCompletion),
    }
}

fn reduced_selected_result(
    record: &FieldPlannerSearchRecord,
) -> Option<FieldReducedSelectedResult> {
    let witness = record
        .selected_continuation
        .as_ref()
        .map(|continuation| continuation.selected_private_witness.clone())
        .or_else(|| {
            record
                .run
                .as_ref()
                .and_then(|run| run.selected_node_path.clone())
        })
        .or_else(|| {
            record
                .run
                .as_ref()
                .and_then(|run| run.report.observation.selected_result_witness.clone())
        })?;
    let selected_neighbor = record
        .selected_continuation
        .as_ref()
        .map(|continuation| continuation.chosen_neighbor)
        .or_else(|| witness.get(1).copied());
    Some(FieldReducedSelectedResult {
        witness,
        selected_neighbor,
    })
}

fn reduced_search_projection(record: &FieldPlannerSearchRecord) -> FieldReducedSearchProjection {
    FieldReducedSearchProjection {
        objective_class: reduced_objective_class(&record.objective),
        query: record.query.as_ref().map(reduced_query),
        execution_policy: reduced_execution_policy(&record.effective_config),
        selected_result: reduced_selected_result(record),
        snapshot_epoch: record
            .run
            .as_ref()
            .map(|run| run.report.final_state.epoch.clone()),
        reconfiguration: record
            .run
            .as_ref()
            .and_then(|run| run.reconfiguration.clone()),
        planning_failure: record.planning_failure,
    }
}

fn reduced_protocol_session(
    session: &crate::choreography::FieldProtocolSessionKey,
) -> FieldReducedProtocolSession {
    FieldReducedProtocolSession {
        protocol: session.protocol(),
        route_id: session.route_id(),
        topology_epoch: session.topology_epoch(),
        destination: session.destination(),
    }
}

fn reduced_protocol_artifact(
    artifact: &crate::choreography::FieldProtocolArtifact,
) -> FieldReducedProtocolArtifact {
    FieldReducedProtocolArtifact {
        session: reduced_protocol_session(artifact.session()),
        detail: artifact.detail.as_str().to_owned(),
        last_updated_at: artifact.last_updated_at,
    }
}

fn reduced_protocol_reconfiguration(
    reconfiguration: &FieldProtocolReconfiguration,
) -> FieldReducedProtocolReconfiguration {
    FieldReducedProtocolReconfiguration {
        prior_session: reduced_protocol_session(&reconfiguration.prior_session),
        next_session: reduced_protocol_session(&reconfiguration.next_session),
        prior_owner_tag: reconfiguration.prior_owner_tag,
        next_owner_tag: reconfiguration.next_owner_tag,
        prior_generation: reconfiguration.prior_generation,
        next_generation: reconfiguration.next_generation,
        cause: reconfiguration.cause,
        recorded_at: reconfiguration.recorded_at,
    }
}

impl FieldReplaySnapshot {
    #[must_use]
    pub fn reduced_runtime_search_replay(&self) -> FieldReducedRuntimeSearchReplay {
        FieldReducedRuntimeSearchReplay {
            schema_version: self.schema_version,
            search: self.search.record.as_ref().map(reduced_search_projection),
            runtime_artifacts: self.runtime.artifacts.clone(),
        }
    }

    #[must_use]
    pub fn reduced_protocol_replay(&self) -> FieldReducedProtocolReplay {
        FieldReducedProtocolReplay {
            schema_version: self.schema_version,
            artifacts: self
                .protocol
                .artifacts
                .iter()
                .map(reduced_protocol_artifact)
                .collect(),
            reconfigurations: self
                .protocol
                .reconfigurations
                .iter()
                .map(reduced_protocol_reconfiguration)
                .collect(),
        }
    }

    #[must_use]
    pub fn exported_bundle(&self) -> FieldExportedReplayBundle {
        FieldExportedReplayBundle {
            schema_version: self.schema_version,
            runtime_search: exported_runtime_search_replay(&self.reduced_runtime_search_replay()),
            protocol: exported_protocol_replay(&self.reduced_protocol_replay()),
            recovery: exported_recovery_replay(&self.recovery),
        }
    }

    pub fn exported_bundle_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.exported_bundle())
    }
}

impl FieldExportedReplayBundle {
    // long-block-exception: this helper intentionally projects the full reduced replay bundle into one proof-facing fixture object in one auditable place.
    #[must_use]
    pub fn lean_replay_fixture(&self, scenario: impl Into<String>) -> FieldLeanReplayFixture {
        let recovery = self
            .recovery
            .entries
            .first()
            .map(|entry| FieldLeanRecoveryFixture {
                last_trigger: entry.last_trigger.clone(),
                last_outcome: entry.last_outcome.clone(),
                bootstrap_active: entry.bootstrap_active,
                continuity_band: entry.continuity_band.clone(),
                last_continuity_transition: entry.last_continuity_transition.clone(),
                last_bootstrap_transition: entry.last_bootstrap_transition.clone(),
                last_promotion_decision: entry.last_promotion_decision.clone(),
                last_promotion_blocker: entry.last_promotion_blocker.clone(),
                bootstrap_activation_count: entry.bootstrap_activation_count,
                bootstrap_hold_count: entry.bootstrap_hold_count,
                bootstrap_narrow_count: entry.bootstrap_narrow_count,
                bootstrap_upgrade_count: entry.bootstrap_upgrade_count,
                bootstrap_withdraw_count: entry.bootstrap_withdraw_count,
                degraded_steady_entry_count: entry.degraded_steady_entry_count,
                degraded_steady_recovery_count: entry.degraded_steady_recovery_count,
                degraded_to_bootstrap_count: entry.degraded_to_bootstrap_count,
                degraded_steady_round_count: entry.degraded_steady_round_count,
                service_retention_carry_forward_count: entry.service_retention_carry_forward_count,
                asymmetric_shift_success_count: entry.asymmetric_shift_success_count,
                checkpoint_capture_count: entry.checkpoint_capture_count,
                checkpoint_restore_count: entry.checkpoint_restore_count,
                continuation_shift_count: entry.continuation_shift_count,
                corridor_narrow_count: entry.corridor_narrow_count,
            });
        FieldLeanReplayFixture {
            scenario: scenario.into(),
            search: self
                .runtime_search
                .search
                .as_ref()
                .map(|search| FieldLeanSearchFixture {
                    objective_class: search.objective_class.clone(),
                    query_kind: search
                        .query
                        .as_ref()
                        .map_or_else(|| "None".to_string(), |query| query.kind.clone()),
                    selected_neighbor_present: search
                        .selected_result
                        .as_ref()
                        .is_some_and(|selected| selected.selected_neighbor.is_some()),
                    snapshot_epoch_present: search.snapshot_epoch.is_some(),
                    planning_failure: search.planning_failure.clone(),
                }),
            protocol: FieldLeanProtocolFixture {
                reconfiguration_causes: self
                    .protocol
                    .reconfigurations
                    .iter()
                    .map(|step| step.cause.clone())
                    .collect(),
                route_bound_reconfiguration_count: self
                    .protocol
                    .reconfigurations
                    .iter()
                    .filter(|step| step.route_id.is_some())
                    .count(),
            },
            runtime: FieldLeanRuntimeLinkageFixture {
                artifact_count: self.runtime_search.runtime_artifacts.len(),
                search_linked_artifact_count: self
                    .runtime_search
                    .runtime_artifacts
                    .iter()
                    .filter(|artifact| artifact.search_snapshot_epoch.is_some())
                    .count(),
                route_artifact_count: self
                    .runtime_search
                    .runtime_artifacts
                    .iter()
                    .filter(|artifact| artifact.router_artifact.is_some())
                    .count(),
                bootstrap_route_artifact_count: self
                    .runtime_search
                    .runtime_artifacts
                    .iter()
                    .filter(|artifact| {
                        artifact
                            .router_artifact
                            .as_ref()
                            .is_some_and(|route| route.bootstrap_class == "Bootstrap")
                    })
                    .count(),
            },
            recovery,
        }
    }
}

fn exported_runtime_search_replay(
    replay: &FieldReducedRuntimeSearchReplay,
) -> FieldExportedRuntimeSearchReplay {
    FieldExportedRuntimeSearchReplay {
        schema_version: replay.schema_version,
        search: replay.search.as_ref().map(exported_search_projection),
        runtime_artifacts: replay
            .runtime_artifacts
            .iter()
            .map(exported_runtime_round_artifact)
            .collect(),
    }
}

fn exported_search_projection(
    projection: &FieldReducedSearchProjection,
) -> FieldExportedSearchProjection {
    FieldExportedSearchProjection {
        objective_class: format!("{:?}", projection.objective_class),
        query: projection.query.as_ref().map(exported_search_query),
        execution_policy: FieldExportedSearchExecutionPolicy {
            scheduler_profile: format!("{:?}", projection.execution_policy.scheduler_profile),
            batch_width: projection.execution_policy.batch_width,
            exact: projection.execution_policy.exact,
            run_to_completion: projection.execution_policy.run_to_completion,
        },
        selected_result: projection.selected_result.as_ref().map(|selected| {
            FieldExportedSelectedResult {
                witness: selected.witness.clone(),
                selected_neighbor: selected.selected_neighbor,
            }
        }),
        snapshot_epoch: projection
            .snapshot_epoch
            .as_ref()
            .map(exported_search_epoch),
        reconfiguration: projection
            .reconfiguration
            .as_ref()
            .map(exported_search_reconfiguration),
        planning_failure: projection
            .planning_failure
            .map(|failure| format!("{failure:?}")),
    }
}

fn exported_search_query(query: &FieldReducedSearchQuery) -> FieldExportedSearchQuery {
    FieldExportedSearchQuery {
        start: query.start,
        kind: format!("{:?}", query.kind),
        accepted_goals: query.accepted_goals.clone(),
    }
}

fn exported_search_epoch(epoch: &FieldSearchEpoch) -> FieldExportedSearchEpoch {
    FieldExportedSearchEpoch {
        route_epoch: epoch.route_epoch.0,
        snapshot_id: format!("{:?}", epoch.snapshot_id.0),
    }
}

fn exported_search_reconfiguration(
    reconfiguration: &FieldSearchReconfiguration,
) -> FieldExportedSearchReconfiguration {
    FieldExportedSearchReconfiguration {
        from: exported_search_epoch(&reconfiguration.from),
        to: exported_search_epoch(&reconfiguration.to),
        reseeding_policy: format!("{:?}", reconfiguration.reseeding_policy),
        transition_class: format!("{:?}", reconfiguration.transition_class),
    }
}

fn exported_runtime_round_artifact(
    artifact: &FieldRuntimeRoundArtifact,
) -> FieldExportedRuntimeRoundArtifact {
    FieldExportedRuntimeRoundArtifact {
        protocol: format!("{:?}", artifact.protocol),
        destination: artifact.destination.clone(),
        destination_class: artifact.destination_class.map(|class| format!("{class:?}")),
        blocked_receive: artifact.blocked_receive.map(|marker| format!("{marker:?}")),
        disposition: format!("{:?}", artifact.disposition),
        host_wait_status: format!("{:?}", artifact.host_wait_status),
        emitted_count: artifact.emitted_count,
        step_budget_remaining: artifact.step_budget_remaining,
        execution_policy: format!("{:?}", artifact.execution_policy),
        search_snapshot_epoch: artifact
            .search_snapshot_epoch
            .as_ref()
            .map(exported_search_epoch),
        search_selected_result_present: artifact.search_selected_result_present,
        search_reconfiguration_present: artifact.search_reconfiguration_present,
        router_artifact: artifact.router_artifact.as_ref().map(|route| {
            FieldExportedRuntimeRouteArtifact {
                destination: route.destination.clone(),
                route_shape: format!("{:?}", route.route_shape),
                bootstrap_class: format!("{:?}", route.bootstrap_class),
                continuity_band: format!("{:?}", route.continuity_band),
                route_support: route.route_support,
                topology_epoch: route.topology_epoch.0,
            }
        }),
        observed_at_tick: artifact.observed_at_tick.0,
    }
}

fn exported_protocol_replay(replay: &FieldReducedProtocolReplay) -> FieldExportedProtocolReplay {
    FieldExportedProtocolReplay {
        schema_version: replay.schema_version,
        artifacts: replay
            .artifacts
            .iter()
            .map(|artifact| FieldExportedProtocolArtifact {
                protocol: format!("{:?}", artifact.session.protocol),
                route_id: artifact.session.route_id,
                topology_epoch: artifact.session.topology_epoch.0,
                destination: artifact.session.destination.clone(),
                detail: artifact.detail.clone(),
                last_updated_at: artifact.last_updated_at.0,
            })
            .collect(),
        reconfigurations: replay
            .reconfigurations
            .iter()
            .map(|step| FieldExportedProtocolReconfiguration {
                protocol: format!("{:?}", step.prior_session.protocol),
                route_id: step.prior_session.route_id,
                destination: step.prior_session.destination.clone(),
                prior_owner_tag: step.prior_owner_tag,
                next_owner_tag: step.next_owner_tag,
                prior_generation: step.prior_generation,
                next_generation: step.next_generation,
                cause: format!("{:?}", step.cause),
                recorded_at: step.recorded_at.0,
            })
            .collect(),
    }
}

fn exported_recovery_replay(replay: &FieldRecoveryReplaySurface) -> FieldExportedRecoveryReplay {
    FieldExportedRecoveryReplay {
        schema_version: replay.schema_version,
        entries: replay
            .entries
            .iter()
            .map(|entry| FieldExportedRecoveryEntry {
                route_id: entry.route_id,
                checkpoint_available: entry.state.checkpoint_available,
                last_trigger: entry
                    .state
                    .last_trigger
                    .map(|trigger| format!("{trigger:?}")),
                last_outcome: entry
                    .state
                    .last_outcome
                    .map(|outcome| format!("{outcome:?}")),
                bootstrap_active: entry.state.bootstrap_active,
                continuity_band: entry.state.continuity_band.map(|band| format!("{band:?}")),
                last_continuity_transition: entry
                    .state
                    .last_continuity_transition
                    .map(|transition| format!("{transition:?}")),
                last_bootstrap_transition: entry
                    .state
                    .last_bootstrap_transition
                    .map(|transition| format!("{transition:?}")),
                last_promotion_decision: entry
                    .state
                    .last_promotion_decision
                    .map(|decision| format!("{decision:?}")),
                last_promotion_blocker: entry
                    .state
                    .last_promotion_blocker
                    .map(|blocker| format!("{blocker:?}")),
                bootstrap_activation_count: entry.state.bootstrap_activation_count,
                bootstrap_hold_count: entry.state.bootstrap_hold_count,
                bootstrap_narrow_count: entry.state.bootstrap_narrow_count,
                bootstrap_upgrade_count: entry.state.bootstrap_upgrade_count,
                bootstrap_withdraw_count: entry.state.bootstrap_withdraw_count,
                degraded_steady_entry_count: entry.state.degraded_steady_entry_count,
                degraded_steady_recovery_count: entry.state.degraded_steady_recovery_count,
                degraded_to_bootstrap_count: entry.state.degraded_to_bootstrap_count,
                degraded_steady_round_count: entry.state.degraded_steady_round_count,
                service_retention_carry_forward_count: entry
                    .state
                    .service_retention_carry_forward_count,
                asymmetric_shift_success_count: entry.state.asymmetric_shift_success_count,
                checkpoint_capture_count: entry.state.checkpoint_capture_count,
                checkpoint_restore_count: entry.state.checkpoint_restore_count,
                continuation_shift_count: entry.state.continuation_shift_count,
                corridor_narrow_count: entry.state.corridor_narrow_count,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{DestinationId, LinkEndpoint, RouteEpoch, ServiceId, Tick, TransportError};
    use jacquard_traits::{effect_handler, TransportSenderEffects};
    use telltale_search::SearchSchedulerProfile;

    use super::*;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct NoopTransport;

    #[effect_handler]
    impl TransportSenderEffects for NoopTransport {
        fn send_transport(
            &mut self,
            _endpoint: &LinkEndpoint,
            _payload: &[u8],
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    #[test]
    fn effective_search_config_tracks_posture_without_changing_field_defaults() {
        let mut engine = FieldEngine::new(node(1), NoopTransport, ());
        assert_eq!(
            engine.effective_search_config().scheduler_profile(),
            SearchSchedulerProfile::CanonicalSerial,
        );

        engine.state.posture.current = RoutingPosture::RiskSuppressed;
        let expected = if cfg!(target_arch = "wasm32") {
            SearchSchedulerProfile::CanonicalSerial
        } else {
            SearchSchedulerProfile::ThreadedExactSingleLane
        };
        assert_eq!(
            engine.effective_search_config().scheduler_profile(),
            expected
        );
        assert_eq!(
            engine
                .effective_search_config()
                .per_objective_query_budget(),
            engine.search_config.per_objective_query_budget(),
        );
    }

    #[test]
    fn replay_snapshot_is_versioned_and_surface_typed() {
        let engine = FieldEngine::new(node(1), NoopTransport, ());
        let snapshot = engine.replay_snapshot(&[]);
        assert_eq!(snapshot.schema_version, FIELD_REPLAY_SURFACE_VERSION);
        assert_eq!(
            snapshot.search.surface_class,
            FieldReplaySurfaceClass::Observational
        );
        assert_eq!(
            snapshot.protocol.surface_class,
            FieldReplaySurfaceClass::Observational
        );
        assert_eq!(
            snapshot.runtime.surface_class,
            FieldReplaySurfaceClass::Reduced
        );
        assert_eq!(
            snapshot.recovery.surface_class,
            FieldReplaySurfaceClass::Reduced
        );
        assert_eq!(
            snapshot.commitments.surface_class,
            FieldReplaySurfaceClass::Observational
        );
    }

    #[test]
    fn replay_snapshot_matches_direct_public_surfaces() {
        let engine = FieldEngine::new(node(1), NoopTransport, ());
        let snapshot = engine.replay_snapshot(&[]);
        assert_eq!(snapshot.search.record, engine.last_search_record());
        assert_eq!(snapshot.protocol.artifacts, engine.protocol_artifacts());
        assert_eq!(
            snapshot.protocol.reconfigurations,
            engine.protocol_runtime.reconfigurations()
        );
        assert_eq!(snapshot.runtime.artifacts, engine.runtime_round_artifacts());
        assert_eq!(snapshot.recovery.entries, engine.route_recovery_entries());
        assert!(snapshot.commitments.entries.is_empty());
    }

    #[test]
    fn replay_snapshot_runtime_surface_stays_bounded() {
        let engine = FieldEngine::new(node(1), NoopTransport, ());
        for index in 0..(FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX + 4) {
            engine.record_runtime_round_artifact(FieldRuntimeRoundArtifact {
                protocol: crate::choreography::FieldProtocolKind::SummaryDissemination,
                destination: None,
                destination_class: None,
                blocked_receive: None,
                disposition: crate::choreography::FieldRoundDisposition::Continue,
                host_wait_status: crate::choreography::FieldHostWaitStatus::Idle,
                emitted_count: index,
                step_budget_remaining: 1,
                execution_policy: crate::choreography::FieldExecutionPolicyClass::Cheap,
                search_snapshot_epoch: None,
                search_selected_result_present: false,
                search_reconfiguration_present: false,
                router_artifact: None,
                observed_at_tick: Tick(u64::try_from(index).expect("test index fits")),
            });
        }

        let snapshot = engine.replay_snapshot(&[]);
        assert_eq!(
            snapshot.runtime.artifacts.len(),
            FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX
        );
    }

    #[test]
    fn record_forward_summary_reinforces_service_fanout_before_refresh() {
        let mut engine = FieldEngine::new(node(1), NoopTransport, ());
        let destination = DestinationId::Service(ServiceId(vec![9; 16]));
        for (neighbor, support) in [(node(2), 910), (node(3), 840), (node(4), 780)] {
            engine.record_forward_summary(
                &destination,
                neighbor,
                FieldForwardSummaryObservation::new(RouteEpoch(1), Tick(1), support, 1, 2),
            );
        }

        let state = engine
            .state
            .destinations
            .get(&crate::state::DestinationKey::Service(vec![9; 16]))
            .expect("tracked service destination");
        assert_eq!(state.frontier.len(), 3);
        assert!(
            state.posterior.top_corridor_mass.value() >= 980,
            "service fanout should corroborate corridor mass early: {}",
            state.posterior.top_corridor_mass.value()
        );
        assert!(
            state.corridor_belief.retention_affinity.value() >= 900,
            "service fanout should seed strong retention before refresh: {}",
            state.corridor_belief.retention_affinity.value()
        );
    }
}
