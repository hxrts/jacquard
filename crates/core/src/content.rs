//! Content-addressed identifiers and digest types.

use core::fmt;

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ContentEncodingError {
    #[error("artifact is still open and cannot be canonically addressed")]
    OpenArtifact,
    #[error("artifact bytes are not in canonical form")]
    InvalidCanonicalForm,
}

impl fmt::Display for Blake3Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
