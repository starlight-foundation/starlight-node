use std::sync::atomic::{AtomicU64, Ordering};

use super::{Account, Batch};
use crate::protocol::{Amount, Task};
use crate::storage::ListStore;
use crate::util::{Atomic, Error};

pub struct Bank(ListStore<Account>);

impl Bank {
    pub fn open(dir: &str) -> Result<Self, Error> {
        Ok(Self(ListStore::open(&dir)?))
    }

    /// Get the number of accounts in the `Bank`
    pub fn len(&self) -> u64 {
        self.0.len()
    }

    /// Add a new empty account with representative given by `rep_index`,
    /// and return its index
    pub fn add_account(&mut self) -> u64 {
        let index = self.0.len();
        self.0.push(Account {
            latest_balance: Atomic::new(Amount::zero()),
            finalized_balance: Atomic::new(Amount::zero()),
            weight: Atomic::new(Amount::zero()),
            batch: Atomic::new(Batch::null()),
            nonce: AtomicU64::new(0),
            rep_index: AtomicU64::new(/*rep_index*/0)
        });
        index
    }

    pub fn pop_account(&mut self) -> Option<Account> {
        self.0.pop()
    }

    /// Queues a `Task` to prevent conflicts within the same batch.
    /// The queuing mechanism only impacts the validity and behavior of `Task`s within the specified `batch`.
    pub fn queue_task(&self, task: &Task, batch: Batch) -> Result<(), ()> {
        // 1) ensure nonce matches, and balance is sufficient
        let from = self.0.get(task.from_index).ok_or(())?;
        if from.nonce.load(Ordering::Relaxed) != task.nonce
        || from.latest_balance.load(Ordering::Relaxed) < task.amount {
            return Err(());
        }
        // 2) ensure one transaction per account per batch!
        if from.batch.swap(batch, Ordering::Relaxed) == batch {
            return Err(());
        }
        Ok(())
    }

    /// Finish a created `Task`
    pub fn finish_task(&self, task: &Task) {
        if !task.is_change_representative() {
            // deduct from send half
            let from_account = self.0.get(task.from_index).unwrap();
            from_account.nonce.fetch_add(1, Ordering::Relaxed);
            from_account.latest_balance.fetch_sub(task.amount, Ordering::Relaxed);
            // add to recv half
            let to_account = self.0.get(task.to_index).unwrap();
            to_account.latest_balance.fetch_add(task.amount, Ordering::Relaxed);
        } else {
            let from_account = self.0.get(task.from_index).unwrap();
            from_account.rep_index.store(task.to_index, Ordering::Relaxed);
        }
    }

    /// Revert a task
    pub fn revert_task(&self, task: &Task) {
        if !task.is_change_representative() {
            let from_account = self.0.get(task.from_index).unwrap();
            // Decrement the account nonce
            from_account.nonce.fetch_sub(1, Ordering::Relaxed);
            // Add the transaction amount back to the account balance
            from_account.latest_balance.fetch_add(task.amount, Ordering::Relaxed);

            let to_account = self.0.get(task.to_index).unwrap();
            // Deduct the transaction amount from the account balance
            to_account.latest_balance.fetch_sub(task.amount, Ordering::Relaxed);
        } else {
            let account = self.0.get(task.from_index).unwrap();
            // Revert the representative change
            account.rep_index.store(task.to_index, Ordering::Relaxed);
        }
    }

    /// Finalize a task
    pub fn finalize_task(&self, task: &Task) {
        if !task.is_change_representative() {
            let from_account = self.0.get(task.from_index).unwrap();
            // Deduct the transaction amount from the sender's finalized balance
            from_account.finalized_balance.fetch_sub(task.amount, Ordering::Relaxed);
            let from_rep = from_account.rep_index.load(Ordering::Relaxed);

            let to_account = self.0.get(task.to_index).unwrap();
            // Add the transaction amount to the receiver's finalized balance
            to_account.finalized_balance.fetch_add(task.amount, Ordering::Relaxed);
            let to_rep = to_account.rep_index.load(Ordering::Relaxed);

            let from_rep_account = self.0.get(from_rep).unwrap();
            // Deduct the transaction amount from the representative's weight
            from_rep_account.weight.fetch_sub(task.amount, Ordering::Relaxed);

            let to_rep_account = self.0.get(to_rep).unwrap();
            // Add the transaction amount to the representative's weight
            to_rep_account.weight.fetch_add(task.amount, Ordering::Relaxed);
        } else {
            let from_account = self.0.get(task.from_index).unwrap();
            // Get the previous representative index
            let prev_rep = from_account.rep_index.swap(task.to_index, Ordering::Relaxed);
            let finalized_balance = from_account.finalized_balance.load(Ordering::Relaxed);

            let prev_rep_account = self.0.get(prev_rep).unwrap();
            // Deduct the finalized balance from the previous representative's weight
            prev_rep_account.weight.fetch_sub(finalized_balance, Ordering::Relaxed);

            let to_rep_account = self.0.get(task.to_index).unwrap();
            // Add the finalized balance to the new representative's weight
            to_rep_account.weight.fetch_add(finalized_balance, Ordering::Relaxed);
        }
    }
}
