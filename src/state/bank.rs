use std::sync::atomic::{AtomicU64, Ordering};

use super::{Account, Batch, Block};
use crate::keys::Public;
use crate::protocol::{Amount, Open, Task, Transaction};
use crate::storage::{Database, ListStore, ObjectStore};
use crate::util::{Atomic, Error};

pub struct Bank {
    /// A database mapping public keys to indices
    index_db: Database<Public, u64>,
    /// The account list
    account_list: ListStore<Account>,
    /// The next batch ID
    next_batch: ObjectStore<Batch>
}

impl Bank {
    pub fn open(data_dir: &str, genesis: Public) -> Result<Self, Error> {
        let (mut index_db, mut account_list, mut next_batch) = (
            Database::open(&format!("{}/db", data_dir))?,
            ListStore::open(&format!("{}/list", data_dir))?,
            ObjectStore::open(&format!("{}/next_batch", data_dir), Batch::null())?
        );
        // initialize?
        if index_db.len() == 0 && account_list.len() == 0 {
            // insert genesis
            index_db.put(&genesis, &0);
            account_list.push(
                Account {
                    latest_balance: Atomic::new(Amount::initial_supply()),
                    finalized_balance: Atomic::new(Amount::initial_supply()),
                    weight: Atomic::new(Amount::initial_supply()),
                    batch: Atomic::new(Batch::null()),
                    nonce: AtomicU64::new(0),
                    rep_index: AtomicU64::new(0),
                },
            );
            // insert burn address
            index_db.put(&Public::zero(), &1);
            account_list.push(
                Account {
                    latest_balance: Atomic::new(Amount::zero()),
                    finalized_balance: Atomic::new(Amount::zero()),
                    weight: Atomic::new(Amount::zero()),
                    batch: Atomic::new(Batch::null()),
                    nonce: AtomicU64::new(0),
                    rep_index: AtomicU64::new(1),
                },
            );
        }
        Ok(Self {
            index_db,
            account_list,
            next_batch
        })
    }

    /// Get a globally unique batch ID
    pub fn new_batch(&mut self) -> Batch {
        let batch = self.next_batch.get();
        self.next_batch.put(batch.next()).unwrap();
        batch
    }

    /// Process an `Open` transaction
    pub fn process_open(&mut self, open: &Open, batch: Batch) -> Result<(), ()> {
        // 1) ensure account does not already exist
        if self.index_db.contains_key(&open.account) {
            return Err(());
        }
        // 2) ensure representative exists
        let rep_index = self.index_db.get(&open.representative).ok_or(())?;
        // 3) insert index into index_db
        self.index_db.put(&open.account, &self.account_list.len());
        // 4) insert account into account_list
        self.account_list.push(Account {
            latest_balance: Atomic::new(Amount::zero()),
            finalized_balance: Atomic::new(Amount::zero()),
            weight: Atomic::new(Amount::zero()),
            batch: Atomic::new(batch),
            nonce: AtomicU64::new(0),
            rep_index: AtomicU64::new(rep_index)
        });
        Ok(())
    }

    /// Revert an `Open` transaction
    pub fn revert_open(&self, tx: &Transaction) {
        self.index_db.remove(&tx.from);
        self.account_list.pop();
    }

    /// Process a `transaction` into a `Task`
    pub fn process_transaction(&self, tx: &Transaction) -> Result<Task, ()> {
        Ok(Task {
            nonce: tx.nonce,
            from_index: self.index_db.get(&tx.from).ok_or(())?,
            to_index: self.index_db.get(&tx.to).ok_or(())?,
            amount: tx.amount
        })
    }

