// Derived from the keys module of github.com/feeless/feeless@978eba7.
use crate::{error::Error, error, hexify};

/// A ed25519+blake2 signature that can be generated with [Private](crate::Private) and
/// checked with [Public](crate::Public).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

hexify!(Signature, "signature");

impl Signature {
    pub const LEN: usize = 64;

    pub(crate) fn zero() -> Self {
        Self([0u8; 64])
    }

    pub(crate) fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    pub(crate) fn internal(&self) -> Result<ed25519_dalek_blake2_feeless::Signature, ()> {
        ed25519_dalek_blake2_feeless::Signature::from_bytes(&self.0).or(Err(()))
    }
}
