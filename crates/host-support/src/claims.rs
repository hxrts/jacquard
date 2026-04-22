//! In-flight claim guards for host-side concurrent operations.
//!
//! Host integrations frequently need to ensure that at most one concurrent
//! operation targets a given key — for example, a single in-flight handshake
//! per peer address, or a single active resolution per node identity. This
//! module provides `PendingClaims`, a cloneable shared claim set, and
//! `ClaimGuard`, an RAII guard that releases the claim on drop.
//!
//! `PendingClaims::try_claim` inserts a key and returns a `ClaimGuard` on
//! success. If the key is already held, `ClaimRejected` is returned so the
//! caller can skip the duplicate work. When the `ClaimGuard` is dropped —
//! whether normally or on unwinding — the key is removed from the shared set,
//! allowing the next attempt to succeed.
//!
//! The key type is generic over any `Clone + Ord` value so hosts can use
//! transport addresses, node identifiers, or composite keys without wrapping.

use alloc::collections::BTreeSet;
use core::fmt;

#[cfg(not(feature = "std"))]
use alloc::rc::Rc;
#[cfg(not(feature = "std"))]
use core::cell::RefCell;
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimRejected;

impl fmt::Display for ClaimRejected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("claim is already held")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ClaimRejected {}

#[cfg(feature = "std")]
type SharedClaims<Key> = Arc<Mutex<BTreeSet<Key>>>;

#[cfg(not(feature = "std"))]
type SharedClaims<Key> = Rc<RefCell<BTreeSet<Key>>>;

#[derive(Clone)]
pub struct PendingClaims<Key: Ord> {
    claimed: SharedClaims<Key>,
}

impl<Key: Ord> Default for PendingClaims<Key> {
    fn default() -> Self {
        Self {
            claimed: new_claim_set(),
        }
    }
}

pub struct ClaimGuard<Key: Ord> {
    claimed: SharedClaims<Key>,
    key: Option<Key>,
}

#[cfg(feature = "std")]
fn new_claim_set<Key>() -> SharedClaims<Key> {
    Arc::new(Mutex::new(BTreeSet::new()))
}

#[cfg(not(feature = "std"))]
fn new_claim_set<Key>() -> SharedClaims<Key> {
    Rc::new(RefCell::new(BTreeSet::new()))
}

#[cfg(feature = "std")]
fn with_claims<Key, Output>(
    claims: &SharedClaims<Key>,
    operation: impl FnOnce(&mut BTreeSet<Key>) -> Output,
) -> Output {
    let mut guard = claims
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&mut guard)
}

#[cfg(not(feature = "std"))]
fn with_claims<Key, Output>(
    claims: &SharedClaims<Key>,
    operation: impl FnOnce(&mut BTreeSet<Key>) -> Output,
) -> Output {
    let mut guard = claims.borrow_mut();
    operation(&mut guard)
}

impl<Key> PendingClaims<Key>
where
    Key: Clone + Ord,
{
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_claim(&self, key: Key) -> Result<ClaimGuard<Key>, ClaimRejected> {
        if !with_claims(&self.claimed, |claimed| claimed.insert(key.clone())) {
            return Err(ClaimRejected);
        }
        Ok(ClaimGuard {
            claimed: self.claimed.clone(),
            key: Some(key),
        })
    }

    #[must_use]
    pub fn contains(&self, key: &Key) -> bool {
        with_claims(&self.claimed, |claimed| claimed.contains(key))
    }
}

impl<Key: Ord> ClaimGuard<Key> {
    #[must_use]
    pub fn key(&self) -> &Key {
        self.key.as_ref().expect("claim guard key")
    }
}

impl<Key: Ord> Drop for ClaimGuard<Key> {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            with_claims(&self.claimed, |claimed| claimed.remove(&key));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClaimRejected, PendingClaims};

    #[test]
    fn duplicate_claims_are_rejected() {
        let claims = PendingClaims::new();
        let _guard = claims.try_claim("peer-a").expect("first claim");
        assert_eq!(claims.try_claim("peer-a").err(), Some(ClaimRejected));
    }

    #[test]
    fn dropped_guards_release_claims() {
        let claims = PendingClaims::new();
        let guard = claims.try_claim("peer-a").expect("claim");
        assert!(claims.contains(&"peer-a"));
        assert_eq!(guard.key(), &"peer-a");

        drop(guard);

        assert!(!claims.contains(&"peer-a"));
    }
}
