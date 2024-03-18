mod error;
mod encoding;

use bitvec::{order::BitOrder, store::BitStore, vec::BitVec};
use serde::{de::DeserializeOwned, Serialize};

pub use error::Error;
pub use encoding::{deserialize_from_str, expect_len, to_hex, to_hex_lower};

use crate::keys::{Hash, HashBuilder};

pub fn hash<T: Serialize>(value: &T) -> Hash {
    let mut hasher = HashBuilder::new();
    bincode::serialize_into(&mut hasher, value).unwrap();
    hasher.finalize()
}

pub fn serialize_into<T: Serialize>(buf: &mut Vec<u8>, value: &T) -> Result<(), Error> {
    bincode::serialize_into(buf, value).map_err(Into::into)
}

pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    bincode::serialize(value).map_err(Into::into)
}

pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    bincode::deserialize(bytes).map_err(Into::into)
}

pub trait UninitializedVec<T: Default> {
    fn uninitialized(len: usize) -> Vec<T> {
        let mut v = Vec::with_capacity(len);
        /*unsafe {
            v.set_len(len);
        }*/
        v.extend((0..len).map(|_| T::default()));
        v
    }
}

impl<T: Default> UninitializedVec<T> for Vec<T> {}

pub trait UninitializedBitVec<T: BitStore, O: BitOrder> {
    fn uninitialized(len: usize) -> BitVec<T, O> {
        let mut v = BitVec::with_capacity(len);
        /*unsafe {
            v.set_len(len);
        }*/
        v.extend((0..len).map(|_| false));
        v
    }
}

impl<T: BitStore, O: BitOrder> UninitializedBitVec<T, O> for BitVec<T, O> {}
