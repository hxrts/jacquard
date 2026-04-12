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

use std::{cell::RefCell, collections::VecDeque};

use jacquard_core::{
    ConnectivityPosture, DestinationId, MaterializedRoute, NodeId, RouteCommitment, RouteEpoch,
    RouteId, RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteShapeVisibility,
    RoutingEngineCapabilities, RoutingEngineId, Tick,
};
use jacquard_traits::RoutingEngine;

use crate::{
    choreography::{
        BlockedReceiveMarker, FieldExecutionPolicyClass, FieldHostWaitStatus, FieldProtocolKind,
        FieldProtocolRuntime, FieldRoundDisposition,
    },
    route::ActiveFieldRoute,
    search::{FieldPlannerSearchRecord, FieldSearchConfig, FieldSearchSnapshotState},
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub artifacts: Vec<FieldRuntimeRoundArtifact>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReplaySnapshot {
    pub schema_version: u16,
    pub search: FieldSearchReplaySurface,
    pub protocol: FieldProtocolReplaySurface,
    pub runtime: FieldRuntimeReplaySurface,
    pub commitments: FieldCommitmentReplaySurface,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeRouteArtifact {
    pub destination: DestinationId,
    pub route_shape: RouteShapeVisibility,
    pub route_support: u16,
    pub topology_epoch: RouteEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeRoundArtifact {
    pub protocol: FieldProtocolKind,
    pub destination: Option<DestinationId>,
    pub blocked_receive: Option<BlockedReceiveMarker>,
    pub disposition: FieldRoundDisposition,
    pub host_wait_status: FieldHostWaitStatus,
    pub emitted_count: usize,
    pub step_budget_remaining: u8,
    pub execution_policy: FieldExecutionPolicyClass,
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

    pub fn record_forward_summary(
        &mut self,
        destination: &DestinationId,
        from_neighbor: NodeId,
        observation: FieldForwardSummaryObservation,
    ) {
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
                    retention_support: SupportBucket::default(),
                    uncertainty_penalty: EntropyBucket::default(),
                    evidence_class: EvidenceContributionClass::ForwardPropagated,
                    uncertainty_class: SummaryUncertaintyClass::Low,
                },
                observed_at_tick: observation.observed_at_tick,
            });
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
            },
            runtime: FieldRuntimeReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Reduced,
                artifacts: self.runtime_round_artifacts(),
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

#[cfg(test)]
mod tests {
    use telltale_search::SearchSchedulerProfile;

    use super::*;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    #[test]
    fn effective_search_config_tracks_posture_without_changing_field_defaults() {
        let mut engine = FieldEngine::new(node(1), (), ());
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
        let engine = FieldEngine::new(node(1), (), ());
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
            snapshot.commitments.surface_class,
            FieldReplaySurfaceClass::Observational
        );
    }

    #[test]
    fn replay_snapshot_matches_direct_public_surfaces() {
        let engine = FieldEngine::new(node(1), (), ());
        let snapshot = engine.replay_snapshot(&[]);
        assert_eq!(snapshot.search.record, engine.last_search_record());
        assert_eq!(snapshot.protocol.artifacts, engine.protocol_artifacts());
        assert_eq!(snapshot.runtime.artifacts, engine.runtime_round_artifacts());
        assert!(snapshot.commitments.entries.is_empty());
    }

    #[test]
    fn replay_snapshot_runtime_surface_stays_bounded() {
        let engine = FieldEngine::new(node(1), (), ());
        for index in 0..(FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX + 4) {
            engine.record_runtime_round_artifact(FieldRuntimeRoundArtifact {
                protocol: crate::choreography::FieldProtocolKind::SummaryDissemination,
                destination: None,
                blocked_receive: None,
                disposition: crate::choreography::FieldRoundDisposition::Continue,
                host_wait_status: crate::choreography::FieldHostWaitStatus::Idle,
                emitted_count: index,
                step_budget_remaining: 1,
                execution_policy: crate::choreography::FieldExecutionPolicyClass::Cheap,
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
}
