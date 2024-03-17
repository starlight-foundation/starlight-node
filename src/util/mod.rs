mod error;
mod encoding;
mod compress;

pub use error::Error;
pub use encoding::{deserialize_from_str, expect_len, to_hex, to_hex_lower};
pub use compress::{compress, decompress};

pub trait UninitializedVec {
    fn uninitialized<T>(len: usize) -> Vec<T> {
        let mut v = Vec::with_capacity(len);
        unsafe {
            v.set_len(len);
        }
        v
    }
}

impl UninitializedVec for Vec<u8> {}
