use contour_traits::{TimeEffects, contour_core::Tick};

struct BadHandler;

impl TimeEffects for BadHandler {
    fn now_tick(&self) -> Tick {
        Tick(1)
    }
}

fn main() {}
