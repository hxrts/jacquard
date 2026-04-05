use jacquard_macros::purity;

#[purity(read_only)]
pub trait InvalidReadOnlyTrait {
    fn consume(self);
}

fn main() {}
