use super::{Account, Batch, BatchFactory, Block, Index, IndexFactory};
use crate::keys::Public;
use crate::protocol::{Amount, Epoch, Transaction, TransactionKind};
use leapfrog::LeapMap;

pub struct Bank {
    /// A map storing accounts indexed by public keys
    accounts: LeapMap<Public, Account>,
    /// A factory for generating unique batch IDs
    batch_factory: BatchFactory,
    /// A factory for generating unique account indices
    index_factory: IndexFactory,
}

impl Bank {
    pub fn new(genesis: Public) -> Self {
        let index_factory = IndexFactory::new(Index::zero());
        let accounts = LeapMap::new();
        // insert genesis
        accounts.insert(
            genesis,
            Account {
                latest_balance: Amount::initial_supply(),
                finalized_balance: Amount::initial_supply(),
                weight: Amount::initial_supply(),
                batch: Batch::null(),
                nonce: 0,
                rep: genesis,
                index: index_factory.next(),
            },
        );
        // insert burn address
        accounts.insert(
            Public::zero(),
            Account {
                latest_balance: Amount::zero(),
                finalized_balance: Amount::zero(),
                weight: Amount::zero(),
                batch: Batch::null(),
                nonce: 0,
                rep: Public::zero(),
                index: index_factory.next(),
            },
        );
        Self {
            accounts,
            batch_factory: BatchFactory::new(),
            index_factory,
        }
    }

    // Get a globally unique batch ID
    pub fn new_batch(&self) -> Batch {
        self.batch_factory.next()
    }

    // Update an account by applying the provided function
    fn update_account<T, F: FnMut(&mut Account) -> Result<T, ()>>(
        &self,
        key: &Public,
        mut f: F,
    ) -> Result<T, ()> {
        // Get a mutable reference to the account or return an error if it doesn't exist
        let mut a = self.accounts.get_mut(key).ok_or(())?;
        let mut r = Err(());
        // Update the account using the provided function and store the result in 'r'
        a.update(|a| r = f(a)).unwrap();
        // Return the result of the account update
        r
    }

    /// Insert a new account or update an existing one
    /// Returns Some(f()) if the account was updated, None if the account was inserted
    fn insert_or_update_account<T, F: FnMut(&mut Account) -> T, G: FnMut() -> Account>(
        &self,
        key: &Public,
        mut default: G,
        mut f: F,
    ) -> Option<T> {
        loop {
            if let Some(mut a) = self.accounts.get_mut(key) {
                let mut r = None;
                // Update the existing account using the provided function 'f'
                a.update(|a| r = Some(f(a))).unwrap();
                // Return the result of the account update
                return r;
            } else if self.accounts.try_insert(*key, default()).is_none() {
                // Return None if the account insertion succeeds
                return None;
            }
        }
    }

    // Get the account associated with the given public key
    fn get_account(&self, key: &Public) -> Option<Account> {
        self.accounts.get(key)?.value()
    }

    /// Queue the transaction to ensure there aren't any conflicts with any others in the batch
    /// Queuing only affects the validity & behavior of other transactions within the provided `batch`
    pub fn queue_transaction(&self, tr: &Transaction, batch: Batch) -> Result<(), ()> {
        match tr.kind.get() {
            // this does allow conflicting opens in a single block,
            // which is OK tbh and will be blocked anyway by all
            // but the most pathological of leaders
            // we'll resolve the issue of conflicting reps by picking
            // the largest (by public key) representative
            TransactionKind::Open => {
                // 1) ensure account does not already exist
                if self.accounts.contains_key(&tr.from) {
                    return Err(());
                }
                // 2) ensure representative exists
                if !self.accounts.contains_key(&tr.to) {
                    return Err(());
                }
            }
            TransactionKind::Transfer => {
                // 1) ensure receiving side exists
                if !self.accounts.contains_key(&tr.to) {
                    return Err(());
                }
                // 2) set marker on from side
                self.update_account(&tr.from, |a| {
                    // Return an error if the nonce, balance, or batch doesn't match
                    if a.nonce != tr.nonce || a.latest_balance < tr.amount || a.batch == batch {
                        return Err(());
                    }
                    a.batch = batch;
                    Ok(())
                })?;
            }
            TransactionKind::ChangeRepresentative => {
                // 1) ensure rep exists
                if !self.accounts.contains_key(&tr.to) {
                    return Err(());
                }
                // 2) set batch
                self.update_account(&tr.from, |a| {
                    // Return an error if the nonce or batch doesn't match
                    if a.nonce != tr.nonce || a.batch == batch {
                        return Err(());
                    }
                    // Update the account batch
                    a.batch = batch;
                    Ok(())
                })?;
            }
            TransactionKind::Unknown => {
                return Err(());
            }
        };
        Ok(())
    }

