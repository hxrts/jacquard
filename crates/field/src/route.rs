//! Backend route token encoding and active route tracking for the field engine.
//!
//! `FieldBackendToken` packs the routing decision (destination, primary and
//! alternate neighbors, topology epoch, operating regime, and routing posture)
//! into an opaque byte vector embedded in `BackendRouteRef`. Tokens are encoded
//! by `encode_backend_token` and decoded by `decode_backend_token` with bounds
//! validation. `route_id_for_backend` derives a stable `RouteId` via Blake3
//! over the destination and primary neighbor identifiers.
//!
//! `ActiveFieldRoute` is the runtime record of an installed route. It carries
//! the witness detail (evidence class, uncertainty, regime, posture,
//! degradation) and the corridor envelope at the time of materialization.
//! Active routes are keyed by `RouteId` and consulted during `maintain_route`
//! to detect attractor shifts, posture changes, and delivery support drops that
//! require replacement.

use jacquard_core::{
    BackendRouteId, GatewayId, NodeId, RouteDegradation, RouteEpoch, RouteId, Tick,
};
use jacquard_traits::{Blake3Hashing, Hashing};

use crate::{
    state::{
        CorridorBeliefEnvelope, DestinationKey, OperatingRegime, RoutingPosture,
        MAX_ALTERNATE_COUNT,
    },
    summary::{EvidenceContributionClass, SummaryUncertaintyClass},
};

