use crate::blocks::Transaction;
use leapfrog::LeapMap;
use crate::keys::public::Public;

#[derive(Clone, Copy, Debug)]
struct Account {
    balance: u64,
    nonce: u64,
}

impl leapfrog::Value for Account {
    fn is_redirect(&self) -> bool {
        self.nonce == u64::MAX
    }

    fn is_null(&self) -> bool {
        self.nonce == u64::MAX - 1
    }

    fn redirect() -> Self {
        Self {
            balance: 0,
            nonce: u64::MAX
        }
    }

    fn null() -> Self {
        Self {
            balance: 0,
            nonce: u64::MAX - 1
        }
    }
}

pub struct Bank {
    accounts: LeapMap<Public, Account>,
}

impl Bank {
    pub fn new() -> Self {
        Self {
            accounts: LeapMap::new(),
        }
    }

    fn update_account<F: FnMut(&mut Account) -> Result<(), ()>>(&self, key: Public, mut f: F) -> Result<(), ()> {
        loop {
            let mut a = match self.accounts.get_mut(&key) {
                Some(a) => a,
                None => return Err(()),
            };
            let mut r = Ok(());
            if a.update(|a| {
                r = f(a);
            }).is_none() {
                continue;
            }
            return r;
        }
    }

    fn insert_or_update_account<F: FnMut(&mut Account)>(&self, key: Public, value: Account, mut f: F) {
        loop {
            let mut a = match self.accounts.get_mut(&key) {
                Some(a) => a,
                None => match self.accounts.try_insert(key, value) {
                    Some(_) => continue,
                    None => return,
                }
            };
            if a.update(|a| {
                f(a);
            }).is_none() {
                continue;
            }
            return;
        }
    }

    fn process_send(&self, tr: &Transaction) -> Result<(), ()> {
        self.update_account(tr.from, |a| {
            if a.nonce != tr.nonce
            || a.balance <= tr.balance {
                return Err(());
            }
            a.nonce += 1;
            a.balance -= tr.balance;
            Ok(())
        })
    }

    fn process_recv(&self, tr: &Transaction) {
        let value = Account {
            balance: tr.balance,
            nonce: 0
        };
        self.insert_or_update_account(tr.to, value, |a| {
            a.balance = tr.balance;
        });
    }

    pub fn process_transaction(&self, tr: &Transaction) -> Result<(), ()> {
        tr.validate()?;
        self.process_send(tr)?;
        self.process_recv(tr);
        Ok(())
    }

    pub fn process_block(&self, block: &[Transaction]) -> Result<(), ()> {
        for tr in block {
            tr.validate()?;
            self.process_send(tr)?;
        }
        for tr in block {
            self.process_recv(tr);
        }
        Ok(())
    }
}

