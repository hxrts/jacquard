use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, ConnectivityPosture, NodeId, Observation, RouteId,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteShapeVisibility,
    RoutingEngineCapabilities, RoutingEngineId,
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

pub struct FieldEngine<Transport, Effects> {
    pub(crate) local_node_id: NodeId,
    #[expect(
        dead_code,
        reason = "phase-2 scaffold; forwarding uses transport in later phases"
    )]
    pub(crate) transport: Transport,
    #[expect(
        dead_code,
        reason = "phase-2 scaffold; observer/control updates use effects in later phases"
    )]
    pub(crate) effects: Effects,
    pub(crate) latest_topology: Option<Observation<Configuration>>,
    pub(crate) active_routes: BTreeMap<RouteId, NodeId>,
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    #[must_use]
    pub fn new(local_node_id: NodeId, transport: Transport, effects: Effects) -> Self {
        Self {
            local_node_id,
            transport,
            effects,
            latest_topology: None,
            active_routes: BTreeMap::new(),
        }
    }
}
