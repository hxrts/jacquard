use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ControllerId, LinkEndpoint, NodeId, RouteEpoch, ServiceDescriptor, Tick,
    TopologyLinkObservation,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySnapshot {
    pub epoch: RouteEpoch,
    pub nodes: BTreeMap<NodeId, TopologyNodeObservation>,
    pub links: BTreeMap<(NodeId, NodeId), TopologyLinkObservation>,
    pub last_updated_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyNodeObservation {
    pub controller_id: ControllerId,
    pub services: Vec<ServiceDescriptor>,
    pub endpoints: Vec<LinkEndpoint>,
    pub trust_class: PeerTrustClass,
    pub last_seen_at: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutingEvidenceClass {
    Observed,
    ControllerAuthenticated,
    AdmissionWitnessed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PeerTrustClass {
    LocalOwned,
    ControllerBound,
    UnauthenticatedObserved,
    LowTrustRelay,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingFact<T> {
    pub value: T,
    pub evidence_class: RoutingEvidenceClass,
    pub trust_class: PeerTrustClass,
    pub observed_at: Tick,
}
