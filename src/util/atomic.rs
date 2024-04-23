use std::{marker::PhantomData, sync::atomic::{AtomicU64, Ordering}};

use super::ArchivableTo;

#[derive(Debug)]
pub struct Atomic<T: ArchivableTo<u64>> {
    value: AtomicU64,
    _phantom: PhantomData<T>,
}

impl<T: ArchivableTo<u64>> Atomic<T> {
    pub const fn new(v: T) -> Self {
        Self {
            value: AtomicU64::new(v.archive()),
            _phantom: PhantomData,
        }
    }

    pub fn load(&self, order: Ordering) -> T {
        T::unarchive(self.value.load(order))
    }

    pub fn store(&self, val: T, order: Ordering) {
        self.value.store(val.archive(), order);
    }

    /// Atomically swaps the value of the atomic with the given value.
    /// All `swap` operations are guaranteed to have a global ordering.
    pub fn swap(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.swap(val.archive(), order))
    }

    pub fn compare_exchange(&self, current: T, new: T, success: Ordering, failure: Ordering) -> Result<T, T> {
        match self.value.compare_exchange(current.archive(), new.archive(), success, failure) {
            Ok(v) => Ok(T::unarchive(v)),
            Err(v) => Err(T::unarchive(v)),
        }
    }

    pub fn compare_exchange_weak(&self, current: T, new: T, success: Ordering, failure: Ordering) -> Result<T, T> {
        match self.value.compare_exchange_weak(current.archive(), new.archive(), success, failure) {
            Ok(v) => Ok(T::unarchive(v)),
            Err(v) => Err(T::unarchive(v)),
        }
    }

    pub fn fetch_add(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.fetch_add(val.archive(), order))
    }

    pub fn fetch_sub(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.fetch_sub(val.archive(), order))
    }

    pub fn fetch_and(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.fetch_and(val.archive(), order))
    }

    pub fn fetch_nand(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.fetch_nand(val.archive(), order))
    }

    pub fn fetch_or(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.fetch_or(val.archive(), order))
    }

    pub fn fetch_xor(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.fetch_xor(val.archive(), order))
    }

    pub fn fetch_max(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.fetch_max(val.archive(), order))
    }

    pub fn fetch_min(&self, val: T, order: Ordering) -> T {
        T::unarchive(self.value.fetch_min(val.archive(), order))
    }
}
