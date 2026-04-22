//! Cast delivery-support adapters for in-memory link authoring.
//!
//! `jacquard-cast-support` shapes profile-owned physical facts into
//! route-neutral delivery support. This module turns that support into ordinary
//! in-memory directed links for tests and reference fixtures. Endpoint authoring
//! remains caller-owned through an explicit resolver closure.

use jacquard_cast_support::{
    BroadcastDeliverySupport, MulticastDeliverySupport, UnicastDeliverySupport,
};
use jacquard_core::{ByteCount, Link, LinkEndpoint, NodeId, RatioPermille, Tick};

use crate::{LinkPreset, LinkPresetOptions};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastLinkObservation {
    pub from: NodeId,
    pub to: NodeId,
    pub link: Link,
}

pub struct CastLinkPreset;

impl CastLinkPreset {
    #[must_use]
    pub fn from_unicast_support(
        support: &UnicastDeliverySupport,
        mut endpoint_for: impl FnMut(NodeId, ByteCount) -> LinkEndpoint,
    ) -> CastLinkObservation {
        directed_link(
            support.sender,
            support.receiver,
            support.confidence_permille,
            support.payload_bytes_max,
            support.meta.observed_at_tick,
            &mut endpoint_for,
        )
    }

    #[must_use]
    pub fn from_multicast_support(
        support: &MulticastDeliverySupport,
        mut endpoint_for: impl FnMut(NodeId, ByteCount) -> LinkEndpoint,
    ) -> Vec<CastLinkObservation> {
        support
            .receivers
            .iter()
            .map(|receiver| {
                directed_link(
                    support.sender,
                    receiver.receiver,
                    support.confidence_permille,
                    support.payload_bytes_max,
                    support.meta.observed_at_tick,
                    &mut endpoint_for,
                )
            })
            .collect()
    }

    #[must_use]
    pub fn from_broadcast_support(
        support: &BroadcastDeliverySupport,
        mut endpoint_for: impl FnMut(NodeId, ByteCount) -> LinkEndpoint,
    ) -> Vec<CastLinkObservation> {
        support
            .receivers
            .iter()
            .map(|receiver| {
                directed_link(
                    support.sender,
                    receiver.receiver,
                    support.confidence_permille,
                    support.payload_bytes_max,
                    support.meta.observed_at_tick,
                    &mut endpoint_for,
                )
            })
            .collect()
    }
}

fn directed_link(
    from: NodeId,
    to: NodeId,
    confidence: RatioPermille,
    payload_bytes_max: ByteCount,
    observed_at_tick: Tick,
    endpoint_for: &mut impl FnMut(NodeId, ByteCount) -> LinkEndpoint,
) -> CastLinkObservation {
    let endpoint = endpoint_for(to, payload_bytes_max);
    let link = LinkPreset::lossy(
        LinkPresetOptions::new(endpoint, observed_at_tick).with_confidence(confidence),
    )
    .build();
    CastLinkObservation { from, to, link }
}

#[cfg(test)]
mod tests {
    use jacquard_cast_support::{
        BroadcastDeliverySupport, BroadcastSupportKind, CastDeliveryResourceUse, CastEvidenceMeta,
        MulticastDeliverySupport, ReceiverCoverageEvidence, UnicastDeliverySupport,
    };
    use jacquard_core::{ByteCount, DurationMs, LinkEndpoint, OrderStamp, TransportKind};
    use jacquard_host_support::opaque_endpoint;

    use super::*;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn endpoint(node: NodeId, payload_bytes_max: ByteCount) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![node.0[0]], payload_bytes_max)
    }

    fn meta(order: u64) -> CastEvidenceMeta {
        CastEvidenceMeta::new(
            Tick(7),
            DurationMs(10),
            DurationMs(1_000),
            OrderStamp(order),
        )
    }

    fn resource_use(payload_bytes: u64) -> CastDeliveryResourceUse {
        CastDeliveryResourceUse {
            receiver_count: 1,
            fanout_used: 1,
            copy_budget_used: 0,
            payload_bytes: ByteCount(payload_bytes),
        }
    }

    #[test]
    fn unicast_support_builds_one_directed_link() {
        let support = UnicastDeliverySupport {
            sender: node(1),
            receiver: node(2),
            confidence_permille: RatioPermille(850),
            bidirectional_confidence_permille: RatioPermille(800),
            payload_bytes_max: ByteCount(512),
            resource_use: resource_use(128),
            meta: meta(1),
        };

        let observation = CastLinkPreset::from_unicast_support(&support, endpoint);

        assert_eq!(observation.from, node(1));
        assert_eq!(observation.to, node(2));
        assert_eq!(
            observation
                .link
                .state
                .delivery_confidence_permille
                .value_or(RatioPermille(0)),
            RatioPermille(850)
        );
        assert_eq!(observation.link.endpoint.mtu_bytes, ByteCount(512));
    }

    #[test]
    fn multicast_support_builds_stable_receiver_links() {
        let support = MulticastDeliverySupport {
            sender: node(1),
            group_id: jacquard_cast_support::CastGroupId(b"team".to_vec()),
            receivers: vec![
                ReceiverCoverageEvidence {
                    receiver: node(2),
                    confidence_permille: RatioPermille(900),
                },
                ReceiverCoverageEvidence {
                    receiver: node(3),
                    confidence_permille: RatioPermille(800),
                },
            ],
            confidence_permille: RatioPermille(720),
            group_pressure_permille: RatioPermille(100),
            payload_bytes_max: ByteCount(256),
            resource_use: CastDeliveryResourceUse {
                receiver_count: 2,
                fanout_used: 2,
                copy_budget_used: 0,
                payload_bytes: ByteCount(128),
            },
            meta: meta(2),
        };

        let observations = CastLinkPreset::from_multicast_support(&support, endpoint);

        assert_eq!(
            observations
                .iter()
                .map(|observation| observation.to)
                .collect::<Vec<_>>(),
            vec![node(2), node(3)]
        );
        assert!(observations.iter().all(|observation| {
            observation
                .link
                .state
                .delivery_confidence_permille
                .value_or(RatioPermille(0))
                == RatioPermille(720)
        }));
    }

    #[test]
    fn broadcast_support_preserves_profile_side_delivery_confidence() {
        let support = BroadcastDeliverySupport {
            sender: node(1),
            receivers: vec![ReceiverCoverageEvidence {
                receiver: node(4),
                confidence_permille: RatioPermille(750),
            }],
            support: BroadcastSupportKind::DirectionalOnly,
            confidence_permille: RatioPermille(600),
            reverse_confirmation_permille: None,
            channel_pressure_permille: RatioPermille(100),
            payload_bytes_max: ByteCount(512),
            resource_use: resource_use(128),
            meta: meta(3),
        };

        let observations = CastLinkPreset::from_broadcast_support(&support, endpoint);

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].to, node(4));
        assert_eq!(
            observations[0]
                .link
                .state
                .delivery_confidence_permille
                .value_or(RatioPermille(0)),
            RatioPermille(600)
        );
    }
}
