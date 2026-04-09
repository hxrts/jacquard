//! Regression test: tagged hashing must separate ambiguous domain/payload
//! pairs.
//!
//! The `Blake3Hashing::hash_tagged` method length-prefixes the domain tag so
//! that a (domain="ab", payload="c") pair cannot collide with
//! (domain="a", payload="bc"). This test guards that invariant from regressing
//! if the domain-tag encoding ever changes.

use jacquard_traits::{Blake3Hashing, Hashing};

#[test]
fn tagged_hashing_separates_ambiguous_domain_and_payload_pairs() {
    let hashing = Blake3Hashing;
    let left = hashing.hash_tagged(b"ab", b"c");
    let right = hashing.hash_tagged(b"a", b"bc");

    assert_ne!(left, right);
}
