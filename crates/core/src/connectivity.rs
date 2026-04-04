//! Transport protocols, link endpoints, service descriptors, and connectivity surfaces.

use contour_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    ClusterId, ControllerId, DurationMs, GatewayDomainId, HomeId, KnownValue, NeighborhoodId,
    NodeId, RatioPermille, Tick, TimeWindow,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Identifies a routing family. Mesh is first-party; External covers third-party plugins.
pub enum RoutingFamilyId {
    Mesh,
    External { name: String, contract_id: String },
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ServiceFamily {
    Discover,
    Establish,
    Move,
    Repair,
    Hold,
}

#[public_model]
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

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransportClass {
    Proximity,
    LocalArea,
    Backbone,
}

#[public_model]
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

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliverySemantics {
    UnorderedBestEffort,
    ReliableOrdered,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LinkRuntimeState {
    Active,
    Degraded,
    Suspended,
    Faulted,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LinkEndpoint {
    pub protocol: TransportProtocol,
    pub class: TransportClass,
    pub address: EndpointAddress,
    pub mtu_bytes: u32,
    pub delivery_semantics: DeliverySemantics,
}

#[public_model]
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
    pub capacity: KnownValue<CapacityHint>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceScope {
    Neighborhood(NeighborhoodId),
    Home(HomeId),
    Cluster(ClusterId),
    GatewayDomain(GatewayDomainId),
    Introduction { scope_token: Vec<u8> },
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityHint {
    pub saturation_permille: RatioPermille,
    pub repair_capacity: KnownValue<u32>,
    pub hold_capacity_bytes: KnownValue<u64>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyLinkObservation {
    pub endpoint: LinkEndpoint,
    pub state: LinkRuntimeState,
    pub median_rtt_ms: DurationMs,
    pub loss_permille: RatioPermille,
    pub last_seen_at_tick: Tick,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportIngressEvent {
    PayloadReceived {
        from_node_id: NodeId,
        endpoint: LinkEndpoint,
        payload_bytes: Vec<u8>,
        observed_at_tick: Tick,
    },
    LinkStateObserved {
        remote_node_id: NodeId,
        observation: TopologyLinkObservation,
    },
}
