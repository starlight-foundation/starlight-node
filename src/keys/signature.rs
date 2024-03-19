// Derived from the keys module of github.com/feeless/feeless@978eba7.
use crate::hexify;

/// A ed25519+blake2 signature that can be generated with [Private](crate::Private) and
/// checked with [Public](crate::Public).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(align(8))]
pub struct Signature([u8; 64]);

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
