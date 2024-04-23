//use std::sync::atomic::{AtomicU64, Ordering};

use crate::util::ArchivableTo;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Batch(u64);

impl ArchivableTo<u64> for Batch {
    fn archive(self) -> u64 {
        self.0
    }

    fn unarchive(source: u64) -> Self {
        Self(source)
    }
}

impl Batch {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
/*
pub struct BatchFactory(AtomicU64);

impl BatchFactory {
    pub fn new() -> Self {
        Self(AtomicU64::new(1))
    }
    pub fn next(&self) -> Batch {
        Batch(self.0.fetch_add(1, Ordering::Relaxed))
    }
}
*/