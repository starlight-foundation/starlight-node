mod encoding;
mod error;
mod version;

use bitvec::{order::BitOrder, store::BitStore, vec::BitVec};
use serde::{de::DeserializeOwned, Serialize};

pub use encoding::{
    serialize_list_to_display,
    deserialize_list_from_str,
    deserialize_list_from_string,
    deserialize_from_string,
    serialize_to_display,
    deserialize_from_str,
    expect_len,
    to_hex,
    to_hex_lower
};
pub use error::Error;
pub use version::Version;

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
        if cfg!(debug_assertions) {
            v.extend((0..len).map(|_| T::default()));
        } else {
            unsafe {
                v.set_len(len);
            }
        }
        v
    }
}

impl<T: Default> UninitializedVec<T> for Vec<T> {}

pub trait UninitializedBitVec<T: BitStore, O: BitOrder> {
    fn uninitialized(len: usize) -> BitVec<T, O> {
        let mut v = BitVec::with_capacity(len);
        if cfg!(debug_assertions) {
            v.extend((0..len).map(|_| false));
        } else {
            unsafe {
                v.set_len(len);
            }
        }
        v
    }
}

impl<T: BitStore, O: BitOrder> UninitializedBitVec<T, O> for BitVec<T, O> {}
