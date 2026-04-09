//! `InMemoryRetentionStore`, the shared `RetentionStore` adapter used by
//! tests and the reference client.
//!
//! This module provides a deterministic in-memory implementation of the
//! `RetentionStore` trait from `jacquard-traits`. Payloads are buffered in a
//! `BTreeMap` keyed by `ContentId<Blake3Digest>`, which gives deterministic
//! iteration order across all callers.
//!
//! The store supports the three operations that `RetentionStore` requires:
//! retaining a payload, checking whether a payload is present, and taking
//! (removing and returning) a retained payload. It does not impose size limits
//! or eviction policy; capacity management belongs to the host runtime.
//!
//! Intended for use in partition-mode flush tests, reference composition, and
//! any scenario that needs stable deferred-delivery buffering without a
//! persistent backend.

use std::collections::BTreeMap;

use jacquard_core::{Blake3Digest, ContentId, RetentionError};
use jacquard_traits::{effect_handler, RetentionStore};

#[derive(Default)]
pub struct InMemoryRetentionStore {
    pub payloads: BTreeMap<ContentId<Blake3Digest>, Vec<u8>>,
}

#[effect_handler]
impl RetentionStore for InMemoryRetentionStore {
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError> {
        self.payloads.insert(object_id, payload);
        Ok(())
    }

    fn take_retained_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RetentionError> {
        Ok(self.payloads.remove(object_id))
    }

    fn contains_retained_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, RetentionError> {
        Ok(self.payloads.contains_key(object_id))
    }
}
