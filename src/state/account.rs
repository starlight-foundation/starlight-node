use std::sync::atomic::AtomicU64;

use crate::{
    keys::Public,
    protocol::{Amount, Slot}, util::Atomic,
};

use super::Batch;

#[derive(Debug)]
pub struct Account {
    pub batch: Atomic<Batch>,
    pub latest_balance: Atomic<Amount>,
    pub finalized_balance: Atomic<Amount>,
    pub weight: Atomic<Amount>,
    pub nonce: AtomicU64,
    pub rep_index: AtomicU64
}
