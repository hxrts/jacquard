//! Node, controller, route, and scope identifiers.

use std::net::IpAddr;

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::RouteEpoch;

// NodeId identifies a running Jacquard participant instance.
// ControllerId identifies the cryptographic actor behind one or more nodes.
super::bytes_newtype!(NodeId, 32);
super::bytes_newtype!(ControllerId, 32);
super::bytes_newtype!(KeyId, 32);
super::bytes_newtype!(BleProfileId, 16);
super::bytes_newtype!(NeighborhoodId, 16);
super::bytes_newtype!(HomeId, 16);
super::bytes_newtype!(ClusterId, 16);
super::bytes_newtype!(GatewayId, 16);
super::bytes_newtype!(RouteId, 16);
super::bytes_newtype!(RoutingEngineContractId, 16);
super::bytes_newtype!(RouteOperationId, 16);
super::bytes_newtype!(RouteCommitmentId, 16);
super::bytes_newtype!(CommitteeId, 16);
super::bytes_newtype!(PathId, 16);
super::bytes_newtype!(PublicationId, 16);
super::bytes_newtype!(ReceiptId, 16);

/// Opaque application-defined service identifier. Format is host-specific.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub Vec<u8>);

/// Engine-owned opaque route reference. Jacquard core never inspects the contents.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BackendRouteId(pub Vec<u8>);

/// Platform-specific BLE device address. Format depends on the host BLE stack.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BleDeviceId(pub Vec<u8>);

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HostName(pub String);

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NetworkHost {
    Ip(IpAddr),
    Name(HostName),
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Identifies a routing-engine contract. Mesh is first-party; External covers third-party engines.
pub enum RoutingEngineId {
    Mesh,
    External {
        name: String,
        contract_id: RoutingEngineContractId,
    },
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestinationId {
    Node(NodeId),
    Service(ServiceId),
    Gateway(GatewayId),
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Attestable link between a node instance and its controlling authority.
/// One controller may bind multiple nodes.
pub struct NodeBinding {
    pub node_id: NodeId,
    pub controller_id: ControllerId,
    pub binding_epoch: RouteEpoch,
    pub proof: NodeBindingProof,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeBindingProof {
    Signature {
        key_id: KeyId,
        signature_bytes: Vec<u8>,
    },
    Opaque(Vec<u8>),
}
