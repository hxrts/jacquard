//! Content-addressed identifiers and digest types.

use core::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

macro_rules! bytes_newtype {
    ($name:ident, $size:expr) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Default,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        pub struct $name(pub [u8; $size]);
    };
}

bytes_newtype!(Blake3Digest, 32);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ContentId<D> {
    pub digest: D,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct BloomFilter;

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
