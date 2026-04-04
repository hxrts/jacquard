use contour_core::{Blake3Digest, ContentEncodingError};

use crate::{Blake3Hashing, ContentAddressable, Hashing};

struct StaticContent(&'static [u8]);

impl ContentAddressable for StaticContent {
    type Digest = Blake3Digest;

    fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError> {
        Ok(self.0.to_vec())
    }
}

#[test]
fn blake3_hashing_is_deterministic() {
    let hashing = Blake3Hashing;
    let digest_a = hashing.hash_tagged(b"route", b"payload");
    let digest_b = hashing.hash_tagged(b"route", b"payload");
    let digest_c = hashing.hash_tagged(b"other", b"payload");

    assert_eq!(digest_a, digest_b);
    assert_ne!(digest_a, digest_c);
}

#[test]
fn content_addressing_uses_canonical_bytes() {
    let hashing = Blake3Hashing;
    let item = StaticContent(b"contour");
    let content_id = item.content_id(&hashing).expect("content id");

    assert_eq!(content_id.digest, hashing.hash_bytes(b"contour"));
}
