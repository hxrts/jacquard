//! Canonical backend-token encoding for Mercator corridor routes.

// proc-macro-scope: Mercator engine-private token encoding stays outside #[public_model].

use jacquard_core::{
    BackendRouteId, DestinationId, GatewayId, NodeId, RouteEpoch, RouteId, RouteSelectionError,
    ServiceId,
};
use jacquard_traits::{Blake3Hashing, Hashing};

const MERCATOR_ROUTE_ID_DOMAIN: &[u8] = b"jacquard.mercator.route";
const MERCATOR_TOKEN_VERSION: u8 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MercatorBackendToken {
    pub(super) topology_epoch: RouteEpoch,
    pub(super) destination: DestinationId,
    pub(super) primary_path: Vec<NodeId>,
    pub(super) alternate_next_hops: Vec<NodeId>,
    pub(super) alternate_paths: Vec<Vec<NodeId>>,
    pub(super) support_score: u16,
}

pub(super) fn route_id_for_backend(backend_route_id: &BackendRouteId) -> RouteId {
    RouteId::from(&Blake3Hashing.hash_tagged(MERCATOR_ROUTE_ID_DOMAIN, &backend_route_id.0))
}

pub(super) fn encode_backend_token(
    token: &MercatorBackendToken,
) -> Result<BackendRouteId, RouteSelectionError> {
    let mut bytes = Vec::new();
    bytes.push(MERCATOR_TOKEN_VERSION);
    bytes.extend_from_slice(&token.topology_epoch.0.to_be_bytes());
    encode_destination(&token.destination, &mut bytes)?;
    bytes.push(
        u8::try_from(token.primary_path.len()).map_err(|_| RouteSelectionError::PolicyConflict)?,
    );
    for node in &token.primary_path {
        bytes.extend_from_slice(&node.0);
    }
    bytes.push(
        u8::try_from(token.alternate_next_hops.len())
            .map_err(|_| RouteSelectionError::PolicyConflict)?,
    );
    for node in &token.alternate_next_hops {
        bytes.extend_from_slice(&node.0);
    }
    bytes.push(
        u8::try_from(token.alternate_paths.len())
            .map_err(|_| RouteSelectionError::PolicyConflict)?,
    );
    for path in &token.alternate_paths {
        bytes.push(u8::try_from(path.len()).map_err(|_| RouteSelectionError::PolicyConflict)?);
        for node in path {
            bytes.extend_from_slice(&node.0);
        }
    }
    bytes.extend_from_slice(&token.support_score.to_be_bytes());
    Ok(BackendRouteId(bytes))
}

pub(super) fn decode_backend_token(
    backend_route_id: &BackendRouteId,
) -> Option<MercatorBackendToken> {
    let bytes = &backend_route_id.0;
    let mut cursor = 0_usize;
    if *bytes.get(cursor)? != MERCATOR_TOKEN_VERSION {
        return None;
    }
    cursor = cursor.saturating_add(1);
    let topology_epoch = RouteEpoch(u64::from_be_bytes(read_array(bytes, &mut cursor)?));
    let destination = decode_destination(bytes, &mut cursor)?;
    let primary_len = usize::from(*bytes.get(cursor)?);
    cursor = cursor.saturating_add(1);
    let mut primary_path = Vec::with_capacity(primary_len);
    for _ in 0..primary_len {
        primary_path.push(NodeId(read_array(bytes, &mut cursor)?));
    }
    let alternate_len = usize::from(*bytes.get(cursor)?);
    cursor = cursor.saturating_add(1);
    let mut alternate_next_hops = Vec::with_capacity(alternate_len);
    for _ in 0..alternate_len {
        alternate_next_hops.push(NodeId(read_array(bytes, &mut cursor)?));
    }
    let alternate_path_len = usize::from(*bytes.get(cursor)?);
    cursor = cursor.saturating_add(1);
    let mut alternate_paths = Vec::with_capacity(alternate_path_len);
    for _ in 0..alternate_path_len {
        let path_len = usize::from(*bytes.get(cursor)?);
        cursor = cursor.saturating_add(1);
        let mut path = Vec::with_capacity(path_len);
        for _ in 0..path_len {
            path.push(NodeId(read_array(bytes, &mut cursor)?));
        }
        alternate_paths.push(path);
    }
    let support_score = u16::from_be_bytes(read_array(bytes, &mut cursor)?);
    Some(MercatorBackendToken {
        topology_epoch,
        destination,
        primary_path,
        alternate_next_hops,
        alternate_paths,
        support_score,
    })
}

fn encode_destination(
    destination: &DestinationId,
    out: &mut Vec<u8>,
) -> Result<(), RouteSelectionError> {
    match destination {
        DestinationId::Node(node) => {
            out.push(0);
            out.extend_from_slice(&node.0);
        }
        DestinationId::Service(service) => {
            out.push(1);
            let len =
                u16::try_from(service.0.len()).map_err(|_| RouteSelectionError::PolicyConflict)?;
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(&service.0);
        }
        DestinationId::Gateway(gateway) => {
            out.push(2);
            out.extend_from_slice(&gateway.0);
        }
    }
    Ok(())
}

fn decode_destination(bytes: &[u8], cursor: &mut usize) -> Option<DestinationId> {
    let kind = *bytes.get(*cursor)?;
    *cursor = (*cursor).saturating_add(1);
    match kind {
        0 => Some(DestinationId::Node(NodeId(read_array(bytes, cursor)?))),
        1 => {
            let len = usize::from(u16::from_be_bytes(read_array(bytes, cursor)?));
            let end = (*cursor).checked_add(len)?;
            let service = bytes.get(*cursor..end)?.to_vec();
            *cursor = end;
            Some(DestinationId::Service(ServiceId(service)))
        }
        2 => Some(DestinationId::Gateway(GatewayId(read_array(
            bytes, cursor,
        )?))),
        _ => None,
    }
}

fn read_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Option<[u8; N]> {
    let end = cursor.checked_add(N)?;
    let mut out = [0u8; N];
    out.copy_from_slice(bytes.get(*cursor..end)?);
    *cursor = end;
    Some(out)
}
