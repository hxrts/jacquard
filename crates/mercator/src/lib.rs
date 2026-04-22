//! Deterministic hybrid corridor routing engine skeleton for Jacquard.
//!
//! `Mercator` combines explicit objective search, maintained evidence,
//! corridor continuity, fail-closed stale repair, weakest-flow fairness, and
//! bounded custody. The first implementation phases keep the runtime
//! deterministic and router-owned while enabling bounded connected corridors.

// proc-macro-scope: Mercator's crate root exposes engine services, not shared model vocabulary.

#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::cell::{Cell, RefCell};

pub mod corridor;
mod corridor_token;
pub mod custody;
pub mod evidence;
mod planner;
mod public_state;
mod runtime;

pub use corridor::selected_neighbor_from_backend_route_id;
use corridor::{ActiveMercatorRoute, MercatorPlanningContext, MercatorPlanningOutcome};
use custody::MercatorCustodyRecord;
pub use evidence::{
    MercatorBrokerPressure, MercatorDiagnostics, MercatorEvidenceGraph, MercatorEvidenceMeta,
    MercatorObjectiveKey,
};
use jacquard_core::{
    Blake3Digest, Configuration, ConnectivityPosture, ContentId, DestinationId, NodeId,
    Observation, OrderStamp, RouteEpoch, RouteId, RoutePartitionClass, RouteProtectionClass,
    RouteShapeVisibility, RoutingEngineCapabilities, RoutingEngineId, RoutingObjective, Tick,
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
        repair: jacquard_core::RouteRepairClass::Repairable,
        partition: RoutePartitionClass::ConnectedOnly,
    },
    repair_support: jacquard_core::RepairSupport::Supported,
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
    latest_topology: Option<Observation<Configuration>>,
    evidence: MercatorEvidenceGraph,
    planner_diagnostics: Cell<MercatorDiagnostics>,
    objective_accounts: RefCell<BTreeMap<MercatorObjectiveKey, MercatorObjectiveAccount>>,
    route_objectives: BTreeMap<RouteId, MercatorObjectiveKey>,
    pub(crate) custody_records: BTreeMap<ContentId<Blake3Digest>, MercatorCustodyRecord>,
    active_routes: BTreeMap<RouteId, ActiveMercatorRoute>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MercatorObjectiveAccount {
    active_round_count: u32,
    materialization_count: u32,
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
            latest_topology: None,
            evidence: MercatorEvidenceGraph::new(config.evidence),
            planner_diagnostics: Cell::new(MercatorDiagnostics::default()),
            objective_accounts: RefCell::new(BTreeMap::new()),
            route_objectives: BTreeMap::new(),
            custody_records: BTreeMap::new(),
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
        let mut combined = Self::combined_route_diagnostics(evidence, planner);
        Self::add_route_pressure_diagnostics(&mut combined, evidence, planner);
        Self::add_custody_diagnostics(&mut combined, evidence, planner);
        combined
    }

    fn combined_route_diagnostics(
        evidence: MercatorDiagnostics,
        planner: MercatorDiagnostics,
    ) -> MercatorDiagnostics {
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
            active_stale_route_count: evidence
                .active_stale_route_count
                .saturating_add(planner.active_stale_route_count),
            repair_attempt_count: evidence
                .repair_attempt_count
                .saturating_add(planner.repair_attempt_count),
            repair_success_count: evidence
                .repair_success_count
                .saturating_add(planner.repair_success_count),
            recovery_rounds: evidence
                .recovery_rounds
                .saturating_add(planner.recovery_rounds),
            ..MercatorDiagnostics::default()
        }
    }

    fn add_route_pressure_diagnostics(
        combined: &mut MercatorDiagnostics,
        evidence: MercatorDiagnostics,
        planner: MercatorDiagnostics,
    ) {
        combined.objective_count = evidence
            .objective_count
            .saturating_add(planner.objective_count);
        combined.active_objective_count = evidence
            .active_objective_count
            .saturating_add(planner.active_objective_count);
        combined.weakest_objective_presence_rounds = evidence
            .weakest_objective_presence_rounds
            .saturating_add(planner.weakest_objective_presence_rounds);
        combined.zero_service_objective_count = evidence
            .zero_service_objective_count
            .saturating_add(planner.zero_service_objective_count);
        combined.broker_participation_count = evidence
            .broker_participation_count
            .saturating_add(planner.broker_participation_count);
        combined.hottest_broker_route_count = evidence
            .hottest_broker_route_count
            .saturating_add(planner.hottest_broker_route_count);
        combined.hottest_broker_concentration_permille = evidence
            .hottest_broker_concentration_permille
            .saturating_add(planner.hottest_broker_concentration_permille);
        combined.broker_switch_count = evidence
            .broker_switch_count
            .saturating_add(planner.broker_switch_count);
        combined.overloaded_broker_penalty_count = evidence
            .overloaded_broker_penalty_count
            .saturating_add(planner.overloaded_broker_penalty_count);
        combined.weakest_flow_reserved_search_count = evidence
            .weakest_flow_reserved_search_count
            .saturating_add(planner.weakest_flow_reserved_search_count);
    }

    fn add_custody_diagnostics(
        combined: &mut MercatorDiagnostics,
        evidence: MercatorDiagnostics,
        planner: MercatorDiagnostics,
    ) {
        combined.custody_record_count = evidence
            .custody_record_count
            .saturating_add(planner.custody_record_count);
        combined.custody_reproduction_count = evidence
            .custody_reproduction_count
            .saturating_add(planner.custody_reproduction_count);
        combined.custody_copy_budget_spent = evidence
            .custody_copy_budget_spent
            .saturating_add(planner.custody_copy_budget_spent);
        combined.custody_copy_budget_remaining = evidence
            .custody_copy_budget_remaining
            .saturating_add(planner.custody_copy_budget_remaining);
        combined.custody_protected_budget_spent = evidence
            .custody_protected_budget_spent
            .saturating_add(planner.custody_protected_budget_spent);
        combined.custody_protected_budget_remaining = evidence
            .custody_protected_budget_remaining
            .saturating_add(planner.custody_protected_budget_remaining);
        combined.custody_transmission_count = evidence
            .custody_transmission_count
            .saturating_add(planner.custody_transmission_count);
        combined.custody_storage_bytes = evidence
            .custody_storage_bytes
            .saturating_add(planner.custody_storage_bytes);
        combined.custody_energy_spent_units = evidence
            .custody_energy_spent_units
            .saturating_add(planner.custody_energy_spent_units);
        combined.custody_leakage_risk_permille = evidence
            .custody_leakage_risk_permille
            .saturating_add(planner.custody_leakage_risk_permille);
        combined.custody_suppressed_forward_count = evidence
            .custody_suppressed_forward_count
            .saturating_add(planner.custody_suppressed_forward_count);
        combined.custody_same_cluster_suppression_count = evidence
            .custody_same_cluster_suppression_count
            .saturating_add(planner.custody_same_cluster_suppression_count);
        combined.custody_low_gain_suppression_count = evidence
            .custody_low_gain_suppression_count
            .saturating_add(planner.custody_low_gain_suppression_count);
        combined.custody_bridge_opportunity_count = evidence
            .custody_bridge_opportunity_count
            .saturating_add(planner.custody_bridge_opportunity_count);
        combined.custody_protected_bridge_usage_count = evidence
            .custody_protected_bridge_usage_count
            .saturating_add(planner.custody_protected_bridge_usage_count);
    }

    pub(crate) fn planning_context_for(
        &self,
        objective: &RoutingObjective,
    ) -> MercatorPlanningContext {
        let key = self.register_objective_interest(objective.destination.clone());
        let active = self
            .route_objectives
            .values()
            .any(|objective| objective == &key);
        MercatorPlanningContext {
            reserve_for_underserved_objective: !active,
        }
    }

    pub(crate) fn record_weakest_flow_search_reservation(&self) {
        let mut diagnostics = self.planner_diagnostics.get();
        diagnostics.weakest_flow_reserved_search_count = diagnostics
            .weakest_flow_reserved_search_count
            .saturating_add(1);
        self.planner_diagnostics.set(diagnostics);
    }

    pub(crate) fn record_overloaded_broker_penalty(&self) {
        let mut diagnostics = self.planner_diagnostics.get();
        diagnostics.overloaded_broker_penalty_count = diagnostics
            .overloaded_broker_penalty_count
            .saturating_add(1);
        self.planner_diagnostics.set(diagnostics);
    }

    fn register_objective_interest(&self, destination: DestinationId) -> MercatorObjectiveKey {
        let key = MercatorObjectiveKey::destination(destination);
        self.objective_accounts
            .borrow_mut()
            .entry(key.clone())
            .or_default();
        key
    }

    fn record_route_objective_materialized(
        &mut self,
        route_id: RouteId,
        destination: DestinationId,
    ) {
        let key = self.register_objective_interest(destination);
        self.route_objectives.insert(route_id, key.clone());
        let mut accounts = self.objective_accounts.borrow_mut();
        let account = accounts.entry(key).or_default();
        account.materialization_count = account.materialization_count.saturating_add(1);
    }

    fn remove_route_objective(&mut self, route_id: &RouteId) {
        self.route_objectives.remove(route_id);
    }

    fn refresh_objective_presence_diagnostics(&mut self, count_active_round: bool) {
        let active_objectives = self
            .route_objectives
            .values()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut accounts = self.objective_accounts.borrow_mut();
        if count_active_round {
            for objective in &active_objectives {
                let account = accounts.entry(objective.clone()).or_default();
                account.active_round_count = account.active_round_count.saturating_add(1);
            }
        }
        let objective_count = u32::try_from(accounts.len()).unwrap_or(u32::MAX);
        let active_objective_count = u32::try_from(active_objectives.len()).unwrap_or(u32::MAX);
        let weakest = accounts
            .values()
            .map(|account| account.active_round_count)
            .min()
            .unwrap_or(0);
        let zero_service_count = u32::try_from(
            accounts
                .values()
                .filter(|account| account.active_round_count == 0)
                .count(),
        )
        .unwrap_or(u32::MAX);
        self.evidence.record_objective_presence(
            objective_count,
            active_objective_count,
            weakest,
            zero_service_count,
        );
    }

    fn refresh_broker_diagnostics(&mut self, topology_epoch: RouteEpoch, now: Tick) {
        let mut counts = BTreeMap::<NodeId, u32>::new();
        for active in self.active_routes.values() {
            for broker in broker_nodes_for_path(&active.primary_path) {
                counts
                    .entry(broker)
                    .and_modify(|count| *count = count.saturating_add(1))
                    .or_insert(1);
            }
        }
        let total = counts.values().copied().sum::<u32>();
        let hottest = counts.values().copied().max().unwrap_or(0);
        let concentration = if total == 0 {
            0
        } else {
            u16::try_from(hottest.saturating_mul(1_000) / total).unwrap_or(u16::MAX)
        };
        for (index, (broker, count)) in counts.iter().enumerate() {
            self.evidence
                .record_broker_pressure(MercatorBrokerPressure {
                    broker: *broker,
                    participation_count: *count,
                    pressure_score: broker_pressure_score(*count),
                    meta: MercatorEvidenceMeta::new(
                        topology_epoch,
                        now,
                        self.config.bounds.evidence_validity,
                        OrderStamp(u64::try_from(index).unwrap_or(u64::MAX)),
                    ),
                });
        }
        self.evidence
            .record_broker_concentration(total, hottest, concentration);
    }
}

pub(crate) fn broker_nodes_for_path(path: &[NodeId]) -> Vec<NodeId> {
    path.iter()
        .copied()
        .skip(1)
        .take(path.len().saturating_sub(2))
        .collect()
}

fn broker_pressure_score(route_count: u32) -> u16 {
    u16::try_from(route_count.saturating_mul(250).min(1_000)).unwrap_or(u16::MAX)
}
