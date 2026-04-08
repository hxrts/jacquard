//! Transport protocols, link endpoints, service descriptors, and connectivity
//! surfaces.

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    Belief, BleDeviceId, BleProfileId, ByteCount, ClusterId, ControllerId,
    DiscoveryScopeId, GatewayId, HomeId, NetworkHost, NodeId, RatioPermille,
    RoutingEngineId, TimeWindow,
};

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RouteServiceKind {
    Discover,
    Activate,
    Move,
    Repair,
    Hold,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Shared transport taxonomy for observed links, services, and transport
/// observations.
///
/// Jacquard intentionally keeps a small amount of concrete transport
/// vocabulary in `core` because these variants appear in shared world facts
/// like `LinkEndpoint` and `ServiceDescriptor`. Adapter-specific metadata
/// should still stay out of `core`, and the opaque forms remain available for
/// transports that do not fit the built-in shapes.
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Shared endpoint-address vocabulary for observed carriers.
///
/// BLE remains modeled explicitly here because it is part of the current
/// shared world schema rather than a mesh-private adapter detail. Jacquard
/// intentionally does not force the address model fully opaque until a second
/// transport proves the current shared schema too specific.
pub enum EndpointAddress {
    Ble {
        device_id: BleDeviceId,
        profile_id: BleProfileId,
    },
    Ip {
        host: NetworkHost,
        port: u16,
    },
    Opaque(Vec<u8>),
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
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
    pub address: EndpointAddress,
    /// Link endpoints are frame carriers only. Ordering and traffic control
    /// live above this layer in routing and protocol logic.
    pub mtu_bytes: ByteCount,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// What a node advertises as a shared service surface.
/// Descriptors are shared facts. Local ranking is not published here.
pub struct ServiceDescriptor {
    pub provider_node_id: NodeId,
    pub controller_id: ControllerId,
    pub service_kind: RouteServiceKind,
    /// Bounded by
    /// [`SERVICE_ENDPOINT_COUNT_MAX`](crate::SERVICE_ENDPOINT_COUNT_MAX).
    pub endpoints: Vec<LinkEndpoint>,
    pub routing_engines: Vec<RoutingEngineId>,
    pub scope: ServiceScope,
    pub valid_for: TimeWindow,
    pub capacity: Belief<CapacityHint>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceScope {
    Discovery(DiscoveryScopeId),
    Home(HomeId),
    Cluster(ClusterId),
    Gateway(GatewayId),
    Introduction { scope_token: Vec<u8> },
}

#[id_type]
pub struct RepairCapacitySlots(pub u32);

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityHint {
    pub saturation_permille: RatioPermille,
    pub repair_capacity_slots: Belief<RepairCapacitySlots>,
    pub hold_capacity_bytes: Belief<ByteCount>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportObservation {
    PayloadReceived {
        from_node_id: NodeId,
        endpoint: LinkEndpoint,
        payload: Vec<u8>,
        observed_at_tick: crate::Tick,
    },
    LinkObserved {
        remote_node_id: NodeId,
        observation: crate::Observation<crate::Link>,
    },
}
