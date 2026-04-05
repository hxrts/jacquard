//! Effect vocabulary traits for deterministic routing.
//!
//! In Jacquard, an effect trait names a narrow abstract runtime capability that
//! the pure routing core may request. It describes the operation vocabulary.
//! It does not own supervision, background orchestration, or canonical routing
//! truth. Concrete handlers live in the separate `handler` module.

use jacquard_core::{
    AuditError, Blake3Digest, OrderStamp, RoutingAuditEvent, StorageError, Tick, TransportError,
    TransportIngressEvent,
};
use jacquard_macros::effect_trait;

mod sealed {
    pub trait Sealed {}

    impl<T> Sealed for T where T: ?Sized + crate::__private::EffectDefinition + Send + Sync + 'static {}
}

/// Marker trait for the abstract runtime effect vocabulary.
///
/// Every effect trait in Jacquard should extend this trait so the effect surface
/// stays narrow, object-safe, and clearly separated from concrete handlers.
pub trait Effect: sealed::Sealed + Send + Sync + 'static {}

impl<T> Effect for T where T: ?Sized + crate::__private::EffectDefinition + Send + Sync + 'static {}

#[effect_trait]
pub trait TimeEffects {
    fn now_tick(&self) -> Tick;
}

#[effect_trait]
pub trait OrderEffects {
    fn next_order_stamp(&mut self) -> OrderStamp;
}

#[effect_trait]
pub trait HashEffects {
    fn hash_bytes(&self, input: &[u8]) -> Blake3Digest;

    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Blake3Digest;
}

#[effect_trait]
pub trait StorageEffects {
    fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;

    fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;

    fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError>;
}

#[effect_trait]
pub trait AuditEffects {
    fn emit_audit(&mut self, event: RoutingAuditEvent) -> Result<(), AuditError>;
}

#[effect_trait]
pub trait TransportEffects {
    fn send_transport(
        &mut self,
        endpoint: &jacquard_core::LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError>;

    fn poll_transport(&mut self) -> Result<Vec<TransportIngressEvent>, TransportError>;
}

/// Aggregate marker for runtimes that provide the current minimal effect set.
pub trait RoutingRuntimeEffects:
    TimeEffects + OrderEffects + HashEffects + StorageEffects + AuditEffects + TransportEffects
{
}

impl<T> RoutingRuntimeEffects for T where
    T: TimeEffects + OrderEffects + HashEffects + StorageEffects + AuditEffects + TransportEffects
{
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        AuditError, Blake3Digest, OrderStamp, RoutingAuditEvent, StorageError, Tick,
        TransportError, TransportIngressEvent,
    };

    use super::{
        AuditEffects, Effect, HashEffects, OrderEffects, RoutingRuntimeEffects, StorageEffects,
        TimeEffects, TransportEffects,
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
    impl HashEffects for DummyRuntime {
        fn hash_bytes(&self, _input: &[u8]) -> Blake3Digest {
            Blake3Digest([3; 32])
        }

        fn hash_tagged(&self, _domain: &[u8], _input: &[u8]) -> Blake3Digest {
            Blake3Digest([4; 32])
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
    impl AuditEffects for DummyRuntime {
        fn emit_audit(&mut self, _event: RoutingAuditEvent) -> Result<(), AuditError> {
            Ok(())
        }
    }

    #[effect_handler]
    impl TransportEffects for DummyRuntime {
        fn send_transport(
            &mut self,
            _endpoint: &jacquard_core::LinkEndpoint,
            _payload: &[u8],
        ) -> Result<(), TransportError> {
            Ok(())
        }

        fn poll_transport(&mut self) -> Result<Vec<TransportIngressEvent>, TransportError> {
            Ok(Vec::new())
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
        assert_effect::<dyn HashEffects>();
        assert_effect::<dyn StorageEffects>();
        assert_effect::<dyn AuditEffects>();
        assert_effect::<dyn TransportEffects>();
    }

    #[test]
    fn aggregate_runtime_effects_track_supported_effect_sets() {
        assert_runtime::<DummyRuntime>();
    }
}
