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

use alloc::{string::String, vec::Vec};

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    Belief, BroadcastDomainId, ByteCount, ClusterId, ControllerId, DiscoveryScopeId,
    FactSourceClass, GatewayId, HomeId, Link, MulticastGroupId, NodeId, OriginAuthenticationClass,
    RatioPermille, RouteAdmissionRejection, RoutingEngineId, RoutingEvidenceClass, Tick,
    TimeWindow,
};

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
        Self {
            transport_kind,
            locator,
            mtu_bytes,
        }
    }
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransportDeliveryMode {
    Unicast,
    Multicast,
    Broadcast,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliveryCoverageObjective {
    AnyReceiver,
    AllReceivers,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReverseDeliveryConfirmation {
    Unconfirmed,
    Confirmed,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportDeliveryIntent {
    Unicast {
        endpoint: LinkEndpoint,
    },
    Multicast {
        endpoint: LinkEndpoint,
        group_id: MulticastGroupId,
        receivers: Vec<NodeId>,
    },
    Broadcast {
        endpoint: LinkEndpoint,
        domain_id: BroadcastDomainId,
    },
}

impl TransportDeliveryIntent {
    #[must_use]
    pub fn unicast(endpoint: LinkEndpoint) -> Self {
        Self::Unicast { endpoint }
    }

    #[must_use]
    pub fn mode(&self) -> TransportDeliveryMode {
        match self {
            Self::Unicast { .. } => TransportDeliveryMode::Unicast,
            Self::Multicast { .. } => TransportDeliveryMode::Multicast,
            Self::Broadcast { .. } => TransportDeliveryMode::Broadcast,
        }
    }

    #[must_use]
    pub fn endpoint(&self) -> &LinkEndpoint {
        match self {
            Self::Unicast { endpoint }
            | Self::Multicast { endpoint, .. }
            | Self::Broadcast { endpoint, .. } => endpoint,
        }
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportDeliverySupport {
    IsolatedUnicast {
        endpoint: LinkEndpoint,
        receiver: NodeId,
    },
    Multicast {
        endpoint: LinkEndpoint,
        group_id: MulticastGroupId,
        receivers: Vec<NodeId>,
    },
    Broadcast {
        endpoint: LinkEndpoint,
        domain_id: BroadcastDomainId,
        receivers: Vec<NodeId>,
        reverse_confirmation: ReverseDeliveryConfirmation,
    },
}

impl TransportDeliverySupport {
    #[must_use]
    pub fn mode(&self) -> TransportDeliveryMode {
        match self {
            Self::IsolatedUnicast { .. } => TransportDeliveryMode::Unicast,
            Self::Multicast { .. } => TransportDeliveryMode::Multicast,
            Self::Broadcast { .. } => TransportDeliveryMode::Broadcast,
        }
    }

    #[must_use]
    pub fn endpoint(&self) -> &LinkEndpoint {
        match self {
            Self::IsolatedUnicast { endpoint, .. }
            | Self::Multicast { endpoint, .. }
            | Self::Broadcast { endpoint, .. } => endpoint,
        }
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteDeliveryObjective {
    UnicastNode {
        node_id: NodeId,
    },
    MulticastGroup {
        group_id: MulticastGroupId,
        receivers: Vec<NodeId>,
    },
    BroadcastDomain {
        domain_id: BroadcastDomainId,
        receivers: Vec<NodeId>,
        coverage: DeliveryCoverageObjective,
    },
}

impl RouteDeliveryObjective {
    #[must_use]
    pub fn unicast_node(node_id: NodeId) -> Self {
        Self::UnicastNode { node_id }
    }

    #[must_use]
    pub fn broadcast_domain(
        domain_id: BroadcastDomainId,
        receivers: impl IntoIterator<Item = NodeId>,
        coverage: DeliveryCoverageObjective,
    ) -> Self {
        Self::BroadcastDomain {
            domain_id,
            receivers: receivers.into_iter().collect(),
            coverage,
        }
    }

    #[must_use]
    pub fn mode(&self) -> TransportDeliveryMode {
        match self {
            Self::UnicastNode { .. } => TransportDeliveryMode::Unicast,
            Self::MulticastGroup { .. } => TransportDeliveryMode::Multicast,
            Self::BroadcastDomain { .. } => TransportDeliveryMode::Broadcast,
        }
    }

    #[must_use]
    pub fn compatible_with(
        &self,
        support: &TransportDeliverySupport,
        policy: DeliveryCompatibilityPolicy,
    ) -> DeliveryCompatibility {
        delivery_compatibility(self, support, policy)
    }
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryCompatibilityPolicy {
    ExactDeliveryOnly,
    AllowRepeatedUnicast,
    AllowLossyBroadcast,
}

impl DeliveryCompatibilityPolicy {
    #[must_use]
    pub fn allows_repeated_unicast(self) -> bool {
        matches!(self, Self::AllowRepeatedUnicast)
    }

    #[must_use]
    pub fn allows_lossy_broadcast(self) -> bool {
        matches!(self, Self::AllowLossyBroadcast)
    }
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliveryCompatibility {
    Compatible,
    Rejected(RouteAdmissionRejection),
}

#[must_use]
pub fn delivery_compatibility(
    objective: &RouteDeliveryObjective,
    support: &TransportDeliverySupport,
    policy: DeliveryCompatibilityPolicy,
) -> DeliveryCompatibility {
    if delivery_supports_objective(objective, support, policy) {
        DeliveryCompatibility::Compatible
    } else {
        DeliveryCompatibility::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
    }
}

fn delivery_supports_objective(
    objective: &RouteDeliveryObjective,
    support: &TransportDeliverySupport,
    policy: DeliveryCompatibilityPolicy,
) -> bool {
    match (objective, support) {
        (
            RouteDeliveryObjective::UnicastNode { node_id },
            TransportDeliverySupport::IsolatedUnicast { receiver, .. },
        ) => node_id == receiver,
        (
            RouteDeliveryObjective::MulticastGroup {
                group_id,
                receivers,
            },
            TransportDeliverySupport::Multicast {
                group_id: support_group,
                receivers: support_receivers,
                ..
            },
        ) => group_id == support_group && covers_receivers(support_receivers, receivers.as_slice()),
        (
            RouteDeliveryObjective::MulticastGroup { receivers, .. },
            TransportDeliverySupport::IsolatedUnicast { receiver, .. },
        ) => {
            policy.allows_repeated_unicast()
                && receivers.len() == 1
                && receivers.first() == Some(receiver)
        }
        (
            RouteDeliveryObjective::BroadcastDomain {
                domain_id,
                receivers,
                coverage,
            },
            support,
        ) => broadcast_supports_objective(*domain_id, receivers, *coverage, support, policy),
        _ => false,
    }
}

fn broadcast_supports_objective(
    domain_id: BroadcastDomainId,
    receivers: &[NodeId],
    coverage: DeliveryCoverageObjective,
    support: &TransportDeliverySupport,
    policy: DeliveryCompatibilityPolicy,
) -> bool {
    match support {
        TransportDeliverySupport::Broadcast {
            domain_id: support_domain,
            receivers: support_receivers,
            reverse_confirmation,
            ..
        } => {
            domain_id == *support_domain
                && coverage_satisfied(coverage, support_receivers, receivers)
                && (*reverse_confirmation == ReverseDeliveryConfirmation::Confirmed
                    || policy.allows_lossy_broadcast())
        }
        TransportDeliverySupport::Multicast {
            receivers: support_receivers,
            ..
        } => {
            policy.allows_repeated_unicast()
                && coverage_satisfied(coverage, support_receivers, receivers)
        }
        TransportDeliverySupport::IsolatedUnicast { receiver, .. } => {
            policy.allows_repeated_unicast()
                && coverage_satisfied(coverage, &[*receiver], receivers)
        }
    }
}

fn coverage_satisfied(
    coverage: DeliveryCoverageObjective,
    support_receivers: &[NodeId],
    objective_receivers: &[NodeId],
) -> bool {
    match coverage {
        DeliveryCoverageObjective::AnyReceiver => objective_receivers
            .iter()
            .any(|receiver| support_receivers.contains(receiver)),
        DeliveryCoverageObjective::AllReceivers => {
            covers_receivers(support_receivers, objective_receivers)
        }
    }
}

fn covers_receivers(support_receivers: &[NodeId], objective_receivers: &[NodeId]) -> bool {
    objective_receivers
        .iter()
        .all(|receiver| support_receivers.contains(receiver))
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
        self.repair_capacity_slots = Belief::certain(repair_capacity_slots, updated_at_tick);
        self
    }

    #[must_use]
    pub fn with_hold_capacity_bytes(
        mut self,
        hold_capacity_bytes: ByteCount,
        updated_at_tick: Tick,
    ) -> Self {
        self.hold_capacity_bytes = Belief::certain(hold_capacity_bytes, updated_at_tick);
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
            Self::PayloadReceived {
                from_node_id,
                endpoint,
                payload,
            } => TransportObservation::PayloadReceived {
                from_node_id,
                endpoint,
                payload,
                observed_at_tick,
            },
            Self::LinkObserved {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn group(byte: u8) -> MulticastGroupId {
        MulticastGroupId([byte; 16])
    }

    fn domain(byte: u8) -> BroadcastDomainId {
        BroadcastDomainId([byte; 16])
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        LinkEndpoint::new(
            TransportKind::BleGatt,
            EndpointLocator::Opaque(vec![byte]),
            ByteCount(128),
        )
    }

    #[test]
    fn delivery_intent_preserves_mode_and_endpoint() {
        let endpoint = endpoint(9);
        let intent = TransportDeliveryIntent::Multicast {
            endpoint: endpoint.clone(),
            group_id: group(1),
            receivers: vec![node(2), node(3)],
        };

        assert_eq!(intent.mode(), TransportDeliveryMode::Multicast);
        assert_eq!(intent.endpoint(), &endpoint);
    }

    #[test]
    fn delivery_compatibility_rejects_fanout_for_node_unicast() {
        let objective = RouteDeliveryObjective::unicast_node(node(2));
        let support = TransportDeliverySupport::Multicast {
            endpoint: endpoint(1),
            group_id: group(7),
            receivers: vec![node(2), node(3)],
        };

        assert_eq!(
            objective.compatible_with(&support, DeliveryCompatibilityPolicy::ExactDeliveryOnly),
            DeliveryCompatibility::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
        );
    }

    #[test]
    fn delivery_compatibility_accepts_matching_multicast_group() {
        let objective = RouteDeliveryObjective::MulticastGroup {
            group_id: group(4),
            receivers: vec![node(2), node(3)],
        };
        let support = TransportDeliverySupport::Multicast {
            endpoint: endpoint(1),
            group_id: group(4),
            receivers: vec![node(1), node(2), node(3)],
        };

        assert_eq!(
            objective.compatible_with(&support, DeliveryCompatibilityPolicy::ExactDeliveryOnly),
            DeliveryCompatibility::Compatible
        );
    }

    #[test]
    fn delivery_compatibility_labels_lossy_broadcast_policy() {
        let objective = RouteDeliveryObjective::broadcast_domain(
            domain(5),
            [node(2), node(3)],
            DeliveryCoverageObjective::AllReceivers,
        );
        let support = TransportDeliverySupport::Broadcast {
            endpoint: endpoint(1),
            domain_id: domain(5),
            receivers: vec![node(2), node(3)],
            reverse_confirmation: ReverseDeliveryConfirmation::Unconfirmed,
        };

        assert_eq!(
            objective.compatible_with(&support, DeliveryCompatibilityPolicy::ExactDeliveryOnly),
            DeliveryCompatibility::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
        );
        assert_eq!(
            objective.compatible_with(&support, DeliveryCompatibilityPolicy::AllowLossyBroadcast),
            DeliveryCompatibility::Compatible
        );
    }

    #[test]
    fn lossy_broadcast_policy_does_not_enable_repeated_unicast() {
        let objective = RouteDeliveryObjective::MulticastGroup {
            group_id: group(4),
            receivers: vec![node(2)],
        };
        let support = TransportDeliverySupport::IsolatedUnicast {
            endpoint: endpoint(1),
            receiver: node(2),
        };

        assert_eq!(
            objective.compatible_with(&support, DeliveryCompatibilityPolicy::AllowLossyBroadcast),
            DeliveryCompatibility::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
        );
    }

    #[test]
    fn repeated_unicast_policy_does_not_enable_lossy_broadcast() {
        let objective = RouteDeliveryObjective::broadcast_domain(
            domain(5),
            [node(2), node(3)],
            DeliveryCoverageObjective::AllReceivers,
        );
        let support = TransportDeliverySupport::Broadcast {
            endpoint: endpoint(1),
            domain_id: domain(5),
            receivers: vec![node(2), node(3)],
            reverse_confirmation: ReverseDeliveryConfirmation::Unconfirmed,
        };

        assert_eq!(
            objective.compatible_with(&support, DeliveryCompatibilityPolicy::AllowRepeatedUnicast),
            DeliveryCompatibility::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
        );
    }

    #[test]
    fn broadcast_coverage_requires_requested_receivers() {
        let objective = RouteDeliveryObjective::broadcast_domain(
            domain(5),
            [node(2), node(3)],
            DeliveryCoverageObjective::AllReceivers,
        );
        let support = TransportDeliverySupport::Broadcast {
            endpoint: endpoint(1),
            domain_id: domain(5),
            receivers: vec![node(2)],
            reverse_confirmation: ReverseDeliveryConfirmation::Confirmed,
        };

        assert_eq!(
            objective.compatible_with(&support, DeliveryCompatibilityPolicy::ExactDeliveryOnly),
            DeliveryCompatibility::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
        );
    }

    #[test]
    fn broadcast_coverage_rejects_mismatched_domain() {
        let objective = RouteDeliveryObjective::broadcast_domain(
            domain(5),
            [node(2)],
            DeliveryCoverageObjective::AnyReceiver,
        );
        let support = TransportDeliverySupport::Broadcast {
            endpoint: endpoint(1),
            domain_id: domain(6),
            receivers: vec![node(2)],
            reverse_confirmation: ReverseDeliveryConfirmation::Confirmed,
        };

        assert_eq!(
            objective.compatible_with(&support, DeliveryCompatibilityPolicy::ExactDeliveryOnly),
            DeliveryCompatibility::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
        );
    }

    #[test]
    fn delivery_support_serializes_with_stable_mode_label() {
        let support = TransportDeliverySupport::IsolatedUnicast {
            endpoint: endpoint(8),
            receiver: node(2),
        };

        let encoded = serde_json::to_string(&support).expect("serialize support");

        assert!(encoded.contains("IsolatedUnicast"));
    }
}
