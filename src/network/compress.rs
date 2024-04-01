use std::cell::RefCell;

use zstd_safe::{CCtx, DCtx};

use crate::{
    error,
    util::{Error, UninitVec},
};

const ZSTD_LEVEL: i32 = 6;

thread_local! {
    static ZSTD_CCTX: RefCell<CCtx<'static>> = RefCell::new(CCtx::create());
    static ZSTD_DCTX: RefCell<DCtx<'static>> = RefCell::new(DCtx::create());
}

pub fn compress(bytes: &[u8]) -> Vec<u8> {
    // safety: output is not read before initialized
    let mut output = unsafe { Vec::uninit(zstd_safe::compress_bound(bytes.len())) };
    let n = ZSTD_CCTX
        .with(|cctx| {
            cctx.borrow_mut()
                .compress(&mut output[..], bytes, ZSTD_LEVEL)
        })
        .unwrap();
    output.truncate(n);
    output
}

pub fn decompress(bytes: &[u8], max_size: Option<usize>) -> Result<Vec<u8>, Error> {
    let decompress_bound = zstd_safe::decompress_bound(bytes)
        .or_else(|e| Err(error!("decompress_bound failed: {:?}", e)))?;
    if let Some(max_size) = max_size {
        if decompress_bound > max_size as u64 {
            return Err(error!("decompressed size > max_size"));
        }
    }
    // safety: output is not read before initialized
    let mut output = unsafe { Vec::uninit(decompress_bound as usize) };
    let n = ZSTD_DCTX
        .with(|dctx| dctx.borrow_mut().decompress(&mut output[..], bytes))
        .or_else(|e| Err(error!("decompress failed: {:?}", e)))?;
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
        let decompressed = decompress(&compressed, None).unwrap();
        assert_eq!(data, &decompressed[..]);
    }

    #[test]
    fn test_too_large() {
        let data = b"hello world";
        let compressed = compress(data);
        let max_size = Some(5); // Intentionally smaller than the expected decompressed size
        assert!(decompress(&compressed, max_size).is_err());
    }

    #[test]
    fn test_incorrect_decompress() {
        let data = b"wt2gh2giojamonguspotion";
        assert!(decompress(data, None).is_err());
    }
}
