use std::io::Write;

use bincode::{Decode, Encode};
use blake3::Hasher;

use crate::hexify;

/// A 32-byte blake3 hash
#[derive(Clone, Copy, PartialEq, Eq, std::hash::Hash, PartialOrd, Ord, Decode, Encode)]
#[repr(align(8))]
pub struct Hash([u8; 32]);

hexify!(Hash, "hash");

impl Hash {
    const LEN: usize = 32;

    pub fn random() -> Self {
        Self(rand::random())
    }

    pub fn digest(slice: &[u8]) -> Self {
        Self(blake3::hash(slice).into())
    }

    pub const fn zero() -> Self {
        Self([0u8; 32])
    }
}

pub struct HashBuilder(Hasher);
impl HashBuilder {
    pub fn new() -> Self {
        Self(Hasher::new())
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub fn finish(&self) -> Hash {
        Hash(self.0.finalize().into())
    }
}

impl Write for HashBuilder {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
