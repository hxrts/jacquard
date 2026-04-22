//! Effect vocabulary traits for deterministic routing.
//!
//! In Jacquard, an effect trait names a narrow abstract runtime capability that
//! the pure routing core may request. It describes the operation vocabulary
//! without owning supervision, background orchestration, or canonical routing
//! truth. Concrete implementations are attached to handler structs via the
//! `#[effect_handler]` proc-macro attribute and registered against these
//! traits.
//!
//! Key traits exported from this module:
//! - [`TimeEffects`] — read-only access to monotonic logical time (`Tick`).
//! - [`OrderEffects`] — deterministic `OrderStamp` allocation.
//! - [`StorageEffects`] — opaque byte persistence (load, store, remove).
//! - [`RouteEventLogEffects`] — append-only stamped route-event log.
//! - [`TransportSenderEffects`] — synchronous byte-send capability over a named
//!   endpoint; ingress supervision lives on `TransportDriver` instead.
//! - [`RetentionStore`] — deferred-delivery payload retention by content id.
//! - [`RoutingRuntimeEffects`] — blanket aggregate for the minimal required
//!   set.
//! - [`Effect`] — sealed marker that every effect trait automatically
//!   satisfies.

use jacquard_core::{
    Blake3Digest, ContentId, OrderStamp, RetentionError, RouteEventLogError, RouteEventStamped,
    StorageError, Tick, TransportDeliveryIntent, TransportError,
};
use jacquard_macros::{effect_trait, purity};
pub use rust_toolkit_effects::Effect;

#[effect_trait]
/// Read-only runtime capability for monotonic logical time.
///
/// Effectful runtime boundary.
pub trait TimeEffects {
    #[must_use]
    fn now_tick(&self) -> Tick;
}

#[effect_trait]
/// Runtime capability for deterministic order-stamp allocation.
///
/// Effectful runtime boundary.
pub trait OrderEffects {
    #[must_use]
    fn next_order_stamp(&mut self) -> OrderStamp;
}

#[effect_trait]
/// Runtime persistence boundary for opaque bytes.
///
/// Effectful runtime boundary.
pub trait StorageEffects {
    must_use_evidence!("load_bytes", "storage errors";
        fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    );

    #[must_use = "unchecked store_bytes result silently discards write failures"]
    fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;

    #[must_use = "unchecked remove_bytes result silently discards deletion failures"]
    fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError>;
}

#[effect_trait]
/// Runtime route-event log boundary for replay-visible stamped events.
///
/// Effectful runtime boundary.
pub trait RouteEventLogEffects {
    #[must_use = "unhandled record_route_event result silently loses a route audit event"]
    fn record_route_event(&mut self, event: RouteEventStamped) -> Result<(), RouteEventLogError>;
}

#[effect_trait]
/// Runtime transport-send capability. This carries bytes only.
///
/// Effectful runtime boundary — connectivity surface. Host-owned ingress
/// supervision lives on `TransportDriver`, not in the effect vocabulary.
pub trait TransportSenderEffects {
    #[must_use = "unchecked send_transport result silently discards send failures"]
    fn send_transport(
        &mut self,
        endpoint: &jacquard_core::LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError>;

    #[must_use = "unchecked send_transport_to result silently discards send failures"]
    fn send_transport_to(
        &mut self,
        intent: &TransportDeliveryIntent,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        match intent {
            TransportDeliveryIntent::Unicast { endpoint } => self.send_transport(endpoint, payload),
            TransportDeliveryIntent::Multicast { .. }
            | TransportDeliveryIntent::Broadcast { .. } => Err(TransportError::Rejected),
        }
    }
}

#[effect_trait]
/// Runtime boundary for opaque deferred-delivery payload storage.
///
/// Effectful runtime boundary.
pub trait RetentionStore {
    #[must_use = "unchecked retain_payload result silently discards retention failures"]
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError>;

