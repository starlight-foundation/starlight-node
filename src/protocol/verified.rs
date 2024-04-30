use bincode::{Encode, Decode};

use crate::keys::Hash;
use crate::util::Error;

pub trait Verifiable {
    fn verify_and_hash(&self) -> Result<Hash, Error>;
}

#[derive(Clone, Encode, Decode)]
pub struct Verified<T: Verifiable + Encode + Decode> {
    pub val: T,
    pub hash: Hash
}

impl<T: Verifiable + Encode + Decode> Verified<T> {
    pub fn new(val: T) -> Result<Self, Error> {
        let hash = val.verify_and_hash()?;
        Ok(Self { val, hash })
    }
}