use std::cell::RefCell;

use zstd_safe::{CCtx, DCtx};

use crate::util::UninitializedVec;

const ZSTD_LEVEL: i32 = 6;

thread_local! {
    static ZSTD_CCTX: RefCell<CCtx<'static>> = RefCell::new(CCtx::create());
    static ZSTD_DCTX: RefCell<DCtx<'static>> = RefCell::new(DCtx::create());
}

pub fn compress(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::uninitialized(zstd_safe::compress_bound(bytes.len()));
    let n = ZSTD_CCTX
        .with(|cctx| {
            cctx.borrow_mut()
                .compress(&mut output[..], bytes, ZSTD_LEVEL)
        })
        .unwrap();
    output.truncate(n);
    output
}

pub fn decompress(bytes: &[u8]) -> Result<Vec<u8>, zstd_safe::ErrorCode> {
    let mut output = Vec::uninitialized(zstd_safe::decompress_bound(bytes)? as usize);
    let n = ZSTD_DCTX.with(|dctx| dctx.borrow_mut().decompress(&mut output[..], bytes))?;
    output.truncate(n);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let data = b"hello world";
        let compressed = compress(data);
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(data, &decompressed[..]);
    }

    #[test]
    fn test_incorrect_decompress() {
        let data = b"wt2gh2giojamonguspotion";
        assert!(decompress(data).is_err());
    }
}
