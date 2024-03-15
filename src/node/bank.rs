use std::sync::atomic::{AtomicU64, Ordering};

use super::{Account, Batch, BatchFactory, Block};
use crate::blocks::{Amount, Slot, Transaction};
use crate::keys::Public;
use leapfrog::LeapMap;

pub struct Bank {
    accounts: LeapMap<Public, Account>,
    batch_factory: BatchFactory
}

impl Bank {
    pub fn new() -> Self {
        Self {
            accounts: LeapMap::new(),
            batch_factory: BatchFactory::new()
        }
    }

    /// Get a globally unique batch ID.
    pub fn new_batch(&self) -> Batch {
        self.batch_factory.next()
    }
    

    /// Update an account by applying the provided function
    fn update_account<T, F: FnMut(&mut Account) -> Result<T, ()>>(
        &self,
        key: &Public,
        mut f: F,
    ) -> Result<T, ()> {
        let mut a = self.accounts.get_mut(key).ok_or(())?;
        let mut r = Err(());
        a.update(|a| r = f(a)).unwrap();
        r
    }

    /// Insert a new account or update an existing one
    fn insert_or_update_account<F: FnMut(&mut Account)>(&self, key: Public, value: Account, f: F) {
        loop {
            if let Some(mut a) = self.accounts.get_mut(&key) {
                a.update(f).unwrap();
                return;
            } else if self.accounts.try_insert(key, value).is_none() {
                return;
            }
        }
    }

    /// Process the send half of a transaction
    fn process_send(&self, tr: &Transaction, batch: Batch) -> Result<(), ()> {
        self.update_account(&tr.from, |a| {
            if a.nonce != tr.nonce || a.latest_balance <= tr.amount || a.batch == batch {
                return Err(());
            }
            let new_balance = a.latest_balance - tr.amount;
            if new_balance != tr.balance {
                return Err(());
            }
            a.nonce += 1;
            a.latest_balance -= tr.amount;
            a.batch = batch;
            Ok(())
        })
    }

    /// Process the receive half of the transaction
    fn process_recv(&self, tr: &Transaction) {
        let value = Account::new(tr.amount);
        self.insert_or_update_account(tr.to, value, |a| {
            a.latest_balance += tr.amount;
        });
    }

    fn revert_transaction(&self, tr: &Transaction) {
        self.update_account(&tr.from, |a| {
            a.nonce -= 1;
            a.latest_balance += tr.amount;
            Ok(())
        }).unwrap();
        let remove_account = self.update_account(&tr.to, |a| {
            a.latest_balance -= tr.amount;
            Ok(a.latest_balance == Amount::zero() && a.nonce == 0)
        }).unwrap();
        if remove_account {
            self.accounts.remove(&tr.to);
        }
    }

    fn finalize_transaction(&self, tr: &Transaction) {
        self.update_account(&tr.from, |a| {
            a.finalized_balance += tr.amount;
            Ok(())
        }).unwrap();
        self.update_account(&tr.to, |a| {
            a.finalized_balance -= tr.amount;
            Ok(())
        }).unwrap();
    }

    /// Process a transaction
    pub fn process_transaction(&self, tr: &Transaction, batch: Batch) -> Result<(), ()> {
        self.process_send(tr, batch)?;
        self.process_recv(tr);
        Ok(())
    }

    /// Process a block of transactions
    pub fn process_block(&self, block: &Block) -> Result<(), ()> {
        let batch = self.new_batch();
        for tr in block.transactions.iter() {
            self.process_send(tr, batch)?;
        }
        for tr in block.transactions.iter() {
            self.process_recv(tr);
        }
        Ok(())
    }

    /// Revert a block of transactions
    pub fn revert_block(&self, block: &Block) -> Result<(), ()> {
        for tr in block.transactions.iter() {
            self.revert_transaction(tr);
        }
        Ok(())
    }

    pub fn finalize_block(&self, block: &Block) -> Result<(), ()> {
        for tr in block.transactions.iter() {
            self.finalize_transaction(tr);
        }
        Ok(())
    }

    /// Get the latest finalized balance, nonce, and slot for an account
    pub fn get_latest_finalized_and_nonce(&self, public: Public) -> (Amount, Amount, u64) {
        self.accounts
            .get(&public)
            .map(|mut a| {
                let a = a.value().unwrap();
                (a.latest_balance, a.finalized_balance, a.nonce)
            })
            .unwrap_or((Amount::zero(), Amount::zero(), 0))
    }
}