mod error;
mod encoding;
mod compress;

use bitvec::{order::BitOrder, store::BitStore, vec::BitVec};
pub use error::Error;
pub use encoding::{deserialize_from_str, expect_len, to_hex, to_hex_lower};
pub use compress::{compress, decompress};

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