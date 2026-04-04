//! Time and ordering effect traits for deterministic routing.

use contour_core::{OrderStamp, Tick};

pub trait TimeEffects {
    fn now_tick(&self) -> Tick;
}

pub trait OrderEffects {
    fn next_order_stamp(&mut self) -> OrderStamp;
}