    /// Finish a queued transaction
    pub fn finish_transaction(&self, tr: &Transaction, batch: Batch) {
        match tr.kind.get() {
            TransactionKind::Transfer => {
                // deduct from send half
                self.update_account(&tr.from, |a| {
                    a.nonce += 1;
                    a.latest_balance -= tr.amount;
                    Ok(())
                })
                .unwrap();
                // add to recv half
                self.update_account(&tr.to, |a| {
                    a.latest_balance += tr.amount;
                    Ok(())
                })
                .unwrap();
            }
            TransactionKind::ChangeRepresentative => {
                // update representative
                self.update_account(&tr.from, |a| {
                    a.rep = tr.to;
                    Ok(())
                })
                .unwrap();
            }
            TransactionKind::Open => {
                self.insert_or_update_account(
                    &tr.from,
                    || Account {
                        latest_balance: Amount::zero(),
                        finalized_balance: Amount::zero(),
                        weight: Amount::zero(),
                        batch,
                        nonce: 0,
                        rep: tr.to,
                        index: self.index_factory.next(),
                    },
                    |a| {
                        // there may be multiple opens in a single block,
                        // so we have to handle the case of conflicting representatives
                        // gracefully.
                        if tr.to > a.rep {
                            a.rep = tr.to;
                        }
                    },
                )
                .unwrap();
            }
            TransactionKind::Unknown => {}
        }
    }

    // Revert a transaction
    fn revert_transaction(&self, tx: &Transaction) {
        match tx.kind.get() {
            TransactionKind::Open => {
                // Remove the account for the map. If it existed --
                if self.accounts.remove(&tx.from).is_some() {
                    // Decrement the account index counter.
                    // This action invalidates any indices created by the factory during
                    // the current block's reversion. However, creating new accounts isn't necessary
                    // in this phase, so this approach is acceptable.
                    self.index_factory.prev();
                }
            }
            TransactionKind::ChangeRepresentative => {
                self.update_account(&tx.from, |a| {
                    // Decrement the account nonce
                    a.nonce -= 1;
                    Ok(())
                })
                .unwrap();
            }
            TransactionKind::Transfer => {
                self.update_account(&tx.from, |a| {
                    // Decrement the account nonce
                    a.nonce -= 1;
                    // Add the transaction amount back to the account balance
                    a.latest_balance += tx.amount;
                    Ok(())
                })
                .unwrap();
                self.update_account(&tx.to, |a| {
                    // Deduct the transaction amount from the account balance
                    a.latest_balance -= tx.amount;
                    Ok(())
                })
                .unwrap();
            }
            TransactionKind::Unknown => {}
        }
    }

    // Finalize a transaction
    fn finalize_transaction(&self, tx: &Transaction) {
        match tx.kind.get() {
            TransactionKind::Open => {}
            TransactionKind::Transfer => {
                let from_rep = self
                    .update_account(&tx.from, |a| {
                        // Deduct the transaction amount from the sender's finalized balance
                        a.finalized_balance -= tx.amount;
                        Ok(a.rep)
                    })
                    .unwrap();
                let to_rep = self
                    .update_account(&tx.to, |a| {
                        // Add the transaction amount to the receiver's finalized balance
                        a.finalized_balance += tx.amount;
                        Ok(a.rep)
                    })
                    .unwrap();
                self.update_account(&from_rep, |a| {
                    // Deduct the transaction amount from the representative's weight
                    a.weight -= tx.amount;
                    Ok(())
                })
                .unwrap();
                self.update_account(&to_rep, |a| {
                    // Add the transaction amount to the representative's weight
                    a.weight += tx.amount;
                    Ok(())
                })
                .unwrap();
            }
            TransactionKind::ChangeRepresentative => {
                let (prev_rep, finalized_balance) = self
                    .update_account(&tx.from, |a| {
                        // Update the account representative to the new representative
                        let prev_rep = std::mem::replace(&mut a.rep, tx.to);
                        Ok((prev_rep, a.finalized_balance))
                    })
                    .unwrap();
                self.update_account(&prev_rep, |a| {
                    // Deduct the finalized balance from the previous representative's weight
                    a.weight -= finalized_balance;
                    Ok(())
                })
                .unwrap();
                self.update_account(&tx.to, |a| {
                    // Add the transaction amount to the new representative's weight
                    a.weight += tx.amount;
                    Ok(())
                })
                .unwrap();
            }
            TransactionKind::Unknown => {}
        }
    }

    // Process a block of transactions outright, queuing and finishing them in a new batch
    pub fn process_block(&self, block: &Block) -> Result<(), ()> {
        // Generate a new batch ID
        let batch = self.new_batch();
        for tr in block.transactions.iter() {
            // Queue each transaction in the block to ensure there are no conflicts
            self.queue_transaction(tr, batch)?;
        }
        for tr in block.transactions.iter() {
            // Execute the send half of each transaction in the block
            self.finish_transaction(tr, batch);
        }
        Ok(())
    }

    /// Finish a block of transactions which have all already been queued in a batch
    pub fn finish_block(&self, block: Block, batch: Batch) {
        for tr in block.transactions.iter() {
            self.finish_transaction(tr, batch);
        }
    }

    // Revert a block of transactions
    pub fn revert_block(&self, block: &Block) {
        for tx in block.transactions.iter() {
            // Revert each transaction in the block
            self.revert_transaction(tx);
        }
    }

    // Finalize a block of transactions
    pub fn finalize_block(&self, block: &Block) {
        for tx in block.transactions.iter() {
            // Finalize each transaction in the block
            self.finalize_transaction(tx);
        }
    }

    // Get the latest balance, finalized balance, and nonce for an account
    pub fn get_latest_finalized_and_nonce(&self, public: &Public) -> (Amount, Amount, u64) {
        match self.get_account(public) {
            Some(account) => {
                // Return the latest balance, finalized balance, and nonce of the account
                (
                    account.latest_balance,
                    account.finalized_balance,
                    account.nonce,
                )
            }
            None => (Amount::zero(), Amount::zero(), 0), // Return zero values if the account doesn't exist
        }
    }
}
