mod encoding;
mod error;

use bitvec::{order::BitOrder, store::BitStore, vec::BitVec};
use serde::{de::DeserializeOwned, Serialize};

pub use encoding::{deserialize_from_str, expect_len, to_hex, to_hex_lower};
pub use error::Error;

use crate::keys::{Hash, HashBuilder};

/* pub fn hash<T: Serialize>(value: &T) -> Hash {
    let mut hasher = HashBuilder::new();
    bincode::serialize_into(&mut hasher, value).unwrap();
    hasher.finalize()
} */

pub fn serialize_into<T: Serialize>(buf: &mut Vec<u8>, value: &T) {
    bincode::serialize_into(buf, value).unwrap()
}

pub fn serialize<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).unwrap()
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
