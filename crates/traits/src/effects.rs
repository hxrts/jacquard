//! Effect vocabulary traits for deterministic routing.
//!
//! In Contour, an effect trait names a narrow abstract runtime capability that
//! the pure routing core may request. It describes the operation vocabulary.
//! It does not own supervision, background orchestration, or canonical routing
//! truth. Concrete handlers live in the separate `handler` module.

use contour_core::{OrderStamp, Tick};
use contour_macros::effect_trait;

mod sealed {
    pub trait Sealed {}

    impl<T> Sealed for T where T: ?Sized + crate::__private::EffectDefinition + Send + Sync + 'static {}
}

/// Marker trait for the abstract runtime effect vocabulary.
///
/// Every effect trait in Contour should extend this trait so the effect surface
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

/// Aggregate marker for runtimes that provide the current minimal effect set.
pub trait RoutingRuntimeEffects: TimeEffects + OrderEffects {}

impl<T> RoutingRuntimeEffects for T where T: TimeEffects + OrderEffects {}

#[cfg(test)]
mod tests {
    use contour_core::{OrderStamp, Tick};

    use super::{Effect, OrderEffects, RoutingRuntimeEffects, TimeEffects};
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
    }

    #[test]
    fn aggregate_runtime_effects_track_supported_effect_sets() {
        assert_runtime::<DummyRuntime>();
    }
}
