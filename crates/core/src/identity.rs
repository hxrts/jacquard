//! Node, controller, route, and scope identifiers.

use contour_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::RouteEpoch;

macro_rules! bytes_newtype {
    ($name:ident, $size:expr) => {
        #[id_type]
        pub struct $name(pub [u8; $size]);
    };
}

// NodeId identifies a running Contour participant instance.
// ControllerId identifies the cryptographic actor behind one or more nodes.
bytes_newtype!(NodeId, 32);
bytes_newtype!(ControllerId, 32);
bytes_newtype!(NeighborhoodId, 16);
bytes_newtype!(HomeId, 16);
bytes_newtype!(ClusterId, 16);
bytes_newtype!(GatewayDomainId, 16);
bytes_newtype!(RouteId, 16);
bytes_newtype!(RouteOperationId, 16);
bytes_newtype!(RouteCommitmentId, 16);
bytes_newtype!(PathId, 16);

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub Vec<u8>);

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestinationId {
    Node(NodeId),
    Service(ServiceId),
    Gateway(GatewayDomainId),
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
        key_id: [u8; 32],
        signature_bytes: Vec<u8>,
    },
    Opaque(Vec<u8>),
}
