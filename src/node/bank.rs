use super::{Account, Batch, BatchFactory, Block};
use crate::blocks::{Amount, Tx, TxKind};
use crate::keys::Public;
use leapfrog::LeapMap;

pub struct Bank {
    accounts: LeapMap<Public, Account>, // A map storing accounts indexed by public keys
    batch_factory: BatchFactory, // A factory for generating unique batch IDs
}

impl Bank {
    pub fn new() -> Self {
        Self {
            accounts: LeapMap::new(),
            batch_factory: BatchFactory::new(),
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
        let mut a = self.accounts.get_mut(key).ok_or(())?; // Get a mutable reference to the account or return an error if it doesn't exist
        let mut r = Err(());
        a.update(|a| r = f(a)).unwrap(); // Update the account using the provided function and store the result in 'r'
        r // Return the result of the account update
    }

    /// Insert a new account or update an existing one
    fn insert_or_update_account<T, F: FnMut(&mut Account) -> T, G: FnMut() -> Account>(
        &self,
        key: &Public,
        mut default: G,
        mut f: F,
    ) -> Option<T> {
        loop {
            if let Some(mut a) = self.accounts.get_mut(key) {
                let mut r = None;
                a.update(|a| r = Some(f(a))).unwrap(); // Update the existing account using the provided function 'f'
                return r; // Return the result of the account update
            } else if self.accounts.try_insert(*key, default()).is_none() {
                return None; // Return None if the account insertion fails
            }
        }
    }

    fn get_account(&self, key: &Public) -> Option<Account> {
        self.accounts.get(key)?.value() // Get the account associated with the given public key
    }

    /// Process the send half of a transaction
    fn process_send(&self, tx: &Tx, batch: Batch) -> Result<(), ()> {
        if tx.kind == TxKind::ChangeRepresentative {
            self.update_account(&tx.from, |a| {
                if a.nonce != tx.nonce || a.batch == batch {
                    return Err(()); // Return an error if the nonce or batch doesn't match
                }
                a.nonce += 1; // Increment the account nonce
                a.batch = batch; // Update the account batch
                Ok(())
            })?;
        }
        self.update_account(&tx.from, |a| {
            if a.nonce != tx.nonce || a.latest_balance <= tx.amount || a.batch == batch {
                return Err(()); // Return an error if the nonce, balance, or batch doesn't match
            }
            let new_balance = a.latest_balance - tx.amount;
            if new_balance != tx.balance {
                return Err(()); // Return an error if the new balance doesn't match the expected balance
            }
            a.nonce += 1; // Increment the account nonce
            a.latest_balance -= tx.amount; // Deduct the transaction amount from the account balance
            a.batch = batch; // Update the account batch
            Ok(()) 
        })?;
        Ok(())
    }

    /// Process the receive half of the transaction
    fn process_recv(&self, tx: &Tx) {
        if tx.kind == TxKind::ChangeRepresentative {
            return; // Skip processing for change representative transactions
        }
        self.insert_or_update_account(
            &tx.to,
            || Account::with_latest_balance(tx.amount), // Create a new account with the transaction amount as the latest balance
            |a| {
                a.latest_balance += tx.amount; // Add the transaction amount to the account balance
            },
        );
    }

    fn revert_transaction(&self, tx: &Tx) {
        if tx.kind == TxKind::ChangeRepresentative {
            self.update_account(&tx.from, |a| {
                a.nonce -= 1; // Decrement the account nonce
                Ok(())
            }).unwrap();
            return;
        }
        self.update_account(&tx.from, |a| {
            a.nonce -= 1; // Decrement the account nonce
            a.latest_balance += tx.amount; // Add the transaction amount back to the account balance
            Ok(())
        })
        .unwrap();
        let remove_account = self
            .update_account(&tx.to, |a| {
                a.latest_balance -= tx.amount; // Deduct the transaction amount from the account balance
                Ok(a.latest_balance == Amount::zero() && a.nonce == 0) // Check if the account should be removed
            })
            .unwrap();
        if remove_account {
            self.accounts.remove(&tx.to); // Remove the account if it has zero balance and nonce
        }
    }

    fn finalize_transaction(&self, tx: &Tx) {
        match tx.kind {
            TxKind::Normal => {
                let from_rep = self.update_account(&tx.from, |a| {
                    a.finalized_balance -= tx.amount; // Deduct the transaction amount from the sender's finalized balance
                    Ok(a.rep)
                })
                .unwrap();
                let to_rep = self.update_account(&tx.to, |a| {
                    a.finalized_balance += tx.amount; // Add the transaction amount to the receiver's finalized balance
                    Ok(a.rep)
                })
                .unwrap();
                self.update_account(&from_rep, |a| {
                    a.weight -= tx.amount; // Deduct the transaction amount from the representative's weight
                    Ok(())
                })
                .unwrap();
                self.insert_or_update_account(
                    &to_rep,
                    || Account::with_weight(tx.amount), // Create a new account with the transaction amount as the weight
                    |a| {
                        a.weight += tx.amount; // Add the transaction amount to the representative's weight
                    },
                );
            }
            TxKind::ChangeRepresentative => {
                let (prev_rep, finalized_balance) = self.update_account(
                    &tx.from,
                    |a| {
                        let prev_rep = a.rep;
                        a.rep = tx.to; // Update the account representative to the new representative
                        Ok((prev_rep, a.finalized_balance))
                    },
                )
                .unwrap();
                _ = self.update_account(&prev_rep, |a| {
                    a.weight -= finalized_balance; // Deduct the finalized balance from the previous representative's weight
                    Ok(())
                });
                self.insert_or_update_account(
                    &tx.to,
                    || Account::with_weight(tx.amount), // Create a new account with the transaction amount as the weight
                    |a| {
                        a.weight += tx.amount; // Add the transaction amount to the new representative's weight
                    },
                );
            }
        }
    }

    /// Process a transaction
    pub fn process_transaction(&self, tx: &Tx, batch: Batch) -> Result<(), ()> {
        self.process_send(tx, batch)?; // Process the send half of the transaction
        self.process_recv(tx); // Process the receive half of the transaction
        Ok(())
    }

    /// Process a block of transactions
    pub fn process_block(&self, block: &Block) -> Result<(), ()> {
        let batch = self.new_batch(); // Generate a new batch ID
        for tx in block.transactions.iter() {
            self.process_send(tx, batch)?; // Process the send half of each transaction in the block
        }
        for tx in block.transactions.iter() {
            self.process_recv(tx); // Process the receive half of each transaction in the block
        }
        Ok(())
    }

    /// Revert a block of transactions
    pub fn revert_block(&self, block: &Block) -> Result<(), ()> {
        for tx in block.transactions.iter() {
            self.revert_transaction(tx); // Revert each transaction in the block
        }
        Ok(())
    }

    pub fn finalize_block(&self, block: &Block) -> Result<(), ()> {
        for tx in block.transactions.iter() {
            self.finalize_transaction(tx); // Finalize each transaction in the block
        }
        Ok(())
    }

    /// Get the latest balance, finalized balance, and nonce for an account
    pub fn get_latest_finalized_and_nonce(&self, public: &Public) -> (Amount, Amount, u64) {
        match self.get_account(public) {
            Some(account) => {
                (account.latest_balance, account.finalized_balance, account.nonce) // Return the latest balance, finalized balance, and nonce of the account
            }
            None => (Amount::zero(), Amount::zero(), 0), // Return zero values if the account doesn't exist
        }
    }
}