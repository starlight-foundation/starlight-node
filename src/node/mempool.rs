use std::hash::Hasher;

use crate::{keys::Difficulty, protocol::Transaction};
use std::hash::Hash;
use topk::FilteredSpaceSaving;

struct MempoolEntry(Box<Transaction>);
impl PartialEq for MempoolEntry {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref() as *const Transaction == other.0.as_ref() as *const Transaction
    }
}
impl Eq for MempoolEntry {}
impl Hash for MempoolEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0.as_ref() as *const Transaction).hash(state);
    }
}

/// The Starlight mempool. Currently prioritizes transactions based on their PoW difficulty,
/// though could be extended in future for user-configurable prioritization (such as TaaC)
///
/// The protocol is agnostic to the mempool prioritization algorithm, so individual leaders
/// can select it based on their preference.
pub struct Mempool {
    pool: FilteredSpaceSaving<MempoolEntry>,
}

impl Mempool {
    pub fn new(size: usize) -> Self {
        Self {
            pool: FilteredSpaceSaving::new(size),
        }
    }
    pub fn insert(&mut self, tr: Box<Transaction>, d: Difficulty) {
        self.pool.insert(MempoolEntry(tr), d.as_u64());
    }
    pub fn drain(&mut self) -> impl Iterator<Item = Box<Transaction>> {
        let new_pool = FilteredSpaceSaving::new(self.pool.k());
        let pool = std::mem::replace(&mut self.pool, new_pool);
        pool.into_iter().map(|(tr, _)| tr.0)
    }
}
