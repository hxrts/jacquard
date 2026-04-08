//! `InMemoryRetentionStore`, the shared `RetentionStore` adapter used by
//! tests. Buffers opaque deferred-delivery payloads in a `BTreeMap` keyed
//! by content id so partition-mode flush paths stay deterministic.

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
