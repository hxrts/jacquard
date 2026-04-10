//! Hashing traits and content-addressable identity for routing artifacts.
//!
//! This module defines the pure deterministic hashing surface used throughout
//! Jacquard to derive stable content identifiers from canonical byte encodings.
//! All hashing must remain deterministic: no ambient randomness, no wall-clock
//! seeds, and no host-dependent state.
//!
//! Key items exported from this module:
//! - [`Hashing`] — pure trait over a concrete digest type; callers hash raw
//!   bytes or domain-tagged payloads without depending on one algorithm.
//! - [`HashDigestBytes`] — view a digest as its raw byte representation.
//! - [`Blake3Hashing`] — concrete `Hashing` implementation using BLAKE3.
//! - [`ContentAddressable`] — derive a `ContentId` from canonical encoding.
//! - [`TemplateAddressable`] — like `ContentAddressable` but for schema or
//!   template identity rather than instance identity.
//!
//! Domain-tagged hashing (`hash_tagged`) length-prefixes the domain tag to
//! prevent ambiguous collisions between different (domain, payload) pairs.

use jacquard_core::{Blake3Digest, ContentEncodingError, ContentId};
use jacquard_macros::purity;

#[purity(pure)]
/// View a digest as canonical bytes without committing to one hash algorithm.
pub trait HashDigestBytes {
    fn as_bytes(&self) -> &[u8];
}

impl HashDigestBytes for Blake3Digest {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[purity(pure)]
/// Pure deterministic hashing interface over one digest type.
///
/// Pure deterministic boundary.
pub trait Hashing {
    type Digest: Clone + Eq + HashDigestBytes;

    #[must_use]
    fn hash_bytes(&self, input: &[u8]) -> Self::Digest;
    /// Length-prefixed domain tag prevents ambiguous (domain, payload) pairs.
    #[must_use]
    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Self::Digest;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Blake3Hashing;

#[allow(clippy::disallowed_methods)]
impl Hashing for Blake3Hashing {
    type Digest = Blake3Digest;

    fn hash_bytes(&self, input: &[u8]) -> Self::Digest {
        Blake3Digest(*blake3::hash(input).as_bytes())
    }

    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Self::Digest {
        let mut hasher = blake3::Hasher::new();
        // Only the domain is length-prefixed: input is the final field so its
        // length is implicit from the stream end. Domain tags over u32 are
        // programming errors, not recoverable runtime failures.
        let domain_len = u32::try_from(domain.len()).expect("domain tag length exceeds u32");
        hasher.update(&domain_len.to_le_bytes());
        hasher.update(domain);
        hasher.update(input);
        Blake3Digest(*hasher.finalize().as_bytes())
    }
}

/// Shared helper to construct a ContentId from bytes and a hasher.
///
/// This eliminates duplication between ContentAddressable::content_id and
/// TemplateAddressable::template_id.
fn make_content_id<D, H>(bytes: &[u8], hasher: &H) -> ContentId<D>
where
    D: Clone + Eq,
    H: Hashing<Digest = D>,
{
    ContentId {
        digest: hasher.hash_bytes(bytes),
    }
}

#[purity(pure)]
/// Derive a content id from deterministic canonical serialization.
///
/// Pure deterministic boundary.
pub trait ContentAddressable {
    type Digest: Clone + Eq;

    must_use_evidence!("canonical bytes", "encoding errors";
        fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;
    );

    #[must_use = "dropping a computed content id usually means the artifact identity was not checked or recorded"]
    fn content_id<H: Hashing<Digest = Self::Digest>>(
        &self,
        hasher: &H,
    ) -> Result<ContentId<Self::Digest>, ContentEncodingError> {
        Ok(make_content_id(&self.canonical_bytes()?, hasher))
    }
}

#[purity(pure)]
/// Like ContentAddressable but for template/schema identity rather than
/// instance identity.
///
/// Pure deterministic boundary.
pub trait TemplateAddressable {
    type Digest: Clone + Eq;

    must_use_evidence!("template bytes", "encoding errors";
        fn template_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;
    );

    #[must_use = "dropping a computed template id usually means the template identity was not checked or recorded"]
    fn template_id<H: Hashing<Digest = Self::Digest>>(
        &self,
        hasher: &H,
    ) -> Result<ContentId<Self::Digest>, ContentEncodingError> {
        Ok(make_content_id(&self.template_bytes()?, hasher))
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{Blake3Digest, ContentEncodingError};

    use super::{Blake3Hashing, ContentAddressable, HashDigestBytes, Hashing};

    struct StaticContent(&'static [u8]);

    impl ContentAddressable for StaticContent {
        type Digest = Blake3Digest;

        fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError> {
            Ok(self.0.to_vec())
        }
    }

    #[test]
    fn hashing_is_deterministic() {
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
        let item = StaticContent(b"jacquard");
        let content_id = item.content_id(&hashing).expect("content id");

        assert_eq!(content_id.digest, hashing.hash_bytes(b"jacquard"));
    }

    #[test]
    fn blake3_digest_exposes_bytes() {
        let digest = Blake3Hashing.hash_bytes(b"jacquard");
        assert_eq!(digest.as_bytes().len(), 32);
    }
}
