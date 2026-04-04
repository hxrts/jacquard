use contour_core::{Blake3Digest, ContentEncodingError, ContentId};

pub trait Hashing {
    type Digest: Clone + Eq;

    fn hash_bytes(&self, input: &[u8]) -> Self::Digest;
    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Self::Digest;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Blake3Hashing;

impl Hashing for Blake3Hashing {
    type Digest = Blake3Digest;

    fn hash_bytes(&self, input: &[u8]) -> Self::Digest {
        Blake3Digest(*blake3::hash(input).as_bytes())
    }

    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Self::Digest {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&(domain.len() as u32).to_le_bytes());
        hasher.update(domain);
        hasher.update(input);
        Blake3Digest(*hasher.finalize().as_bytes())
    }
}

pub trait ContentAddressable {
    type Digest: Clone + Eq;

    fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;

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

pub trait TemplateAddressable {
    type Digest: Clone + Eq;

    fn template_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;

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
