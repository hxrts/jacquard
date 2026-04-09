//! Concrete handler markers for Jacquard effect vocabularies.
//!
//! A handler is a concrete runtime implementation of one effect trait. It
//! performs the requested operation for one abstract effect vocabulary.
//! Handlers should stay narrow and infrastructure-oriented; they must not
//! become owners of canonical routing truth or long-lived orchestration state.
//!
//! Key type exported from this module:
//! - [`EffectHandler<E>`] — sealed marker trait satisfied by any type that
//!   carries an `#[effect_handler]` impl for the effect vocabulary `E`.
//!
//! The sealing mechanism relies on `HandlerDefinition<E>` which the
//! `#[effect_handler]` proc-macro emits automatically. External crates
//! cannot implement `EffectHandler` without going through the macro, keeping
//! the effect-handler boundary auditable and explicit.

use jacquard_macros::purity;

use crate::{sealed, Effect};

#[purity(effectful)]
/// Marker trait for concrete implementations of one effect vocabulary.
///
/// The `Sealed` super-trait is intentionally `pub(crate)` — external crates
/// implement `EffectHandler` only through the `#[effect_handler]` proc
/// macro, which attaches the required `HandlerDefinition` bound.
#[allow(private_bounds)]
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
