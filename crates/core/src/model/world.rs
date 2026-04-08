//! World and configuration primitives for routing.

use std::collections::BTreeMap;

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ByteCount, ControllerId, DurationMs, HoldItemCount, InformationSetSummary,
    LinkEndpoint, LinkRuntimeState, MaintenanceWorkBudget, NodeId, NodeRelayBudget,
    PartitionRecoveryClass, RatioPermille, RelayWorkBudget, RepairCapability,
    RouteEpoch, ServiceDescriptor,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Stable node capability and service surface in the routing world model.
pub struct NodeProfile {
    pub services: Vec<ServiceDescriptor>,
    /// Bounded by
    /// [`SERVICE_ENDPOINT_COUNT_MAX`](crate::SERVICE_ENDPOINT_COUNT_MAX).
    pub endpoints: Vec<LinkEndpoint>,
    pub connection_count_max: u32,
    pub neighbor_state_count_max: u32,
    pub simultaneous_transfer_count_max: u32,
    pub active_route_count_max: u32,
    pub relay_work_budget_max: RelayWorkBudget,
    pub maintenance_work_budget_max: MaintenanceWorkBudget,
    pub hold_item_count_max: HoldItemCount,
    pub hold_capacity_bytes_max: ByteCount,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Current node state in the routing world model.
pub struct NodeState {
    pub relay_budget: Belief<NodeRelayBudget>,
    pub available_connection_count: Belief<u32>,
    pub hold_capacity_available_bytes: Belief<ByteCount>,
    pub information_summary: Belief<InformationSetSummary>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Instantiated node object in the routing world model.
pub struct Node {
    pub controller_id: ControllerId,
    pub profile: NodeProfile,
    pub state: NodeState,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Stable link capability surface in the routing world model.
pub struct LinkProfile {
    pub latency_floor_ms: DurationMs,
    pub repair_capability: RepairCapability,
    pub partition_recovery: PartitionRecoveryClass,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Current link state in the routing world model.
pub struct LinkState {
    pub state: LinkRuntimeState,
    pub median_rtt_ms: DurationMs,
    pub transfer_rate_bytes_per_sec: Belief<u32>,
    pub stability_horizon_ms: Belief<DurationMs>,
    pub loss_permille: RatioPermille,
    pub delivery_confidence_permille: Belief<RatioPermille>,
    pub symmetry_permille: Belief<RatioPermille>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Instantiated link object in the routing world model.
///
/// A link represents a directed connection from one node to another. The
/// link's `endpoint` identifies the remote address and protocol. The
/// `profile` captures stable routing-relevant capability such as latency floor
/// and repair class. The `state` captures current runtime observations such as
/// latency, loss, and delivery confidence.
pub struct Link {
    pub endpoint: LinkEndpoint,
    pub profile: LinkProfile,
    pub state: LinkState,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Instantiated local environment object in the routing world model.
pub struct Environment {
    pub reachable_neighbor_count: u32,
    pub churn_permille: RatioPermille,
    pub contention_permille: RatioPermille,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Wired-together routing configuration. In practice this is often a partial
/// local view.
pub struct Configuration {
    pub epoch: RouteEpoch,
    pub nodes: BTreeMap<NodeId, Node>,
    pub links: BTreeMap<(NodeId, NodeId), Link>,
    pub environment: Environment,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::NodeId;

    fn empty_node(controller_byte: u8) -> Node {
        Node {
            controller_id: ControllerId([controller_byte; 32]),
            profile: NodeProfile {
                services: Vec::new(),
                endpoints: Vec::new(),
                connection_count_max: 0,
                neighbor_state_count_max: 0,
                simultaneous_transfer_count_max: 0,
                active_route_count_max: 0,
                relay_work_budget_max: RelayWorkBudget(0),
                maintenance_work_budget_max: MaintenanceWorkBudget(0),
                hold_item_count_max: HoldItemCount(0),
                hold_capacity_bytes_max: ByteCount(0),
            },
            state: NodeState {
                relay_budget: Belief::Absent,
                available_connection_count: Belief::Absent,
                hold_capacity_available_bytes: Belief::Absent,
                information_summary: Belief::Absent,
            },
        }
    }

    #[test]
    fn configuration_has_deterministic_node_key_order() {
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId([2; 32]), empty_node(9));
        nodes.insert(NodeId([1; 32]), empty_node(8));

        let configuration = Configuration {
            epoch: RouteEpoch(1),
            nodes,
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        };

        let keys: Vec<_> = configuration.nodes.keys().copied().collect();
        assert_eq!(keys, vec![NodeId([1; 32]), NodeId([2; 32])]);
    }

    #[test]
    fn link_preserves_profile_and_state_split() {
        let link = Link {
            endpoint: LinkEndpoint {
                protocol: crate::TransportProtocol::BleGatt,
                address: crate::EndpointAddress::Opaque(vec![1, 2, 3]),
                mtu_bytes: ByteCount(128),
            },
            profile: LinkProfile {
                latency_floor_ms: DurationMs(8),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: DurationMs(20),
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(10),
                delivery_confidence_permille: Belief::Absent,
                symmetry_permille: Belief::Absent,
            },
        };

        assert_eq!(link.profile.latency_floor_ms, DurationMs(8));
        assert_eq!(link.state.median_rtt_ms, DurationMs(20));
    }
}
