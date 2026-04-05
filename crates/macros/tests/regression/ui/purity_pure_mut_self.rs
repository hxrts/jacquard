use jacquard_macros::purity;

#[purity(pure)]
pub trait InvalidPureTrait {
    fn mutate(&mut self);
}

fn main() {}
