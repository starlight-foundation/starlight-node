mod archived;
mod encoding;
mod error;
mod merkle;
mod version;
mod atomic;

use bitvec::{order::BitOrder, store::BitStore, vec::BitVec};
use serde::{de::DeserializeOwned, Serialize};

pub use archived::{ArchivableTo, Archived};
pub use encoding::{
    deserialize_from_str, deserialize_from_string, deserialize_list_from_str,
    deserialize_list_from_string, expect_len, serialize_list_to_display, serialize_to_display,
    to_hex, to_hex_lower,
};
pub use error::Error;
pub use merkle::{merkle_root, merkle_root_direct};
pub use version::Version;
pub use atomic::Atomic;

#[macro_export]
macro_rules! static_assert {
    ($($tt:tt)*) => {
        const _: () = assert!($($tt)*);
    }
}

pub fn serialize_into<T: Serialize>(buf: &mut Vec<u8>, value: &T) {
    bincode::serialize_into(buf, value).unwrap()
}

pub fn serialize<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).unwrap()
}

pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    bincode::deserialize(bytes).map_err(Into::into)
}

pub trait UninitVec<T: Copy> {
    unsafe fn uninit(len: usize) -> Vec<T> {
        let mut v = Vec::with_capacity(len);
        v.set_len(len);
        v
    }
}

impl<T: Copy> UninitVec<T> for Vec<T> {}

pub trait UninitBitVec<T: BitStore, O: BitOrder> {
    unsafe fn uninit(len: usize) -> BitVec<T, O> {
        let mut v = BitVec::with_capacity(len);
        v.set_len(len);
        v
    }
}

pub trait DefaultInitVec<T: Default> {
    fn default_init(len: usize) -> Vec<T> {
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(T::default());
        }
        v
    }
}

impl<T: Default> DefaultInitVec<T> for Vec<T> {}

impl<T: BitStore, O: BitOrder> UninitBitVec<T, O> for BitVec<T, O> {}

pub const fn view_as_bytes<T: Copy>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>()) }
}
