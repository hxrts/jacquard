//! Concrete handler markers for Jacquard effect vocabularies.
//!
//! This module re-exports the generic toolkit support trait so Jacquard keeps a
//! stable public API while the effect-handler machinery lives in the toolkit.

pub use rust_toolkit_effects::EffectHandler;

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
