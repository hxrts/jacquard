//! OLSRv2-class control message encoding.
//!
//! The first pass carries two message kinds:
//! - `HelloMessage` for one-hop / two-hop discovery and MPR selection signaling
//! - `TcMessage` for flooded topology advertisement

use jacquard_core::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HelloMessage {
    pub originator: NodeId,
    pub sequence: u64,
    pub symmetric_neighbors: Vec<NodeId>,
    pub mprs: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TcMessage {
    pub originator: NodeId,
    pub sequence: u64,
    pub advertised_neighbors: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OlsrMessage {
    Hello(HelloMessage),
    Tc(TcMessage),
}

pub(crate) fn encode_message(message: &OlsrMessage) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(message)
}

pub(crate) fn decode_message(payload: &[u8]) -> Option<OlsrMessage> {
    bincode::deserialize(payload).ok()
}

pub(crate) fn originated_hello(
    originator: NodeId,
    sequence: u64,
    symmetric_neighbors: impl IntoIterator<Item = NodeId>,
    mprs: impl IntoIterator<Item = NodeId>,
) -> OlsrMessage {
    let mut symmetric_neighbors: Vec<NodeId> = symmetric_neighbors.into_iter().collect();
    let mut mprs: Vec<NodeId> = mprs.into_iter().collect();
    symmetric_neighbors.sort();
    mprs.sort();
    OlsrMessage::Hello(HelloMessage {
        originator,
        sequence,
        symmetric_neighbors,
        mprs,
    })
}

pub(crate) fn originated_tc(
    originator: NodeId,
    sequence: u64,
    advertised_neighbors: impl IntoIterator<Item = NodeId>,
) -> OlsrMessage {
    let mut advertised_neighbors: Vec<NodeId> = advertised_neighbors.into_iter().collect();
    advertised_neighbors.sort();
    OlsrMessage::Tc(TcMessage {
        originator,
        sequence,
        advertised_neighbors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    #[test]
    fn hello_round_trips_through_bincode() {
        let message = originated_hello(node(1), 7, [node(2), node(3)], [node(2)]);
        let payload = encode_message(&message).expect("encode hello");
        let decoded = decode_message(&payload).expect("decode hello");

        assert_eq!(decoded, message);
    }

    #[test]
    fn tc_round_trips_through_bincode() {
        let message = originated_tc(node(4), 9, [node(2), node(5), node(3)]);
        let payload = encode_message(&message).expect("encode tc");
        let decoded = decode_message(&payload).expect("decode tc");

        assert_eq!(decoded, message);
    }
}
