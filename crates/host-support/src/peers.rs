//! Peer identity bookkeeping for host-owned transport integrations.
//!
//! Transport integrations often discover neighbors by their transport address
//! before the full `NodeId` is known. This module provides `PeerDirectory`, a
//! bidirectional map that tracks the lifecycle of a peer from initial hint to
//! resolved identity, and `PeerIdentityState`, which names each lifecycle
//! stage.
//!
//! `PeerIdentityState::Hint` holds a transport-supplied discovery hint (e.g. a
//! raw address or announcement payload) while the node identity is pending.
//! `PeerIdentityState::Resolved` records the canonical `NodeId` once the
//! host integration has confirmed the peer's identity through the control protocol.
//!
//! `PeerDirectory` maintains two concurrent indexes — by transport address and
//! by resolved `NodeId` — so hosts can look up peers in either direction.
//! Resolution and removal both clean up both indexes atomically to keep the
//! directory consistent under address migration and reconnection.

use std::collections::BTreeMap;

use jacquard_core::NodeId;
use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerIdentityState<Hint> {
    Hint(Hint),
    Resolved(NodeId),
}

#[derive(Clone, Debug)]
pub struct PeerDirectory<Addr, Hint> {
    by_addr: BTreeMap<Addr, PeerIdentityState<Hint>>,
    resolved_by_node: BTreeMap<NodeId, Addr>,
}

impl<Addr, Hint> Default for PeerDirectory<Addr, Hint> {
    fn default() -> Self {
        Self {
            by_addr: BTreeMap::new(),
            resolved_by_node: BTreeMap::new(),
        }
    }
}

impl<Addr, Hint> PeerDirectory<Addr, Hint>
where
    Addr: Clone + Ord,
{
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn identity_state(&self, addr: &Addr) -> Option<&PeerIdentityState<Hint>> {
        self.by_addr.get(addr)
    }

    #[must_use]
    pub fn address_for_resolved(&self, node_id: &NodeId) -> Option<&Addr> {
        self.resolved_by_node.get(node_id)
    }

    pub fn upsert_hint(&mut self, addr: Addr, hint: Hint) {
        if let Some(PeerIdentityState::Resolved(node_id)) = self.by_addr.get(&addr) {
            self.resolved_by_node.remove(node_id);
        }
        self.by_addr.insert(addr, PeerIdentityState::Hint(hint));
    }

    pub fn resolve(&mut self, addr: Addr, node_id: NodeId) {
        if let Some(previous_addr) = self.resolved_by_node.get(&node_id).cloned() {
            if previous_addr != addr {
                self.by_addr.remove(&previous_addr);
            }
        }

        if let Some(PeerIdentityState::Resolved(previous_node_id)) = self.by_addr.get(&addr) {
            if *previous_node_id != node_id {
                self.resolved_by_node.remove(previous_node_id);
            }
        }

        self.resolved_by_node.insert(node_id, addr.clone());
        self.by_addr
            .insert(addr, PeerIdentityState::Resolved(node_id));
    }

    pub fn remove(&mut self, addr: &Addr) -> Option<PeerIdentityState<Hint>> {
        let removed = self.by_addr.remove(addr);
        if let Some(PeerIdentityState::Resolved(node_id)) = removed.as_ref() {
            self.resolved_by_node.remove(node_id);
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::NodeId;

    use super::{PeerDirectory, PeerIdentityState};

    #[test]
    fn hint_insertion_is_visible_by_address() {
        let mut directory = PeerDirectory::new();
        directory.upsert_hint("peer-a", vec![1, 2, 3]);

        assert_eq!(
            directory.identity_state(&"peer-a"),
            Some(&PeerIdentityState::Hint(vec![1, 2, 3]))
        );
    }

    #[test]
    fn resolution_promotes_hint_to_node_identity() {
        let mut directory = PeerDirectory::new();
        let node_id = NodeId([7; 32]);
        directory.upsert_hint("peer-a", vec![1]);

        directory.resolve("peer-a", node_id);

        assert_eq!(
            directory.identity_state(&"peer-a"),
            Some(&PeerIdentityState::Resolved(node_id))
        );
        assert_eq!(directory.address_for_resolved(&node_id), Some(&"peer-a"));
    }

    #[test]
    fn overwrite_cleanup_removes_old_resolved_address() {
        let mut directory = PeerDirectory::new();
        let node_id = NodeId([9; 32]);
        directory.resolve("peer-a", node_id);
        directory.upsert_hint("peer-b", vec![2]);

        directory.resolve("peer-b", node_id);

        assert_eq!(directory.identity_state(&"peer-a"), None);
        assert_eq!(
            directory.identity_state(&"peer-b"),
            Some(&PeerIdentityState::Resolved(node_id))
        );
        assert_eq!(directory.address_for_resolved(&node_id), Some(&"peer-b"));
    }

    #[test]
    fn removal_cleanup_clears_reverse_lookup() {
        let mut directory: PeerDirectory<&str, Vec<u8>> = PeerDirectory::new();
        let node_id = NodeId([3; 32]);
        directory.resolve("peer-a", node_id);

        let removed = directory.remove(&"peer-a");

        assert_eq!(removed, Some(PeerIdentityState::Resolved(node_id)));
        assert_eq!(directory.address_for_resolved(&node_id), None);
    }
}
