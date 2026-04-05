//! Concrete handler markers for Jacquard effect vocabularies.
//!
//! A handler is a concrete implementation of an effect trait. It performs the
//! requested operation for one abstract effect vocabulary. Handlers should stay
//! narrow and infrastructure-oriented. They should not become owners of
//! canonical routing truth or long-lived orchestration state.

use crate::Effect;

mod sealed {
    pub trait Sealed {}

    impl<T> Sealed for T where T: ?Sized + Send + Sync + 'static {}
}

/// Marker trait for concrete implementations of one effect vocabulary.
pub trait EffectHandler<E>: sealed::Sealed + Send + Sync + 'static
where
    E: ?Sized + Effect,
    Self: crate::__private::HandlerDefinition<E>,
{
}

impl<T, E> EffectHandler<E> for T
where
    T: ?Sized + Send + Sync + 'static,
    E: ?Sized + Effect,
    T: crate::__private::HandlerDefinition<E>,
{
}

#[cfg(test)]
mod tests {
    use jacquard_core::{OrderStamp, Tick};

    use crate::{effect_handler, EffectHandler, OrderEffects, TimeEffects};

    struct DummyHandler;

    #[effect_handler]
    impl TimeEffects for DummyHandler {
        fn now_tick(&self) -> Tick {
            Tick(7)
        }
    }

    #[effect_handler]
    impl OrderEffects for DummyHandler {
        fn next_order_stamp(&mut self) -> OrderStamp {
            OrderStamp(9)
        }
    }

    fn assert_handler<H, E>()
    where
        H: EffectHandler<E>,
        E: ?Sized + crate::Effect,
    {
    }

    #[test]
    fn handlers_can_be_checked_against_effect_vocabularies() {
        assert_handler::<DummyHandler, dyn TimeEffects>();
        assert_handler::<DummyHandler, dyn OrderEffects>();
    }
}
