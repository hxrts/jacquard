//! In-memory runtime-effect adapters for reference composition and tests.
//!
//! This module provides a deterministic implementation of the shared runtime
//! effect traits used by router and engine:
//! - time
//! - ordering
//! - key-value storage
//! - route-event logging
//!
//! It is intentionally reference-only and in-memory. It exists to support
//! tests, examples, and the reference client, not to model a production host
//! runtime.

use std::collections::BTreeMap;

use jacquard_core::{OrderStamp, RouteEventLogError, RouteEventStamped, StorageError, Tick};
use jacquard_traits::{
    effect_handler, OrderEffects, RouteEventLogEffects, StorageEffects, TimeEffects,
};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct InMemoryRuntimeEffects {
    pub now: Tick,
    pub next_order: u64,
    pub storage: BTreeMap<Vec<u8>, Vec<u8>>,
    pub store_bytes_call_count: u32,
    pub events: Vec<RouteEventStamped>,
    pub fail_store_bytes: bool,
    pub fail_record_route_event: bool,
}

#[effect_handler]
impl TimeEffects for InMemoryRuntimeEffects {
    fn now_tick(&self) -> Tick {
        self.now
    }
}

#[effect_handler]
impl OrderEffects for InMemoryRuntimeEffects {
    fn next_order_stamp(&mut self) -> OrderStamp {
        self.next_order = self.next_order.saturating_add(1);
        OrderStamp(self.next_order)
    }
}

#[effect_handler]
impl StorageEffects for InMemoryRuntimeEffects {
    fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.storage.get(key).cloned())
    }

    fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        if self.fail_store_bytes {
            return Err(StorageError::Unavailable);
        }
        self.store_bytes_call_count = self.store_bytes_call_count.saturating_add(1);
        self.storage.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError> {
        self.storage.remove(key);
        Ok(())
    }
}

#[effect_handler]
impl RouteEventLogEffects for InMemoryRuntimeEffects {
    fn record_route_event(&mut self, event: RouteEventStamped) -> Result<(), RouteEventLogError> {
        if self.fail_record_route_event {
            return Err(RouteEventLogError::Unavailable);
        }
        self.events.push(event);
        Ok(())
    }
}