    /// Queues a `Task` to prevent conflicts within the same batch.
    /// The queuing mechanism only impacts the validity and behavior of `Task`s within the specified `batch`.
    pub fn queue_task(&self, task: &Task, batch: Batch) -> Result<(), ()> {
        // 1) ensure nonce matches, and balance is sufficient
        let from = self.account_list.get(task.from_index).ok_or(())?;
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
            let from_account = self.account_list.get(task.from_index).unwrap();
            from_account.nonce.fetch_add(1, Ordering::Relaxed);
            from_account.latest_balance.fetch_sub(task.amount, Ordering::Relaxed);
            // add to recv half
            let to_account = self.account_list.get(task.to_index).unwrap();
            to_account.latest_balance.fetch_add(task.amount, Ordering::Relaxed);
        } else {
            let from_account = self.account_list.get(task.from_index).unwrap();
            from_account.rep_index.store(task.to_index, Ordering::Relaxed);
        }
    }

    /// Revert a task
    pub fn revert_task(&self, task: &Task) {
        if !task.is_change_representative() {
            let from_account = self.account_list.get(task.from_index).unwrap();
            // Decrement the account nonce
            from_account.nonce.fetch_sub(1, Ordering::Relaxed);
            // Add the transaction amount back to the account balance
            from_account.latest_balance.fetch_add(task.amount, Ordering::Relaxed);

            let to_account = self.account_list.get(task.to_index).unwrap();
            // Deduct the transaction amount from the account balance
            to_account.latest_balance.fetch_sub(task.amount, Ordering::Relaxed);
        } else {
            let account = self.account_list.get(task.from_index).unwrap();
            // Revert the representative change
            account.rep_index.store(task.to_index, Ordering::Relaxed);
        }
    }

    // Finalize a task
    pub fn finalize_task(&self, task: &Task) {
        if !task.is_change_representative() {
            let from_account = self.account_list.get(task.from_index).unwrap();
            // Deduct the transaction amount from the sender's finalized balance
            from_account.finalized_balance.fetch_sub(task.amount, Ordering::Relaxed);
            let from_rep = from_account.rep_index.load(Ordering::Relaxed);

            let to_account = self.account_list.get(task.to_index).unwrap();
            // Add the transaction amount to the receiver's finalized balance
            to_account.finalized_balance.fetch_add(task.amount, Ordering::Relaxed);
            let to_rep = to_account.rep_index.load(Ordering::Relaxed);

            let from_rep_account = self.account_list.get(from_rep).unwrap();
            // Deduct the transaction amount from the representative's weight
            from_rep_account.weight.fetch_sub(task.amount, Ordering::Relaxed);

            let to_rep_account = self.account_list.get(to_rep).unwrap();
            // Add the transaction amount to the representative's weight
            to_rep_account.weight.fetch_add(task.amount, Ordering::Relaxed);
        } else {
            let from_account = self.account_list.get(task.from_index).unwrap();
            // Get the previous representative index
            let prev_rep = from_account.rep_index.swap(task.to_index, Ordering::Relaxed);
            let finalized_balance = from_account.finalized_balance.load(Ordering::Relaxed);

            let prev_rep_account = self.account_list.get(prev_rep).unwrap();
            // Deduct the finalized balance from the previous representative's weight
            prev_rep_account.weight.fetch_sub(finalized_balance, Ordering::Relaxed);

            let to_rep_account = self.account_list.get(task.to_index).unwrap();
            // Add the finalized balance to the new representative's weight
            to_rep_account.weight.fetch_add(finalized_balance, Ordering::Relaxed);
        }
    }

    // Process a block of transactions outright, queuing and finishing them in a new batch
    pub fn process_block(&mut self, block: &Block) -> Result<(), ()> {
        // Generate a new batch ID
        let batch = self.new_batch();
        // Process each open request one-by-one
        for open in block.opens.iter() {
            self.process_open(open, batch)?;
        }
        // Convert all transactions to tasks in parallel,
        // queuing them as we go
        let mut tasks = Vec::with_capacity(block.transactions.len());
        for tx in block.transactions.iter() {
            let task = self.process_transaction(tx)?;
            self.queue_task(&task, batch)?;
            tasks.push(task);
        }
        // Finish all tasks in parallel
        for task in tasks.iter() {
            self.finish_task(task);
        }
        Ok(())
    }
}
