//! Transport protocols, link endpoints, service descriptors, and connectivity surfaces.

use serde::{Deserialize, Serialize};

use crate::{
    ClusterId, ControllerId, DurationMs, GatewayDomainId, HomeId, NeighborhoodId, NodeId,
    RatioPermille, Tick, TimeWindow,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Identifies a routing family. Mesh is first-party; External covers third-party plugins.
pub enum RoutingFamilyId {
    Mesh,
    External { name: String, contract_id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ServiceFamily {
    Discover,
    Establish,
    Move,
    Repair,
    Hold,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransportProtocol {
    BleGatt,
    BleL2cap,
    WifiAware,
    WifiLan,
    Quic,
    TcpRelay,
    Custom(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransportClass {
    Proximity,
    LocalArea,
    Backbone,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EndpointAddress {
    Ble {
        device_id: Vec<u8>,
        service_uuid: [u8; 16],
    },
    Ip {
        host: String,
        port: u16,
    },
    Opaque(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliverySemantics {
    UnorderedBestEffort,
    ReliableOrdered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LinkRuntimeState {
    Active,
    Degraded,
    Suspended,
    Faulted,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LinkEndpoint {
    pub protocol: TransportProtocol,
    pub class: TransportClass,
    pub address: EndpointAddress,
    pub mtu_bytes: u32,
    pub delivery_semantics: DeliverySemantics,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// What a node advertises as a shared service surface.
/// Descriptors are shared facts. Local ranking is not published here.
pub struct ServiceDescriptor {
    pub provider_node_id: NodeId,
    pub controller_id: ControllerId,
    pub family: ServiceFamily,
    pub endpoints: Vec<LinkEndpoint>,
    pub routing_families: Vec<RoutingFamilyId>,
    pub scope: ServiceScope,
    pub valid_for: TimeWindow,
    pub capacity: Option<CapacityHint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceScope {
    Neighborhood(NeighborhoodId),
    Home(HomeId),
    Cluster(ClusterId),
    GatewayDomain(GatewayDomainId),
    Introduction { scope_token: Vec<u8> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityHint {
    pub saturation_permille: RatioPermille,
    pub repair_capacity: Option<u32>,
    pub hold_capacity_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyLinkObservation {
    pub endpoint: LinkEndpoint,
    pub state: LinkRuntimeState,
    pub median_rtt: DurationMs,
    pub loss_permille: RatioPermille,
    pub last_seen_at: Tick,
}
