use crate::keys::Hash;
use crate::util::Error;

pub trait Verifiable {
    fn verify_and_hash(&self) -> Result<Hash, Error>;
}

pub struct Verified<T: Verifiable> {
    pub val: T,
    pub hash: Hash
}

impl<T: Verifiable> Verified<T> {
    pub fn new(val: T) -> Result<Self, Error> {
        let hash = val.verify_and_hash()?;
        Ok(Self { val, hash })
    }
}