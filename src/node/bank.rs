use crate::blocks::Transaction;
use leapfrog::LeapMap;
use crate::keys::public::Public;
use super::{Account, Block};

pub struct Bank {
    accounts: LeapMap<Public, Account>,
}

impl Bank {
    pub fn new() -> Self {
        Self {
            accounts: LeapMap::new(),
        }
    }

    fn update_account<T, F: FnMut(&mut Account) -> Result<T, ()>>(&self, key: Public, mut f: F) -> Result<T, ()> {
        let mut a = match self.accounts.get_mut(&key) {
            Some(a) => a,
            None => return Err(()),
        };
        let mut r = Err(());
        a.update(|a| {
            r = f(a);
        }).unwrap();
        r
    }

    fn insert_or_update_account<F: FnMut(&mut Account)>(&self, key: Public, value: Account, mut f: F) {
        loop {
            match self.accounts.get_mut(&key) {
                Some(a) => a,
                None => match self.accounts.try_insert(key, value) {
                    Some(_) => continue,
                    None => return,
                }
            }.update(f).unwrap();
            return;
        }
    }

    fn process_send(&self, tr: &Transaction, slot: u64) -> Result<(), ()> {
        self.update_account(tr.from, |a| {
            if a.nonce != tr.nonce
            || a.balance <= tr.amount
            || a.slot >= slot {
                return Err(());
            }
            let new_balance = a.balance - tr.amount;
            if new_balance != tr.balance {
                return Err(());
            }
            a.nonce += 1;
            a.balance -= tr.amount;
            a.slot = slot;
            Ok(())
        })
    }

    fn process_recv(&self, tr: &Transaction) {
        let value = Account::new(tr.amount);
        self.insert_or_update_account(tr.to, value, |a| {
            a.balance += tr.amount;
        });
    }

    pub fn process_transaction(&self, tr: &Transaction, slot: u64) -> Result<(), ()> {
        self.process_send(tr, slot)?;
        self.process_recv(tr);
        Ok(())
    }

    pub fn process_block(&self, block: &Block) -> Result<(), ()> {
        for tr in block.transactions.iter() {
            self.process_send(tr, block.slot)?;
        }
        for tr in block.transactions.iter() {
            self.process_recv(tr);
        }
        Ok(())
    }

    pub fn get_balance_and_nonce(&self, public: Public) -> (u64, u64) {
        match self.accounts.get(&public) {
            Some(mut a) => {
                let a = a.value().unwrap();
                (a.balance, a.nonce)
            }
            None => (0, 0),
        }
    }
}

