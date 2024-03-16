use blake2b_simd::{Params, State};
use once_cell::sync::Lazy;

use crate::hexify;

static PARAMS: Lazy<Params> = Lazy::new(|| {
    let mut params = Params::new();
    params.hash_length(32);
    params
});

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hash([u8; 32]);

hexify!(Hash, "hash");

impl Hash {
    pub const LEN: usize = 32;

    pub fn random() -> Self {
        Self(rand::random())
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn digest(slice: &[u8]) -> Self {
        Self(PARAMS.hash(slice).as_bytes().try_into().unwrap())
    }

    pub const fn zero() -> Self {
        Self([0u8; 32])
    }
}

pub struct HashBuilder(State);
impl HashBuilder {
    pub fn new() -> Self {
        Self(PARAMS.to_state())
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub fn finalize(&self) -> Hash {
        let mut v = [0u8; 32];
        Hash(self.0.finalize().as_bytes().try_into().unwrap())
    }
}
