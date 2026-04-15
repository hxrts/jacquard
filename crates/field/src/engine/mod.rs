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

mod replay;
mod surface;

pub use replay::*;

use std::{cell::RefCell, collections::VecDeque};

use jacquard_core::{
    ConnectivityPosture, NodeId, RouteId, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteShapeVisibility, RoutingEngineCapabilities, RoutingEngineId,
};

use crate::{
    choreography::FieldProtocolRuntime,
    policy::FieldPolicy,
    route::ActiveFieldRoute,
    search::{FieldPlannerSearchRecord, FieldSearchConfig, FieldSearchSnapshotState},
    state::FieldEngineState,
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
pub const FIELD_POLICY_EVENT_RETENTION_MAX: usize = 32;
pub const FIELD_REPLAY_SURFACE_VERSION: u16 = 1;

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
    pub(crate) policy_events: RefCell<VecDeque<FieldPolicyEvent>>,
    pub(crate) protocol_runtime: FieldProtocolRuntime,
    pub(crate) active_routes: std::collections::BTreeMap<RouteId, ActiveFieldRoute>,
    pub(crate) policy: FieldPolicy,
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
            policy_events: RefCell::new(VecDeque::new()),
            protocol_runtime: FieldProtocolRuntime::default(),
            active_routes: std::collections::BTreeMap::new(),
            policy: FieldPolicy::default(),
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
    pub(crate) fn policy(&self) -> &FieldPolicy {
        &self.policy
    }

    #[must_use]
    #[expect(
        dead_code,
        reason = "phase-5 experimental surface; profile wiring will consume this policy hook"
    )]
    pub(crate) fn with_policy(mut self, policy: FieldPolicy) -> Self {
        self.policy = policy;
        self
    }
}
