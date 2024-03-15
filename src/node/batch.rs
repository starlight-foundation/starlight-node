use std::sync::atomic::{AtomicU64, Ordering};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Batch(u64);

impl Batch {
    pub fn null() -> Self {
        Self(0)
    }
}

pub struct BatchFactory(AtomicU64);

impl BatchFactory {
    pub fn new() -> Self {
        Self(AtomicU64::new(1))
    }
    pub fn next(&self) -> Batch {
        Batch(self.0.fetch_add(1, Ordering::Relaxed))
    }
}