    must_use_evidence!("take_retained_payload", "the held payload";
        fn take_retained_payload(
            &mut self,
            object_id: &ContentId<Blake3Digest>,
        ) -> Result<Option<Vec<u8>>, RetentionError>;
    );

    must_use_evidence!("contains_retained_payload", "storage errors";
        fn contains_retained_payload(
            &self,
            object_id: &ContentId<Blake3Digest>,
        ) -> Result<bool, RetentionError>;
    );
}

#[purity(effectful)]
/// Aggregate marker for runtimes that provide the current minimal effect set.
///
/// Effectful runtime boundary.
pub trait RoutingRuntimeEffects:
    TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects + TransportSenderEffects
{
}

impl<T> RoutingRuntimeEffects for T where
    T: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects + TransportSenderEffects
{
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        BroadcastDomainId, ByteCount, EndpointLocator, LinkEndpoint, MulticastGroupId, NodeId,
        OrderStamp, RouteEventLogError, RouteEventStamped, StorageError, Tick,
        TransportDeliveryIntent, TransportError, TransportKind,
    };

    use super::{
        Effect, OrderEffects, RouteEventLogEffects, RoutingRuntimeEffects, StorageEffects,
        TimeEffects, TransportSenderEffects,
    };
    use crate::effect_handler;

    struct DummyRuntime;

    #[effect_handler]
    impl TimeEffects for DummyRuntime {
        fn now_tick(&self) -> Tick {
            Tick(1)
        }
    }

    #[effect_handler]
    impl OrderEffects for DummyRuntime {
        fn next_order_stamp(&mut self) -> OrderStamp {
            OrderStamp(2)
        }
    }

    #[effect_handler]
    impl StorageEffects for DummyRuntime {
        fn load_bytes(&self, _key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
            Ok(None)
        }

        fn store_bytes(&mut self, _key: &[u8], _value: &[u8]) -> Result<(), StorageError> {
            Ok(())
        }

        fn remove_bytes(&mut self, _key: &[u8]) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[effect_handler]
    impl RouteEventLogEffects for DummyRuntime {
        fn record_route_event(
            &mut self,
            _event: RouteEventStamped,
        ) -> Result<(), RouteEventLogError> {
            Ok(())
        }
    }

    #[effect_handler]
    impl TransportSenderEffects for DummyRuntime {
        fn send_transport(
            &mut self,
            _endpoint: &jacquard_core::LinkEndpoint,
            _payload: &[u8],
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }

    fn assert_effect<E>()
    where
        E: ?Sized + Effect,
    {
    }

    fn assert_runtime<R>()
    where
        R: RoutingRuntimeEffects,
    {
    }

    #[test]
    fn effect_traits_participate_in_the_effect_marker() {
        assert_effect::<dyn TimeEffects>();
        assert_effect::<dyn OrderEffects>();
        assert_effect::<dyn StorageEffects>();
        assert_effect::<dyn RouteEventLogEffects>();
        assert_effect::<dyn TransportSenderEffects>();
    }

    #[test]
    fn aggregate_runtime_effects_track_supported_effect_sets() {
        assert_runtime::<DummyRuntime>();
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        LinkEndpoint::new(
            TransportKind::BleGatt,
            EndpointLocator::Opaque(vec![byte]),
            ByteCount(128),
        )
    }

    #[test]
    fn default_send_intent_rejects_non_unicast_delivery() {
        let mut runtime = DummyRuntime;
        let multicast = TransportDeliveryIntent::Multicast {
            endpoint: endpoint(1),
            group_id: MulticastGroupId([2; 16]),
            receivers: vec![NodeId([3; 32])],
        };
        let broadcast = TransportDeliveryIntent::Broadcast {
            endpoint: endpoint(4),
            domain_id: BroadcastDomainId([5; 16]),
        };

        assert_eq!(
            runtime.send_transport_to(&multicast, b"frame"),
            Err(TransportError::Rejected)
        );
        assert_eq!(
            runtime.send_transport_to(&broadcast, b"frame"),
            Err(TransportError::Rejected)
        );
    }
}
