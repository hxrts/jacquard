//! Concrete handler markers for Jacquard effect vocabularies.
//!
//! These marker traits are the runtime-free support surface used by the
//! effect-handler proc macros.

use jacquard_macros::purity;

mod sealed {
    pub trait EffectSealed {}

    impl<T> EffectSealed for T where
        T: ?Sized + crate::__private::EffectDefinition + Send + Sync + 'static
    {
    }
}

/// Marker trait for abstract effect vocabularies.
#[purity(pure)]
pub trait Effect: sealed::EffectSealed + Send + Sync + 'static {}

impl<T> Effect for T where T: ?Sized + crate::__private::EffectDefinition + Send + Sync + 'static {}

/// Marker trait for concrete handlers of one effect vocabulary.
#[purity(pure)]
pub trait EffectHandler<E>: Send + Sync + 'static
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
