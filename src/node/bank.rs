use super::{Account, Batch, BatchFactory, Block};
use crate::blocks::{Amount, Tx, TxKind};
use crate::keys::Public;
use leapfrog::LeapMap;

pub struct Bank {
    // A map storing accounts indexed by public keys
    accounts: LeapMap<Public, Account>,
    // A factory for generating unique batch IDs
    batch_factory: BatchFactory,
}

impl Bank {
    pub fn new() -> Self {
        Self {
            accounts: LeapMap::new(),
            batch_factory: BatchFactory::new(),
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

    // Insert a new account or update an existing one
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
                // Return None if the account insertion fails
                return None;
            }
        }
    }

    // Get the account associated with the given public key
    fn get_account(&self, key: &Public) -> Option<Account> {
        self.accounts.get(key)?.value()
    }

    // Process the send half of a transaction
    fn process_send(&self, tx: &Tx, batch: Batch) -> Result<(), ()> {
        if tx.kind == TxKind::ChangeRepresentative {
            self.update_account(&tx.from, |a| {
                // Return an error if the nonce or batch doesn't match
                if a.nonce != tx.nonce || a.batch == batch {
                    return Err(());
                }
                // Increment the account nonce
                a.nonce += 1;
                // Update the account batch
                a.batch = batch;
                Ok(())
            })?;
        }
        self.update_account(&tx.from, |a| {
            // Return an error if the nonce, balance, or batch doesn't match
            if a.nonce != tx.nonce || a.latest_balance <= tx.amount || a.batch == batch {
                return Err(());
            }
            let new_balance = a.latest_balance - tx.amount;
            // Return an error if the new balance doesn't match the expected balance
            if new_balance != tx.balance {
                return Err(());
            }
            // Increment the account nonce
            a.nonce += 1;
            // Deduct the transaction amount from the account balance
            a.latest_balance -= tx.amount;
            // Update the account batch
            a.batch = batch;
            Ok(())
        })?;
        Ok(())
    }

    // Process the receive half of the transaction
    fn process_recv(&self, tx: &Tx) {
        // Skip processing for change representative transactions
        if tx.kind == TxKind::ChangeRepresentative {
            return;
        }
        self.insert_or_update_account(
            &tx.to,
            // Create a new account with the transaction amount as the latest balance
            || Account::with_latest_balance(tx.amount),
            |a| {
                // Add the transaction amount to the account balance
                a.latest_balance += tx.amount;
            },
        );
    }

    // Revert a transaction
    fn revert_transaction(&self, tx: &Tx) {
        if tx.kind == TxKind::ChangeRepresentative {
            self.update_account(&tx.from, |a| {
                // Decrement the account nonce
                a.nonce -= 1;
                Ok(())
            })
            .unwrap();
            return;
        }
        self.update_account(&tx.from, |a| {
            // Decrement the account nonce
            a.nonce -= 1;
            // Add the transaction amount back to the account balance
            a.latest_balance += tx.amount;
            Ok(())
        })
        .unwrap();
        let remove_account = self
            .update_account(&tx.to, |a| {
                // Deduct the transaction amount from the account balance
                a.latest_balance -= tx.amount;
                // Check if the account should be removed
                Ok(a.latest_balance == Amount::zero() && a.nonce == 0)
            })
            .unwrap();
        if remove_account {
            // Remove the account if it has zero balance and nonce
            self.accounts.remove(&tx.to);
        }
    }

    // Finalize a transaction
    fn finalize_transaction(&self, tx: &Tx) {
        match tx.kind {
            TxKind::Normal => {
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
                self.insert_or_update_account(
                    &to_rep,
                    // Create a new account with the transaction amount as the weight
                    || Account::with_weight(tx.amount),
                    |a| {
                        // Add the transaction amount to the representative's weight
                        a.weight += tx.amount;
                    },
                );
            }
            TxKind::ChangeRepresentative => {
                let (prev_rep, finalized_balance) = self
                    .update_account(&tx.from, |a| {
                        let prev_rep = a.rep;
                        // Update the account representative to the new representative
                        a.rep = tx.to;
                        Ok((prev_rep, a.finalized_balance))
                    })
                    .unwrap();
                _ = self.update_account(&prev_rep, |a| {
                    // Deduct the finalized balance from the previous representative's weight
                    a.weight -= finalized_balance;
                    Ok(())
                });
                self.insert_or_update_account(
                    &tx.to,
                    // Create a new account with the transaction amount as the weight
                    || Account::with_weight(tx.amount),
                    |a| {
                        // Add the transaction amount to the new representative's weight
                        a.weight += tx.amount;
                    },
                );
            }
        }
    }

    // Process a transaction
    pub fn process_transaction(&self, tx: &Tx, batch: Batch) -> Result<(), ()> {
        // Process the send half of the transaction
        self.process_send(tx, batch)?;
        // Process the receive half of the transaction
        self.process_recv(tx);
        Ok(())
    }

    // Process a block of transactions
    pub fn process_block(&self, block: &Block) -> Result<(), ()> {
        // Generate a new batch ID
        let batch = self.new_batch();
        for tx in block.transactions.iter() {
            // Process the send half of each transaction in the block
            self.process_send(tx, batch)?;
        }
        for tx in block.transactions.iter() {
            // Process the receive half of each transaction in the block
            self.process_recv(tx);
        }
        Ok(())
    }

    // Revert a block of transactions
    pub fn revert_block(&self, block: &Block) -> Result<(), ()> {
        for tx in block.transactions.iter() {
            // Revert each transaction in the block
            self.revert_transaction(tx);
        }
        Ok(())
    }

    // Finalize a block of transactions
    pub fn finalize_block(&self, block: &Block) -> Result<(), ()> {
        for tx in block.transactions.iter() {
            // Finalize each transaction in the block
            self.finalize_transaction(tx);
        }
        Ok(())
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
