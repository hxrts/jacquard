//! Content-addressed identifiers and digest types for routing artifacts.
//!
//! This module defines the shared hashing and content-addressing primitives
//! used to give routing artifacts stable, deterministic identities. The
//! primary types are [`Blake3Digest`] (a 32-byte Blake3 hash value),
//! [`ContentId`] (a generic wrapper that pairs a type parameter with its
//! digest), [`BloomFilter`] (a marker type for bloom-filter content
//! summaries), and [`ContentEncodingError`] (the error cases raised when
//! an artifact is not in a canonically addressable state).
//!
//! Content addresses are used by routing identity newtypes such as `RouteId`,
//! `RouteCommitmentId`, `CommitteeId`, and `ReceiptId`, which are truncated
//! to 16 bytes from a full `Blake3Digest` via the `From<&Blake3Digest>`
//! conversions in `base/identity.rs`.

use core::fmt;

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::base::bytes_newtype;

bytes_newtype!(Blake3Digest, 32);

#[public_model]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ContentId<D> {
    pub digest: D,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct BloomFilter;

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentEncodingError {
    OpenArtifact,
    InvalidCanonicalForm,
}

impl fmt::Display for ContentEncodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenArtifact => {
                f.write_str("artifact is still open and cannot be canonically addressed")
            }
            Self::InvalidCanonicalForm => f.write_str("artifact bytes are not in canonical form"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ContentEncodingError {}

impl fmt::Display for Blake3Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
