use std::sync::atomic::{AtomicU64, Ordering};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Index(u64);

impl Index {
    pub const fn zero() -> Self {
        Self(0)
    }
    pub const fn plus(self, n: u64) -> Self {
        Self(self.0 + n)
    }
    pub const fn saturating_sub(self, other: Index) -> u64 {
        self.0.saturating_sub(other.0)
    }
    pub fn to_u64(self) -> u64 {
        self.0
    }
}

pub struct IndexFactory(AtomicU64);

impl IndexFactory {
    pub const fn new(i: Index) -> Self {
        Self(AtomicU64::new(i.0))
    }
    pub fn next(&self) -> Index {
        Index(self.0.fetch_add(1, Ordering::Relaxed))
    }
    pub fn prev(&self) -> Index {
        Index(self.0.fetch_sub(1, Ordering::Relaxed) - 1)
    }
    pub fn get_next(&self) -> Index {
        Index(self.0.load(Ordering::SeqCst))
    }
    pub fn set_next(&self, i: Index) {
        self.0.store(i.0, Ordering::SeqCst);
    }
}
