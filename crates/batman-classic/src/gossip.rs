//! Classic BATMAN OGM encoding: originator ID, sequence, TQ scalar, and hop limit.
//!
//! `OriginatorAdvertisement` carries no link state. Quality is represented
//! entirely by the `tq` field — a permille scalar that the originator
//! initialises to 1000 and each re-broadcasting node updates via `tq_product`
//! before forwarding. The `remaining_hop_limit` field decrements at each hop;
//! OGMs with zero remaining hops are not re-broadcast.
//!
//! Key functions:
//! - `local_advertisement` — builds the originator's own OGM (tq=1000,
//!   hop limit=`DEFAULT_OGM_HOP_LIMIT`).
//! - `rebroadcast_advertisement` — applies `tq_product` and decrements the hop
//!   limit, returning `None` when it reaches zero.
//! - `encode_advertisement` / `decode_advertisement` — frame and validate OGMs
//!   with the eight-byte magic prefix `JQBATMNC`.

use jacquard_core::{NodeId, RatioPermille, Tick};
use serde::{Deserialize, Serialize};

use crate::scoring;

const GOSSIP_MAGIC: &[u8; 8] = b"JQBATMNC";

/// Default hop limit assigned to OGMs at the originating node. Decremented at
/// each re-broadcasting hop; OGMs reaching zero are discarded.
pub(crate) const DEFAULT_OGM_HOP_LIMIT: u8 = 50;

/// A classic BATMAN originator message.
///
/// Carries no per-link state. The `tq` field begins at 1000 (perfect quality)
/// at the originator and is multiplied by each re-broadcasting node's link
/// quality to that node before forwarding, producing a monotonically decreasing
/// path-quality estimate as the OGM propagates through the network.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OriginatorAdvertisement {
    pub originator: NodeId,
    pub sequence: u64,
    pub tq: RatioPermille,
    pub remaining_hop_limit: u8,
}

/// A received OGM stored in the local learned-advertisement table.
///
/// `from_neighbor` records which direct neighbor forwarded this OGM to us,
/// allowing `flood_gossip` to compute the correct re-broadcast TQ using the
/// local link quality to that sender.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearnedAdvertisement {
    pub advertisement: OriginatorAdvertisement,
    pub from_neighbor: NodeId,
    pub observed_at_tick: Tick,
}

impl LearnedAdvertisement {
    pub(crate) fn new(
        advertisement: OriginatorAdvertisement,
        from_neighbor: NodeId,
        observed_at_tick: Tick,
    ) -> Self {
        Self {
            advertisement,
            from_neighbor,
            observed_at_tick,
        }
    }
}

/// Encode an OGM for transmission: prepend the eight-byte magic prefix, then
/// postcard-serialize the advertisement.
pub(crate) fn encode_advertisement(
    advertisement: &OriginatorAdvertisement,
) -> Result<Vec<u8>, postcard::Error> {
    let payload = postcard::to_allocvec(advertisement)?;
    let mut framed = Vec::with_capacity(GOSSIP_MAGIC.len() + payload.len());
    framed.extend_from_slice(GOSSIP_MAGIC);
    framed.extend_from_slice(&payload);
    Ok(framed)
}

/// Validate the magic prefix and deserialize an OGM from a raw payload.
pub(crate) fn decode_advertisement(payload: &[u8]) -> Option<OriginatorAdvertisement> {
    if !payload.starts_with(GOSSIP_MAGIC) {
        return None;
    }
    postcard::from_bytes(&payload[GOSSIP_MAGIC.len()..]).ok()
}

/// Build the originator's own OGM for flooding.
///
/// TQ starts at 1000 (perfect quality); hop limit starts at
/// `DEFAULT_OGM_HOP_LIMIT`.
/// The `sequence` argument should be monotonically increasing (tick number is
/// an appropriate source).
pub(crate) fn local_advertisement(local_node_id: NodeId, sequence: u64) -> OriginatorAdvertisement {
    OriginatorAdvertisement {
        originator: local_node_id,
        sequence,
        tq: RatioPermille(1000),
        remaining_hop_limit: DEFAULT_OGM_HOP_LIMIT,
    }
}

/// Construct the re-broadcast form of a received OGM.
///
/// Applies `tq_product(local_link_tq, received.tq)` to encode this node's
/// path quality to the originator, then decrements the hop limit. Returns
/// `None` if the OGM's remaining hop limit is already zero.
pub(crate) fn rebroadcast_advertisement(
    received: &OriginatorAdvertisement,
    local_link_tq: RatioPermille,
) -> Option<OriginatorAdvertisement> {
    if received.remaining_hop_limit == 0 {
        return None;
    }
    Some(OriginatorAdvertisement {
        originator: received.originator,
        sequence: received.sequence,
        tq: scoring::tq_product(local_link_tq, received.tq),
        remaining_hop_limit: received.remaining_hop_limit - 1,
    })
}
