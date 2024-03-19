use std::fmt;

// if len <= 0, Error is a statically-allocated string
// else, Error is a heap-allocated string
pub struct Error {
    ptr: *mut u8,
    len: isize,
}

impl Error {
    pub const fn from_static(s: &'static str) -> Self {
        Self {
            ptr: s.as_ptr() as *mut u8,
            len: -(s.len() as isize),
        }
    }

    pub fn from_string(s: String) -> Self {
        let s = s.into_boxed_str();
        let len = s.len();
        // since len == 0 are considered static,
        // we have to do this otherwise `s` will never get dropped
        if len == 0 {
            return Self {
                ptr: std::ptr::null_mut(),
                len: 0,
            };
        }
        let ptr = Box::into_raw(s);
        Self {
            ptr: ptr as *mut u8,
            len: len as isize,
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            let len = self.len.abs() as usize;
            let bytes = std::slice::from_raw_parts(self.ptr, len);
            std::str::from_utf8_unchecked(bytes)
        }
    }
}

unsafe impl Send for Error {}

impl Clone for Error {
    fn clone(&self) -> Self {
        if self.len <= 0 {
            return Self {
                ptr: self.ptr,
                len: self.len,
            };
        }
        Self::from_string(self.as_str().to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.as_str())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Error").field(&self.as_str()).finish()
    }
}

impl Drop for Error {
    fn drop(&mut self) {
        if self.len <= 0 {
            return;
        }
        let len = self.len as usize;
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr, len) };
        let _ = unsafe { Box::from_raw(slice) };
    }
}

impl<T> From<T> for Error
where
    T: std::error::Error,
{
    fn from(value: T) -> Self {
        Self::from_string(value.to_string())
    }
}

#[macro_export]
macro_rules! error {
    ($msg:expr) => (
        crate::util::Error::from_static(concat!($msg, " @ ", file!(), ":", line!()))
    );
    ($fmt:expr, $($arg:tt)*) => (
        crate::util::Error::from_string(format!(concat!($fmt, " @ {}:{}"), $($arg)*, file!(), line!()))
    );
}

#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => (
        return std::result::Result::Err(crate::error!($($arg)*))
    );
}
