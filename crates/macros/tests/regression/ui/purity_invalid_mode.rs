use jacquard_macros::purity;

#[purity(side_effecty)]
pub trait InvalidPurityMode {
    fn inspect(&self);
}

fn main() {}
