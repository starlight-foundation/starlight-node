use blake2::{digest::{Update, VariableOutput}, Blake2bVar};

use crate::{hexify, keys::blake2b};

pub struct HashBuilder(Blake2bVar);
impl HashBuilder {
    pub fn new() -> Self {
        Self(Blake2bVar::new(32).expect("Output size was zero"))
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub fn finalize(self) -> Hash {
        let mut v = [0u8; 32];
        self.0.finalize_variable(&mut v).unwrap();
        Hash(v)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hash(pub [u8; 32]);

hexify!(Hash, "hash");

impl Hash {
    pub const LEN: usize = 32;

    pub fn of_slice(slice: &[u8]) -> Self {
        Self(blake2b::<{Self::LEN}>(slice))
    }

    pub const fn zero() -> Self {
        Self([0u8; 32])
    }
}

