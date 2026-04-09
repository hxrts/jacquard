//! Transport protocols, link endpoints, service descriptors, and connectivity
//! surfaces.

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ByteCount, ClusterId, ControllerId, DiscoveryScopeId, GatewayId, HomeId,
    NodeId, RatioPermille, RoutingEngineId, Tick, TimeWindow,
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
pub enum TransportKind {
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
/// Shared endpoint-locator vocabulary for observed carriers.
///
/// The shared shape stays transport-neutral. Transport-specific crates may map
/// their concrete endpoint identities into one of these locator forms, but
/// `jacquard-core` does not own those transport-specific constructors.
pub enum EndpointLocator {
    Socket { host: String, port: u16 },
    ScopedBytes { scope: String, bytes: Vec<u8> },
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
/// Stable retransmit or retry semantics for a link.
pub enum RepairCapability {
    None,
    TransportRetransmit,
    ApplicationRetransmit,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Stable class of delivery recovery a link can support after disruption.
pub enum PartitionRecoveryClass {
    None,
    LocalReconnect,
    EndToEndRecoverable,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LinkEndpoint {
    pub transport_kind: TransportKind,
    pub locator: EndpointLocator,
    /// Link endpoints are frame carriers only. Ordering and traffic control
    /// live above this layer in routing and protocol logic.
    pub mtu_bytes: ByteCount,
}

impl LinkEndpoint {
    #[must_use]
    pub fn new(
        transport_kind: TransportKind,
        locator: EndpointLocator,
        mtu_bytes: ByteCount,
    ) -> Self {
        Self { transport_kind, locator, mtu_bytes }
    }
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

impl CapacityHint {
    #[must_use]
    pub fn new(saturation_permille: RatioPermille) -> Self {
        Self {
            saturation_permille,
            repair_capacity_slots: Belief::Absent,
            hold_capacity_bytes: Belief::Absent,
        }
    }

    #[must_use]
    pub fn with_repair_capacity_slots(
        mut self,
        repair_capacity_slots: RepairCapacitySlots,
        updated_at_tick: Tick,
    ) -> Self {
        self.repair_capacity_slots =
            Belief::certain(repair_capacity_slots, updated_at_tick);
        self
    }

    #[must_use]
    pub fn with_hold_capacity_bytes(
        mut self,
        hold_capacity_bytes: ByteCount,
        updated_at_tick: Tick,
    ) -> Self {
        self.hold_capacity_bytes =
            Belief::certain(hold_capacity_bytes, updated_at_tick);
        self
    }
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