const FIELD_BACKEND_TOKEN_VERSION: u8 = 1;
const FIELD_ROUTE_ID_DOMAIN: &[u8] = b"field-route-id";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldWitnessDetail {
    pub(crate) evidence_class: EvidenceContributionClass,
    pub(crate) uncertainty_class: SummaryUncertaintyClass,
    pub(crate) regime: OperatingRegime,
    pub(crate) posture: RoutingPosture,
    pub(crate) degradation: RouteDegradation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldBackendToken {
    pub(crate) destination: DestinationKey,
    pub(crate) primary_neighbor: NodeId,
    pub(crate) alternates: Vec<NodeId>,
    pub(crate) topology_epoch: RouteEpoch,
    pub(crate) regime: OperatingRegime,
    pub(crate) posture: RoutingPosture,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveFieldRoute {
    pub(crate) destination: DestinationKey,
    pub(crate) primary_neighbor: NodeId,
    pub(crate) alternates: Vec<NodeId>,
    pub(crate) corridor_envelope: CorridorBeliefEnvelope,
    pub(crate) witness_detail: FieldWitnessDetail,
    pub(crate) backend_route_id: BackendRouteId,
    pub(crate) topology_epoch: RouteEpoch,
    pub(crate) installed_at_tick: Tick,
}

#[must_use]
pub(crate) fn encode_backend_token(token: &FieldBackendToken) -> BackendRouteId {
    let mut bytes = Vec::with_capacity(128);
    let alternate_count = token.alternates.len().min(MAX_ALTERNATE_COUNT);
    bytes.push(FIELD_BACKEND_TOKEN_VERSION);
    encode_destination(&token.destination, &mut bytes);
    bytes.extend_from_slice(&token.primary_neighbor.0);
    bytes.push(u8::try_from(alternate_count).expect("bounded alternate count fits u8"));
    for alternate in token.alternates.iter().take(alternate_count) {
        bytes.extend_from_slice(&alternate.0);
    }
    bytes.extend_from_slice(&token.topology_epoch.0.to_le_bytes());
    bytes.push(regime_code(token.regime));
    bytes.push(posture_code(token.posture));
    BackendRouteId(bytes)
}

#[must_use]
pub(crate) fn decode_backend_token(backend_route_id: &BackendRouteId) -> Option<FieldBackendToken> {
    let bytes = &backend_route_id.0;
    if bytes.first().copied()? != FIELD_BACKEND_TOKEN_VERSION {
        return None;
    }
    let mut cursor = 1_usize;
    let destination = decode_destination(bytes, &mut cursor)?;
    let primary_neighbor = NodeId(bytes.get(cursor..cursor + 32)?.try_into().ok()?);
    cursor += 32;
    let alternate_count = usize::from(*bytes.get(cursor)?);
    if alternate_count > MAX_ALTERNATE_COUNT {
        return None;
    }
    cursor += 1;
    let mut alternates = Vec::with_capacity(alternate_count);
    for _ in 0..alternate_count {
        alternates.push(NodeId(bytes.get(cursor..cursor + 32)?.try_into().ok()?));
        cursor += 32;
    }
    let topology_epoch = RouteEpoch(u64::from_le_bytes(
        bytes.get(cursor..cursor + 8)?.try_into().ok()?,
    ));
    cursor += 8;
    let regime = regime_from_code(*bytes.get(cursor)?)?;
    cursor += 1;
    let posture = posture_from_code(*bytes.get(cursor)?)?;
    cursor += 1;
    if cursor != bytes.len() {
        return None;
    }
    Some(FieldBackendToken {
        destination,
        primary_neighbor,
        alternates,
        topology_epoch,
        regime,
        posture,
    })
}

#[must_use]
pub(crate) fn route_id_for_backend(backend_route_id: &BackendRouteId) -> RouteId {
    let digest = Blake3Hashing.hash_tagged(FIELD_ROUTE_ID_DOMAIN, &backend_route_id.0);
    RouteId::from(&digest)
}

fn encode_destination(destination: &DestinationKey, out: &mut Vec<u8>) {
    match destination {
        DestinationKey::Node(node) => {
            out.push(0);
            out.extend_from_slice(&node.0);
        }
        DestinationKey::Gateway(gateway) => {
            out.push(1);
            out.extend_from_slice(&gateway.0);
        }
        DestinationKey::Service(service) => {
            out.push(2);
            let len = u16::try_from(service.len()).expect("service id length fits u16");
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(service);
        }
    }
}

fn decode_destination(bytes: &[u8], cursor: &mut usize) -> Option<DestinationKey> {
    match *bytes.get(*cursor)? {
        0 => {
            *cursor += 1;
            let node = NodeId(bytes.get(*cursor..*cursor + 32)?.try_into().ok()?);
            *cursor += 32;
            Some(DestinationKey::Node(node))
        }
        1 => {
            *cursor += 1;
            let gateway = GatewayId(bytes.get(*cursor..*cursor + 16)?.try_into().ok()?);
            *cursor += 16;
            Some(DestinationKey::Gateway(gateway))
        }
        2 => {
            *cursor += 1;
            let len = u16::from_le_bytes(bytes.get(*cursor..*cursor + 2)?.try_into().ok()?);
            *cursor += 2;
            let len = usize::from(len);
            let service = bytes.get(*cursor..*cursor + len)?.to_vec();
            *cursor += len;
            Some(DestinationKey::Service(service))
        }
        _ => None,
    }
}

fn regime_code(regime: OperatingRegime) -> u8 {
    match regime {
        OperatingRegime::Sparse => 0,
        OperatingRegime::Congested => 1,
        OperatingRegime::RetentionFavorable => 2,
        OperatingRegime::Unstable => 3,
        OperatingRegime::Adversarial => 4,
    }
}

fn regime_from_code(code: u8) -> Option<OperatingRegime> {
    Some(match code {
        0 => OperatingRegime::Sparse,
        1 => OperatingRegime::Congested,
        2 => OperatingRegime::RetentionFavorable,
        3 => OperatingRegime::Unstable,
        4 => OperatingRegime::Adversarial,
        _ => return None,
    })
}

fn posture_code(posture: RoutingPosture) -> u8 {
    match posture {
        RoutingPosture::Opportunistic => 0,
        RoutingPosture::Structured => 1,
        RoutingPosture::RetentionBiased => 2,
        RoutingPosture::RiskSuppressed => 3,
    }
}

fn posture_from_code(code: u8) -> Option<RoutingPosture> {
    Some(match code {
        0 => RoutingPosture::Opportunistic,
        1 => RoutingPosture::Structured,
        2 => RoutingPosture::RetentionBiased,
        3 => RoutingPosture::RiskSuppressed,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use jacquard_core::{NodeId, RouteEpoch};

    use super::*;

    #[test]
    fn backend_token_round_trips() {
        let token = FieldBackendToken {
            destination: DestinationKey::Service(vec![1, 2, 3]),
            primary_neighbor: NodeId([1; 32]),
            alternates: vec![NodeId([2; 32]), NodeId([3; 32])],
            topology_epoch: RouteEpoch(9),
            regime: OperatingRegime::Congested,
            posture: RoutingPosture::RetentionBiased,
        };
        let encoded = encode_backend_token(&token);
        assert_eq!(decode_backend_token(&encoded), Some(token));
    }

    #[test]
    fn route_id_derivation_is_stable() {
        let backend = BackendRouteId(vec![1, 2, 3, 4]);
        assert_eq!(
            route_id_for_backend(&backend),
            route_id_for_backend(&backend),
        );
    }

    #[test]
    fn backend_token_enforces_bounded_alternate_count() {
        let token = FieldBackendToken {
            destination: DestinationKey::Node(NodeId([1; 32])),
            primary_neighbor: NodeId([2; 32]),
            alternates: vec![
                NodeId([3; 32]),
                NodeId([4; 32]),
                NodeId([5; 32]),
                NodeId([6; 32]),
            ],
            topology_epoch: RouteEpoch(9),
            regime: OperatingRegime::Sparse,
            posture: RoutingPosture::Structured,
        };
        let encoded = encode_backend_token(&token);
        let decoded = decode_backend_token(&encoded).expect("bounded token");
        assert_eq!(decoded.alternates.len(), MAX_ALTERNATE_COUNT);
    }
}
