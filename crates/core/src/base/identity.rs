//! Node, controller, route, and scope identifier newtypes.
//!
//! This module defines all fixed-size byte-array identifiers used in the
//! routing world model. Participant and authority identifiers (`NodeId`,
//! `ControllerId`, `KeyId`) are 32-byte values. Routing and scope identifiers
//! (`RouteId`, `CommitteeId`, `PathId`, and others) are 16-byte values derived
//! from `Blake3Digest` truncation. Structural types include `RoutingEngineId`
//! (an engine contract wrapper), `DestinationId` (the three-way node/service/
//! gateway address form), and `NodeBinding` with `NodeBindingProof` (the
//! attestable link between a node instance and its controlling authority).
//!
//! All newtypes are declared with `bytes_newtype!` or `id_type` and inherit the
//! standard routing model derives. `From<&Blake3Digest>` impls for the 16-byte
//! routing id types live at the bottom of this file.

use alloc::vec::Vec;

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::{content::Blake3Digest, RouteEpoch};

// NodeId identifies a running Jacquard participant instance.
// ControllerId identifies the cryptographic actor behind one or more nodes.
super::bytes_newtype!(NodeId, 32);
super::bytes_newtype!(ControllerId, 32);
super::bytes_newtype!(KeyId, 32);
super::bytes_newtype!(DiscoveryScopeId, 16);
super::bytes_newtype!(HomeId, 16);
super::bytes_newtype!(ClusterId, 16);
super::bytes_newtype!(GatewayId, 16);
super::bytes_newtype!(MulticastGroupId, 16);
super::bytes_newtype!(BroadcastDomainId, 16);
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

/// Engine-owned opaque route reference. Jacquard core never inspects the
/// contents.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BackendRouteId(pub Vec<u8>);

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Neutral identifier for a routing-engine contract.
pub struct RoutingEngineId {
    pub contract_id: RoutingEngineContractId,
}

impl RoutingEngineId {
    #[must_use]
    pub const fn new(contract_id: RoutingEngineContractId) -> Self {
        Self { contract_id }
    }

    #[must_use]
    pub const fn from_contract_bytes(contract_id: [u8; 16]) -> Self {
        Self::new(RoutingEngineContractId(contract_id))
    }
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

// Conversions from Blake3Digest to 16-byte routing identity newtypes.
// Each impl takes the first 16 bytes of the 32-byte digest.
impl From<&Blake3Digest> for RouteId {
    fn from(digest: &Blake3Digest) -> Self {
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest.0[..16]);
        RouteId(id)
    }
}

impl From<&Blake3Digest> for RouteCommitmentId {
    fn from(digest: &Blake3Digest) -> Self {
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest.0[..16]);
        RouteCommitmentId(id)
    }
}

impl From<&Blake3Digest> for ReceiptId {
    fn from(digest: &Blake3Digest) -> Self {
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest.0[..16]);
        ReceiptId(id)
    }
}

impl From<&Blake3Digest> for CommitteeId {
    fn from(digest: &Blake3Digest) -> Self {
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest.0[..16]);
        CommitteeId(id)
    }
}
