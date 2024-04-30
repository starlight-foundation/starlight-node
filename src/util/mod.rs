mod archived;
mod encoding;
mod error;
mod merkle;
mod version;
mod atomic;
mod ticker;

use std::io::Write;

use bincode::{config::{Configuration, Fixint, LittleEndian, NoLimit}, enc::write::Writer, error::{DecodeError, EncodeError}, Decode, Encode};
use bitvec::{order::BitOrder, store::BitStore, vec::BitVec};

pub use archived::{ArchivableTo, Archived};
pub use encoding::{
    expect_len,
    to_hex, to_hex_lower,
};
pub use error::Error;
pub use merkle::{merkle_root, merkle_root_direct};
pub use version::Version;
pub use atomic::Atomic;
pub use ticker::Ticker;

#[macro_export]
macro_rules! static_assert {
    ($($tt:tt)*) => {
        const _: () = assert!($($tt)*);
    }
}

pub trait UninitVec<T: Copy> {
    unsafe fn uninit(len: usize) -> Vec<T> {
        let mut v = Vec::with_capacity(len);
        v.set_len(len);
        v
    }
}

impl<T: Copy> UninitVec<T> for Vec<T> {}

const BINCODE_CONFIG: Configuration<LittleEndian, Fixint, NoLimit> = bincode::config::standard().with_fixed_int_encoding();

pub fn encode_into_writer<W: Write, E: Encode>(w: &mut W, e: &E) -> Result<(), EncodeError> {
    bincode::encode_into_std_write(e, w, BINCODE_CONFIG)?;
    Ok(())
}
pub fn decode_from_slice<D: Decode>(slice: &[u8]) -> Result<D, DecodeError> {
    bincode::decode_from_slice(slice, BINCODE_CONFIG).map(|x| x.0)
}

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

pub const unsafe fn view_as_value<T: Copy>(bytes: &[u8]) -> Result<&T, ()> {
    if bytes.len() != std::mem::size_of::<T>() {
        return Err(());
    }
    return Ok(unsafe {
        &*(bytes.as_ptr() as *const T)
    })
}