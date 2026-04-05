//! Node, controller, route, and scope identifiers.

use std::net::IpAddr;

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::RouteEpoch;

macro_rules! bytes_newtype {
    ($name:ident, $size:expr) => {
        #[id_type]
        pub struct $name(pub [u8; $size]);
    };
}

// NodeId identifies a running Jacquard participant instance.
// ControllerId identifies the cryptographic actor behind one or more nodes.
bytes_newtype!(NodeId, 32);
bytes_newtype!(ControllerId, 32);
bytes_newtype!(KeyId, 32);
bytes_newtype!(BleProfileId, 16);
bytes_newtype!(NeighborhoodId, 16);
bytes_newtype!(HomeId, 16);
bytes_newtype!(ClusterId, 16);
bytes_newtype!(GatewayId, 16);
bytes_newtype!(RouteId, 16);
bytes_newtype!(RouteFamilyContractId, 16);
bytes_newtype!(RouteOperationId, 16);
bytes_newtype!(RouteCommitmentId, 16);
bytes_newtype!(PathId, 16);
bytes_newtype!(PublicationId, 16);
bytes_newtype!(ReceiptId, 16);

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub Vec<u8>);

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BackendRouteId(pub Vec<u8>);

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
/// Identifies a route family contract. Mesh is first-party; External covers third-party families.
pub enum RouteFamilyId {
    Mesh,
    External {
        name: String,
        contract_id: RouteFamilyContractId,
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
