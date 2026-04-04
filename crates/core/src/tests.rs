use std::collections::BTreeMap;

use crate::{
    ControllerId, NodeId, OrderStamp, PeerTrustClass, PriorityPoints, RouteEpoch, RouteOrderingKey,
    Tick, TopologyNodeObservation,
};

#[test]
fn route_ordering_key_is_total() {
    let low = RouteOrderingKey {
        priority: PriorityPoints(1),
        topology_epoch: RouteEpoch(2),
        tie_break: OrderStamp(3),
    };
    let high = RouteOrderingKey {
        priority: PriorityPoints(2),
        topology_epoch: RouteEpoch(2),
        tie_break: OrderStamp(3),
    };

    assert!(low < high);
}

#[test]
fn topology_snapshot_has_deterministic_key_order() {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        NodeId([2; 32]),
        TopologyNodeObservation {
            controller_id: ControllerId([9; 32]),
            services: Vec::new(),
            endpoints: Vec::new(),
            trust_class: PeerTrustClass::ControllerBound,
            last_seen_at: Tick(2),
        },
    );
    nodes.insert(
        NodeId([1; 32]),
        TopologyNodeObservation {
            controller_id: ControllerId([8; 32]),
            services: Vec::new(),
            endpoints: Vec::new(),
            trust_class: PeerTrustClass::ControllerBound,
            last_seen_at: Tick(1),
        },
    );

    let keys: Vec<_> = nodes.keys().copied().collect();
    assert_eq!(keys, vec![NodeId([1; 32]), NodeId([2; 32])]);
}
