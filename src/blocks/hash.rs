use crate::{hexify, keys::encoding::blake2b};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hash(pub [u8; 32]);

hexify!(Hash, "hash");

impl Hash {
    pub const LEN: usize = 32;

    pub fn hash(msg: &[u8]) -> Self {
        Self(blake2b::<{Self::LEN}>(msg))
    }
}