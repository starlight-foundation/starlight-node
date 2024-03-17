use std::fmt;

pub struct Error {
    ptr: *const u8,
    len: isize,
}

impl Error {
    pub const fn from_static(s: &'static str) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: -(s.len() as isize),
        }
    }

    pub fn from_string(s: String) -> Self {
        let s = s.into_boxed_str();
        let len = s.len();
        let ptr = Box::into_raw(s) as *const u8;
        Self {
            ptr,
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
        if self.len < 0 {
            return;
        }
        let len = self.len as usize;
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut u8, len) };
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
