//! World and configuration primitives for routing.

use std::collections::BTreeMap;

use contour_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ByteCount, ControllerId, DurationMs, InformationSetSummary, LinkEndpoint,
    LinkRuntimeState, NodeId, NodeRelayBudget, RatioPermille, RouteEpoch, ServiceDescriptor,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Stable node capability and service surface in the routing world model.
pub struct NodeProfile {
    pub services: Vec<ServiceDescriptor>,
    pub endpoints: Vec<LinkEndpoint>,
    pub connection_count_max: u32,
    pub neighbor_state_count_max: u32,
    pub simultaneous_transfer_count_max: u32,
    pub active_route_count_max: u32,
    pub relay_work_budget_max: u32,
    pub maintenance_work_budget_max: u32,
    pub hold_item_count_max: u32,
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
/// Stable link capability and endpoint surface in the routing world model.
pub struct LinkProfile {
    pub endpoint: LinkEndpoint,
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
pub struct Link {
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
/// Wired-together routing configuration. In practice this is often a partial local view.
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
    use crate::{NodeId, Tick};

    #[test]
    fn configuration_has_deterministic_node_key_order() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeId([2; 32]),
            Node {
                controller_id: ControllerId([9; 32]),
                profile: NodeProfile {
                    services: Vec::new(),
                    endpoints: Vec::new(),
                    connection_count_max: 0,
                    neighbor_state_count_max: 0,
                    simultaneous_transfer_count_max: 0,
                    active_route_count_max: 0,
                    relay_work_budget_max: 0,
                    maintenance_work_budget_max: 0,
                    hold_item_count_max: 0,
                    hold_capacity_bytes_max: ByteCount(0),
                },
                state: NodeState {
                    relay_budget: Belief::Absent,
                    available_connection_count: Belief::Absent,
                    hold_capacity_available_bytes: Belief::Absent,
                    information_summary: Belief::Absent,
                },
            },
        );
        nodes.insert(
            NodeId([1; 32]),
            Node {
                controller_id: ControllerId([8; 32]),
                profile: NodeProfile {
                    services: Vec::new(),
                    endpoints: Vec::new(),
                    connection_count_max: 0,
                    neighbor_state_count_max: 0,
                    simultaneous_transfer_count_max: 0,
                    active_route_count_max: 0,
                    relay_work_budget_max: 0,
                    maintenance_work_budget_max: 0,
                    hold_item_count_max: 0,
                    hold_capacity_bytes_max: ByteCount(0),
                },
                state: NodeState {
                    relay_budget: Belief::Absent,
                    available_connection_count: Belief::Absent,
                    hold_capacity_available_bytes: Belief::Absent,
                    information_summary: Belief::Absent,
                },
            },
        );

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
        let _ = Tick(0);
    }
}
