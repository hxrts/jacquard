//! Topology snapshots, node observations, trust classes, and evidence-tagged facts.

use std::collections::BTreeMap;

use contour_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    ControllerId, KnownValue, LinkEndpoint, NodeId, PeerRoutingEstimate, RouteEpoch,
    ServiceDescriptor, Tick, TopologyLinkObservation,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Local view of the neighborhood graph. BTreeMap ensures deterministic iteration order.
pub struct TopologySnapshot {
    pub epoch: RouteEpoch,
    pub nodes: BTreeMap<NodeId, TopologyNodeObservation>,
    pub links: BTreeMap<(NodeId, NodeId), TopologyLinkObservation>,
    pub last_updated_at_tick: Tick,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyNodeObservation {
    pub controller_id: ControllerId,
    pub services: Vec<ServiceDescriptor>,
    pub endpoints: Vec<LinkEndpoint>,
    pub routing_estimate: KnownValue<PeerRoutingEstimate>,
    pub trust_class: PeerTrustClass,
    pub last_seen_at_tick: Tick,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutingEvidenceClass {
    Observed,
    ControllerAuthenticated,
    AdmissionWitnessed,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PeerTrustClass {
    LocalOwned,
    ControllerBound,
    UnauthenticatedObserved,
    LowTrustRelay,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// A value tagged with its evidence provenance. Observed facts must not
/// silently become authoritative routing truth.
pub struct RoutingFact<T> {
    pub value: T,
    pub evidence_class: RoutingEvidenceClass,
    pub trust_class: PeerTrustClass,
    pub observed_at_tick: Tick,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Wrapper for values that are observational rather than canonical.
pub struct Observed<T> {
    pub fact: RoutingFact<T>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Wrapper for values that have been authoritatively published.
pub struct Authoritative<T> {
    pub value: T,
    pub published_at_tick: Tick,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn topology_snapshot_has_deterministic_key_order() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeId([2; 32]),
            TopologyNodeObservation {
                controller_id: ControllerId([9; 32]),
                services: Vec::new(),
                endpoints: Vec::new(),
                routing_estimate: KnownValue::Unknown,
                trust_class: PeerTrustClass::ControllerBound,
                last_seen_at_tick: Tick(2),
            },
        );
        nodes.insert(
            NodeId([1; 32]),
            TopologyNodeObservation {
                controller_id: ControllerId([8; 32]),
                services: Vec::new(),
                endpoints: Vec::new(),
                routing_estimate: KnownValue::Unknown,
                trust_class: PeerTrustClass::ControllerBound,
                last_seen_at_tick: Tick(1),
            },
        );

        let keys: Vec<_> = nodes.keys().copied().collect();
        assert_eq!(keys, vec![NodeId([1; 32]), NodeId([2; 32])]);
    }
}
