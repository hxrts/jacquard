//! Babel update encoding: destination, router_id, seqno, and metric.
//!
//! Unlike the classic BATMAN engine, `BabelUpdate` carries no TTL field.
//! Propagation depth is controlled by feasibility/freshness rather than a
//! hop-count limit: a route is usable only while its `observed_at_tick` is
//! within `decay_window.stale_after_ticks` of the current tick. Once stale,
//! the entry is pruned and the route disappears naturally.
//!
//! Only the selected (best) route per destination is re-advertised by relays.
//! Unlike batman-classic which re-broadcasts all received advertisements, Babel
//! nodes forward only the winner — the route with the lowest finite metric.
//! This reduces overhead and eliminates the need for a TTL-bounded flood
//! to prevent routing loops.
//!
//! Key functions:
//! - `originated_update` — builds the local node's own update (metric=0).
//! - `encode_update` / `decode_update` — frame and validate updates with the
//!   eight-byte magic prefix `JQBABEL.`.

use jacquard_core::NodeId;
use serde::{Deserialize, Serialize};

pub(crate) const GOSSIP_MAGIC: &[u8; 8] = b"JQBABEL.";

/// Metric value representing an unreachable or unusable route.
pub(crate) const BABEL_INFINITY: u16 = 0xFFFF;

/// A Babel route update (OGM equivalent).
///
/// Carries no TTL. The originator initialises metric to 0; each relay adds the
/// local link cost before re-advertising the selected route. Downstream nodes
/// read path quality directly from the received metric.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BabelUpdate {
    pub destination: NodeId,
    pub router_id: NodeId,
    pub seqno: u16,
    pub metric: u16,
}

/// Build the local node's originated update. metric=0 (perfect, no cost yet).
pub(crate) fn originated_update(local_node_id: NodeId, seqno: u16) -> BabelUpdate {
    BabelUpdate {
        destination: local_node_id,
        router_id: local_node_id,
        seqno,
        metric: 0,
    }
}

/// Encode an update for transmission: prepend the eight-byte magic prefix, then
/// postcard-serialize the update.
pub(crate) fn encode_update(update: &BabelUpdate) -> Result<Vec<u8>, postcard::Error> {
    let payload = postcard::to_allocvec(update)?;
    let mut framed = Vec::with_capacity(GOSSIP_MAGIC.len() + payload.len());
    framed.extend_from_slice(GOSSIP_MAGIC);
    framed.extend_from_slice(&payload);
    Ok(framed)
}

/// Validate the magic prefix and deserialize an update from a raw payload.
pub(crate) fn decode_update(payload: &[u8]) -> Option<BabelUpdate> {
    if !payload.starts_with(GOSSIP_MAGIC) {
        return None;
    }
    postcard::from_bytes(&payload[GOSSIP_MAGIC.len()..]).ok()
}
