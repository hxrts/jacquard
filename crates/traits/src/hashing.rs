//! Hashing traits and content-addressable identity for routing artifacts.

use jacquard_core::{Blake3Digest, ContentEncodingError, ContentId};

use crate::{effect_handler, HashEffects};

pub trait Hashing {
    type Digest: Clone + Eq;

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
        // Length prefix separates "ab"+"c" from "a"+"bc".
        hasher.update(&(domain.len() as u32).to_le_bytes());
        hasher.update(domain);
        hasher.update(input);
        Blake3Digest(*hasher.finalize().as_bytes())
    }
}

#[effect_handler]
impl HashEffects for Blake3Hashing {
    fn hash_bytes(&self, input: &[u8]) -> Blake3Digest {
        <Self as Hashing>::hash_bytes(self, input)
    }

    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Blake3Digest {
        <Self as Hashing>::hash_tagged(self, domain, input)
    }
}

/// Derive a content id from deterministic canonical serialization.
pub trait ContentAddressable {
    type Digest: Clone + Eq;

    fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;

    #[must_use = "dropping a computed content id usually means the artifact identity was not checked or recorded"]
    fn content_id<H: Hashing<Digest = Self::Digest>>(
        &self,
        hasher: &H,
    ) -> Result<ContentId<Self::Digest>, ContentEncodingError> {
        let canonical = self.canonical_bytes()?;
        Ok(ContentId {
            digest: hasher.hash_bytes(&canonical),
        })
    }
}

/// Like ContentAddressable but for template/schema identity rather than instance identity.
pub trait TemplateAddressable {
    type Digest: Clone + Eq;

    fn template_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;

    #[must_use = "dropping a computed template id usually means the template identity was not checked or recorded"]
    fn template_id<H: Hashing<Digest = Self::Digest>>(
        &self,
        hasher: &H,
    ) -> Result<ContentId<Self::Digest>, ContentEncodingError> {
        let canonical = self.template_bytes()?;
        Ok(ContentId {
            digest: hasher.hash_bytes(&canonical),
        })
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{Blake3Digest, ContentEncodingError};

    use super::{Blake3Hashing, ContentAddressable, Hashing};

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
        let item = StaticContent(b"jacquard");
        let content_id = item.content_id(&hashing).expect("content id");

        assert_eq!(content_id.digest, hashing.hash_bytes(b"jacquard"));
    }
}
