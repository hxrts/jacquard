//! BATMAN gossip advertisement encoding and topology merging.
//!
//! `OriginatorAdvertisement` carries a node's self-reported link state: its
//! originator ID, a sequence number, and a list of `AdvertisedLink` entries
//! for each directly reachable neighbor. Advertisements are framed with the
//! eight-byte magic prefix `JQBATMAN` and bincode-serialized by
//! `encode_advertisement`, then validated and deserialized by
//! `decode_advertisement`. `local_advertisement` builds the advertisement for
//! the local node from the current topology observation. `merge_advertisements`
//! folds a map of learned advertisements into a topology snapshot, inserting
//! synthesized links for any gossip-discovered edges whose source nodes are
//! still within the staleness window.

use std::collections::BTreeMap;

use jacquard_core::{
    Belief, ByteCount, Configuration, DurationMs, EndpointLocator, Link, LinkEndpoint, LinkProfile,
    LinkRuntimeState, LinkState, NodeId, Observation, RatioPermille, Tick, TransportKind,
};
use serde::{Deserialize, Serialize};

const GOSSIP_MAGIC: &[u8; 8] = b"JQBATMAN";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OriginatorAdvertisement {
    pub originator: NodeId,
    pub sequence: u64,
    pub links: Vec<AdvertisedLink>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AdvertisedLink {
    pub to_node_id: NodeId,
    pub transport_kind: TransportKind,
    pub runtime_state: LinkRuntimeState,
    pub delivery_confidence_permille: Option<RatioPermille>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearnedAdvertisement {
    pub advertisement: OriginatorAdvertisement,
    pub observed_at_tick: Tick,
}

impl LearnedAdvertisement {
    pub(crate) fn new(advertisement: OriginatorAdvertisement, observed_at_tick: Tick) -> Self {
        Self {
            advertisement,
            observed_at_tick,
        }
    }
}

pub(crate) fn encode_advertisement(
    advertisement: &OriginatorAdvertisement,
) -> Result<Vec<u8>, bincode::Error> {
    let payload = bincode::serialize(advertisement)?;
    let mut framed = Vec::with_capacity(GOSSIP_MAGIC.len() + payload.len());
    framed.extend_from_slice(GOSSIP_MAGIC);
    framed.extend_from_slice(&payload);
    Ok(framed)
}

pub(crate) fn decode_advertisement(payload: &[u8]) -> Option<OriginatorAdvertisement> {
    if !payload.starts_with(GOSSIP_MAGIC) {
        return None;
    }
    bincode::deserialize(&payload[GOSSIP_MAGIC.len()..]).ok()
}

pub(crate) fn local_advertisement(
    local_node_id: NodeId,
    topology: &Observation<Configuration>,
    sequence: u64,
) -> OriginatorAdvertisement {
    let links = topology
        .value
        .links
        .iter()
        .filter(|((from_node_id, _), _)| *from_node_id == local_node_id)
        .map(|((_, to_node_id), link)| AdvertisedLink {
            to_node_id: *to_node_id,
            transport_kind: link.endpoint.transport_kind.clone(),
            runtime_state: link.state.state,
            delivery_confidence_permille: match &link.state.delivery_confidence_permille {
                Belief::Absent => None,
                Belief::Estimated(estimate) => Some(estimate.value),
            },
        })
        .collect();
    OriginatorAdvertisement {
        originator: local_node_id,
        sequence,
        links,
    }
}

pub(crate) fn merge_advertisements(
    topology: &Observation<Configuration>,
    advertisements: &BTreeMap<NodeId, LearnedAdvertisement>,
    now: Tick,
    stale_after_ticks: u64,
) -> Observation<Configuration> {
    let mut merged = topology.clone();
    for learned in advertisements.values() {
        if now.0.saturating_sub(learned.observed_at_tick.0) > stale_after_ticks {
            continue;
        }
        for advertised in &learned.advertisement.links {
            let Some(destination) = merged.value.nodes.get(&advertised.to_node_id) else {
                continue;
            };
            merged
                .value
                .links
                .entry((learned.advertisement.originator, advertised.to_node_id))
                .or_insert_with(|| advertised_link_to_link(destination, advertised));
        }
    }
    merged
}

fn advertised_link_to_link(destination: &jacquard_core::Node, advertised: &AdvertisedLink) -> Link {
    let endpoint = destination
        .profile
        .endpoints
        .first()
        .cloned()
        .map(|mut endpoint| {
            endpoint.transport_kind = advertised.transport_kind.clone();
            endpoint
        })
        .unwrap_or_else(|| LinkEndpoint {
            transport_kind: advertised.transport_kind.clone(),
            locator: EndpointLocator::Opaque(Vec::new()),
            mtu_bytes: ByteCount(64),
        });
    let delivery_confidence_permille = advertised
        .delivery_confidence_permille
        .map_or(Belief::Absent, |value| Belief::certain(value, Tick(1)));

    Link {
        endpoint,
        profile: LinkProfile {
            latency_floor_ms: DurationMs(25),
            repair_capability: jacquard_core::RepairCapability::TransportRetransmit,
            partition_recovery: jacquard_core::PartitionRecoveryClass::LocalReconnect,
        },
        state: LinkState {
            state: advertised.runtime_state,
            median_rtt_ms: Belief::Absent,
            transfer_rate_bytes_per_sec: Belief::Absent,
            stability_horizon_ms: Belief::Absent,
            loss_permille: RatioPermille(0),
            delivery_confidence_permille,
            symmetry_permille: Belief::Absent,
        },
    }
}
