//! Classic BATMAN OGM encoding: originator ID, sequence, TQ scalar, and TTL.
//!
//! Unlike the enhanced batman engine, `OriginatorAdvertisement` carries no link
//! state. Quality is represented entirely by the `tq` field — a permille scalar
//! that the originator initialises to 1000 and each re-broadcasting node
//! updates via `tq_product` before forwarding. The `ttl` field decrements at
//! each hop; OGMs with TTL=0 are not re-broadcast.
//!
//! Key functions:
//! - `local_advertisement` — builds the originator's own OGM (tq=1000,
//!   ttl=`DEFAULT_OGM_TTL`).
//! - `rebroadcast_advertisement` — applies `tq_product` and decrements TTL,
//!   returning `None` when TTL has reached zero.
//! - `encode_advertisement` / `decode_advertisement` — frame and validate OGMs
//!   with the eight-byte magic prefix `JQBATMNC`.

use jacquard_core::{NodeId, RatioPermille, Tick};
use serde::{Deserialize, Serialize};

use crate::scoring;

const GOSSIP_MAGIC: &[u8; 8] = b"JQBATMNC";

/// Default TTL assigned to OGMs at the originating node. Decremented at each
/// re-broadcasting hop; OGMs reaching TTL=0 are discarded and not re-broadcast.
pub(crate) const DEFAULT_OGM_TTL: u8 = 50;

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
    pub ttl: u8,
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
/// bincode-serialize the advertisement.
pub(crate) fn encode_advertisement(
    advertisement: &OriginatorAdvertisement,
) -> Result<Vec<u8>, bincode::Error> {
    let payload = bincode::serialize(advertisement)?;
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
    bincode::deserialize(&payload[GOSSIP_MAGIC.len()..]).ok()
}

/// Build the originator's own OGM for flooding.
///
/// TQ starts at 1000 (perfect quality); TTL starts at `DEFAULT_OGM_TTL`.
/// The `sequence` argument should be monotonically increasing (tick number is
/// an appropriate source).
pub(crate) fn local_advertisement(local_node_id: NodeId, sequence: u64) -> OriginatorAdvertisement {
    OriginatorAdvertisement {
        originator: local_node_id,
        sequence,
        tq: RatioPermille(1000),
        ttl: DEFAULT_OGM_TTL,
    }
}

/// Construct the re-broadcast form of a received OGM.
///
/// Applies `tq_product(local_link_tq, received.tq)` to encode this node's
/// path quality to the originator, then decrements TTL. Returns `None` if the
/// OGM's TTL is already zero (do not re-broadcast).
pub(crate) fn rebroadcast_advertisement(
    received: &OriginatorAdvertisement,
    local_link_tq: RatioPermille,
) -> Option<OriginatorAdvertisement> {
    if received.ttl == 0 {
        return None;
    }
    Some(OriginatorAdvertisement {
        originator: received.originator,
        sequence: received.sequence,
        tq: scoring::tq_product(local_link_tq, received.tq),
        ttl: received.ttl - 1,
    })
}
