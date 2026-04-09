//! Transport protocols, link endpoints, service descriptors, and connectivity
//! surfaces for the shared routing world model.
//!
//! This module defines the engine-neutral vocabulary for what a node can reach
//! and what it advertises. Key types include [`TransportKind`] (the protocol
//! taxonomy), [`EndpointLocator`] (the shared address form), [`LinkEndpoint`]
//! (the carrier identity a link uses), [`ServiceDescriptor`] (what a node
//! advertises as a service surface), and [`TransportIngressEvent`] /
//! [`TransportObservation`] (the raw ingress events that the host bridge
//! stamps with Jacquard time before engines consume them).
//!
//! Transport-specific endpoint construction and metadata belong in
//! transport-owned profile crates, not here. `jacquard-core` owns only the
//! shared structural shapes that all engines and routers work against.

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ByteCount, ClusterId, ControllerId, DiscoveryScopeId, FactSourceClass,
    GatewayId, HomeId, Link, NodeId, OriginAuthenticationClass, RatioPermille,
    RoutingEngineId, RoutingEvidenceClass, Tick, TimeWindow,
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
/// Raw host-driver ingress event before the host bridge attaches Jacquard time.
///
/// Drivers own wall-clock I/O and stream supervision, but they do not assign
/// Jacquard `Tick` values internally.
pub enum TransportIngressEvent {
    PayloadReceived {
        from_node_id: NodeId,
        endpoint: LinkEndpoint,
        payload: Vec<u8>,
    },
    LinkObserved {
        remote_node_id: NodeId,
        link: Link,
        source_class: FactSourceClass,
        evidence_class: RoutingEvidenceClass,
        origin_authentication: OriginAuthenticationClass,
    },
}

impl TransportIngressEvent {
    #[must_use]
    pub fn observe_at(self, observed_at_tick: crate::Tick) -> TransportObservation {
        match self {
            | Self::PayloadReceived { from_node_id, endpoint, payload } => {
                TransportObservation::PayloadReceived {
                    from_node_id,
                    endpoint,
                    payload,
                    observed_at_tick,
                }
            },
            | Self::LinkObserved {
                remote_node_id,
                link,
                source_class,
                evidence_class,
                origin_authentication,
            } => TransportObservation::LinkObserved {
                remote_node_id,
                observation: crate::Observation {
                    value: link,
                    source_class,
                    evidence_class,
                    origin_authentication,
                    observed_at_tick,
                },
            },
        }
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
