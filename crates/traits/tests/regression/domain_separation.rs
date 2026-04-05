//! Verify tagged hashing separates ambiguous domain/payload pairs.

use jacquard_traits::{Blake3Hashing, Hashing};

#[test]
fn tagged_hashing_separates_ambiguous_domain_and_payload_pairs() {
    let hashing = Blake3Hashing;
    let left = hashing.hash_tagged(b"ab", b"c");
    let right = hashing.hash_tagged(b"a", b"bc");

    assert_ne!(left, right);
}
