//! Deterministic hybrid corridor routing engine skeleton for Jacquard.
//!
//! `Mercator` combines explicit objective search, maintained evidence,
//! corridor continuity, fail-closed stale repair, weakest-flow fairness, and
//! bounded custody. The first implementation phases keep the runtime
//! deterministic and router-owned while enabling bounded connected corridors.

#![forbid(unsafe_code)]

use std::{cell::Cell, collections::BTreeMap};

pub mod corridor;
pub mod evidence;
mod planner;
mod public_state;
mod runtime;

use corridor::{ActiveMercatorRoute, MercatorPlanningOutcome};
pub use evidence::{MercatorDiagnostics, MercatorEvidenceGraph};
use jacquard_core::{
    ConnectivityPosture, NodeId, RouteEpoch, RouteId, RoutePartitionClass, RouteProtectionClass,
    RouteShapeVisibility, RoutingEngineCapabilities, RoutingEngineId,
};
pub use public_state::{
    MercatorEngineConfig, MercatorEvidenceBounds, MercatorOperationalBounds,
    MercatorRouterAnalysisSnapshot,
};

pub const MERCATOR_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.mercatr");

pub const MERCATOR_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: MERCATOR_ENGINE_ID,
    max_protection: RouteProtectionClass::LinkProtected,
    max_connectivity: ConnectivityPosture {
        repair: jacquard_core::RouteRepairClass::BestEffort,
        partition: RoutePartitionClass::ConnectedOnly,
    },
    repair_support: jacquard_core::RepairSupport::Unsupported,
    hold_support: jacquard_core::HoldSupport::Unsupported,
    decidable_admission: jacquard_core::DecidableSupport::Supported,
    quantitative_bounds: jacquard_core::QuantitativeBoundSupport::Unsupported,
    reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
    route_shape_visibility: RouteShapeVisibility::CorridorEnvelope,
};

#[derive(Clone, Debug)]
pub struct MercatorEngine {
    local_node_id: NodeId,
    config: MercatorEngineConfig,
    latest_topology_epoch: Option<RouteEpoch>,
    evidence: MercatorEvidenceGraph,
    planner_diagnostics: Cell<MercatorDiagnostics>,
    active_routes: BTreeMap<RouteId, ActiveMercatorRoute>,
}

impl MercatorEngine {
    #[must_use]
    pub fn new(local_node_id: NodeId) -> Self {
        Self::with_config(local_node_id, MercatorEngineConfig::default())
    }

    #[must_use]
    pub fn with_config(local_node_id: NodeId, config: MercatorEngineConfig) -> Self {
        Self {
            local_node_id,
            config,
            latest_topology_epoch: None,
            evidence: MercatorEvidenceGraph::new(config.evidence),
            planner_diagnostics: Cell::new(MercatorDiagnostics::default()),
            active_routes: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn local_node_id(&self) -> NodeId {
        self.local_node_id
    }

    #[must_use]
    pub fn config(&self) -> MercatorEngineConfig {
        self.config
    }

    #[must_use]
    pub fn latest_topology_epoch(&self) -> Option<RouteEpoch> {
        self.latest_topology_epoch
    }

    #[must_use]
    pub fn evidence(&self) -> &MercatorEvidenceGraph {
        &self.evidence
    }

    pub fn evidence_mut(&mut self) -> &mut MercatorEvidenceGraph {
        &mut self.evidence
    }

    #[must_use]
    pub fn diagnostics(&self) -> MercatorDiagnostics {
        self.combined_diagnostics()
    }

    #[must_use]
    pub fn active_route_count(&self) -> usize {
        self.active_routes.len()
    }

    #[must_use]
    pub fn router_analysis_snapshot(&self) -> MercatorRouterAnalysisSnapshot {
        MercatorRouterAnalysisSnapshot {
            diagnostics: self.combined_diagnostics(),
            active_route_count: u32::try_from(self.active_routes.len()).unwrap_or(u32::MAX),
            latest_topology_epoch: self.latest_topology_epoch,
        }
    }

    pub(crate) fn record_planning_outcome(&self, outcome: &MercatorPlanningOutcome) {
        let mut diagnostics = self.planner_diagnostics.get();
        match outcome {
            MercatorPlanningOutcome::Selected(_) => {
                diagnostics.selected_result_rounds =
                    diagnostics.selected_result_rounds.saturating_add(1);
            }
            MercatorPlanningOutcome::NoCandidate => {
                diagnostics.no_candidate_attempts =
                    diagnostics.no_candidate_attempts.saturating_add(1);
            }
            MercatorPlanningOutcome::Inadmissible => {
                diagnostics.inadmissible_candidate_attempts = diagnostics
                    .inadmissible_candidate_attempts
                    .saturating_add(1);
            }
        }
        self.planner_diagnostics.set(diagnostics);
    }

    fn combined_diagnostics(&self) -> MercatorDiagnostics {
        let evidence = self.evidence.diagnostics();
        let planner = self.planner_diagnostics.get();
        MercatorDiagnostics {
            selected_result_rounds: evidence
                .selected_result_rounds
                .saturating_add(planner.selected_result_rounds),
            no_candidate_attempts: evidence
                .no_candidate_attempts
                .saturating_add(planner.no_candidate_attempts),
            inadmissible_candidate_attempts: evidence
                .inadmissible_candidate_attempts
                .saturating_add(planner.inadmissible_candidate_attempts),
            support_withdrawal_count: evidence
                .support_withdrawal_count
                .saturating_add(planner.support_withdrawal_count),
            stale_persistence_rounds: evidence
                .stale_persistence_rounds
                .saturating_add(planner.stale_persistence_rounds),
        }
    }
}
