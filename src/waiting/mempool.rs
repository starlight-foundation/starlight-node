use std::hash::Hasher;

use crate::{keys::Difficulty, protocol::Transaction};
use std::hash::Hash;
use topk::FilteredSpaceSaving;

/// A generic Starlight mempool. Currently prioritizes transactions based on their PoW difficulty,
/// though could be extended in future for user-configurable prioritization (such as TaaC)
///
/// The protocol is agnostic to the mempool prioritization algorithm, so individual leaders
/// can select it based on their preference.
pub struct Mempool<T: Eq + Hash> {
    pool: FilteredSpaceSaving<T>,
}

impl<T: Eq + Hash> Mempool<T> {
    pub fn new(size: usize) -> Self {
        Self {
            pool: FilteredSpaceSaving::new(size),
        }
    }
    pub fn insert(&mut self, t: T, d: Difficulty) {
        if self.pool.get(&t).is_some() {
            return;
        }
        self.pool.insert(t, d.as_u64());
    }
    /// Drains this `Mempool` with `f: T -> U`, returning all items as a `Vec<U>`.
    pub fn drain<U>(&mut self, f: impl Fn(T) -> U) -> Vec<U> {
        let new_pool = FilteredSpaceSaving::new(self.pool.k());
        let pool = std::mem::replace(&mut self.pool, new_pool);
        pool.into_iter().map(|(t, _)| f(t)).collect()
    }
}