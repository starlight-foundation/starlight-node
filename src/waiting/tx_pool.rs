use std::hash::{Hash, Hasher};
use crate::{intercom::{Mailbox, Message, Process}, protocol::{Transaction, Verified}};
use super::Mempool;

struct Entry(Box<Verified<Transaction>>);
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.0.val.nonce == other.0.val.nonce
        && self.0.val.from == other.0.val.from
    }
}
impl Eq for Entry {}
impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.val.nonce.hash(state);
        self.0.val.from.hash(state);
    }
}

pub struct TxPool {
    mailbox: Mailbox,
    leader_mode: bool,
    pool: Mempool<Entry>
}

impl TxPool {
    pub fn new(size: usize) -> Self {
        Self {
            mailbox: Process::TxPool.take_mailbox().unwrap(),
            leader_mode: false,
            pool: Mempool::new(size)
        }
    }
    pub async fn run(mut self) {
        loop {
            match self.mailbox.recv().await {
                Message::Transaction(tx) => {
                    let tx_verified = match Verified::new(*tx) {
                        Ok(verified) => verified,
                        Err(_) => continue
                    };
                    let difficulty = tx_verified.val.work.difficulty(&tx_verified.hash);
                    self.pool.insert(Entry(Box::new(tx_verified)), difficulty);
                },
                Message::NewLeaderSlot(slot) => {
                    let txs = self.pool.drain(|x| x.0);
                    Process::Chain.send(Message::TransactionList(Box::new((slot, txs))));
                },
                _ => {}
            }
        }
    }
}
