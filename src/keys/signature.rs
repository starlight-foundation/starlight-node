use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Derived from the keys module of github.com/feeless/feeless@978eba7.
use crate::hexify;

/// A ed25519+blake2 signature that can be generated with [Private](crate::Private) and
/// checked with [Public](crate::Public).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(align(8))]
pub struct Signature([u8; 64]);

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <&[u8]>::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom(format!("Expected {} bytes, got {}", Self::LEN, bytes.len())));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(bytes);
        Ok(Signature(arr))
    }
}



hexify!(Signature, "signature");

impl Signature {
    pub const LEN: usize = 64;

    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    pub(super) fn internal(&self) -> Result<ed25519_dalek_blake2_feeless::Signature, ()> {
        ed25519_dalek_blake2_feeless::Signature::from_bytes(&self.0).or(Err(()))
    }
}
