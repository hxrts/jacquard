//! Router-owned persistence and publication sequencing helpers.
//!
//! Control flow: middleware hands this module a fully formed canonical route
//! snapshot plus its commitments. The adapter persists that snapshot under a
//! router-scoped namespace, records the replay-visible route event, and only
//! then lets middleware expose the route through the live canonical tables.
//! Recovery walks the same router-owned registry and asks the selected engine
//! to restore only its opaque private runtime payloads.

use std::collections::BTreeSet;

use jacquard_core::{
    MaterializedRoute, NodeId, RouteCommitment, RouteError, RouteEvent,
    RouteEventStamped, RouteId, RouteRuntimeError,
};
use jacquard_traits::{
    OrderEffects, RouteEventLogEffects, StorageEffects, TimeEffects,
};

/// Extension trait for converting storage errors into
/// `RouteError::Runtime(Invalidated)`.
trait StorageResultExt<T> {
    fn storage_invalid(self) -> Result<T, RouteError>;
}

impl<T, E> StorageResultExt<T> for Result<T, E> {
    fn storage_invalid(self) -> Result<T, RouteError> {
        self.map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))
    }
}
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RouterCheckpointRecord {
    pub(crate) route: MaterializedRoute,
    pub(crate) commitments: Vec<RouteCommitment>,
}

pub(crate) trait RouterRuntimeEffects:
    TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects
{
}

impl<T> RouterRuntimeEffects for T where
    T: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects
{
}

pub(crate) struct RouterRuntimeAdapter<'a, Effects> {
    local_node_id: NodeId,
    effects: &'a mut Effects,
}

impl<'a, Effects> RouterRuntimeAdapter<'a, Effects>
where
    Effects: RouterRuntimeEffects,
{
    pub(crate) fn new(local_node_id: NodeId, effects: &'a mut Effects) -> Self {
        Self { local_node_id, effects }
    }

    pub(crate) fn persist_route(
        &mut self,
        record: &RouterCheckpointRecord,
    ) -> Result<(), RouteError> {
        let route_key = route_storage_key(
            &self.local_node_id,
            &record.route.identity.stamp.route_id,
        );
        let route_bytes = bincode::serialize(record).storage_invalid()?;
        self.effects
            .store_bytes(&route_key, &route_bytes)
            .storage_invalid()?;

        let mut registry = self.load_route_registry()?;
        registry.insert(record.route.identity.stamp.route_id);
        if let Err(error) = self.store_route_registry(&registry) {
            let _ = self.effects.remove_bytes(&route_key);
            return Err(error);
        }

        Ok(())
    }

    pub(crate) fn remove_route(
        &mut self,
        route_id: &RouteId,
    ) -> Result<(), RouteError> {
        let route_key = route_storage_key(&self.local_node_id, route_id);
        self.effects.remove_bytes(&route_key).storage_invalid()?;
        let mut registry = self.load_route_registry()?;
        registry.remove(route_id);
        self.store_route_registry(&registry)
    }

    pub(crate) fn load_routes(
        &mut self,
    ) -> Result<Vec<(RouteId, RouterCheckpointRecord)>, RouteError> {
        let registry = self.load_route_registry()?;
        let mut recovered = Vec::new();
        let mut pruned_registry = registry.clone();
        for route_id in registry {
            let route_key = route_storage_key(&self.local_node_id, &route_id);
            let Some(bytes) = self.effects.load_bytes(&route_key).storage_invalid()?
            else {
                pruned_registry.remove(&route_id);
                continue;
            };
            let record = bincode::deserialize::<RouterCheckpointRecord>(&bytes)
                .storage_invalid()?;
            recovered.push((route_id, record));
        }
        if pruned_registry != self.load_route_registry()? {
            self.store_route_registry(&pruned_registry)?;
        }
        Ok(recovered)
    }

    pub(crate) fn record_route_event(
        &mut self,
        event: RouteEvent,
    ) -> Result<(), RouteError> {
        let order_stamp = self.effects.next_order_stamp();
        let emitted_at_tick = self.effects.now_tick();
        self.effects
            .record_route_event(RouteEventStamped {
                order_stamp,
                emitted_at_tick,
                event,
            })
            .storage_invalid()
    }

    fn load_route_registry(&mut self) -> Result<BTreeSet<RouteId>, RouteError> {
        let registry_key = route_registry_storage_key(&self.local_node_id);
        let Some(bytes) = self.effects.load_bytes(&registry_key).storage_invalid()?
        else {
            return Ok(BTreeSet::new());
        };
        bincode::deserialize(&bytes).storage_invalid()
    }

    fn store_route_registry(
        &mut self,
        registry: &BTreeSet<RouteId>,
    ) -> Result<(), RouteError> {
        let registry_key = route_registry_storage_key(&self.local_node_id);
        let registry_bytes = bincode::serialize(registry).storage_invalid()?;
        self.effects
            .store_bytes(&registry_key, &registry_bytes)
            .storage_invalid()
    }
}

fn route_registry_storage_key(local_node_id: &NodeId) -> Vec<u8> {
    let mut key = b"router/".to_vec();
    key.extend_from_slice(&local_node_id.0);
    key.extend_from_slice(b"/routes");
    key
}

fn route_storage_key(local_node_id: &NodeId, route_id: &RouteId) -> Vec<u8> {
    let mut key = b"router/".to_vec();
    key.extend_from_slice(&local_node_id.0);
    key.extend_from_slice(b"/route/");
    key.extend_from_slice(&route_id.0);
    key
}
