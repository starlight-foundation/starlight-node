use std::hash::{Hash, Hasher};
use crate::{process::{Handle, Mailbox, Message, Process}, protocol::{Transaction, Verified}, util::Error};
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
    pool: Mempool<Entry>,
    state: Handle,
    leader_mode: bool
}

impl TxPool {
    pub fn new(size: usize, state: Handle) -> Self {
        Self {
            pool: Mempool::new(size),
            state,
            leader_mode: false
        }
    }
}

impl Process for TxPool {
    const NAME: &'static str = "TxPool";
    const RESTART_ON_CRASH: bool = true;

    fn run(&mut self, mailbox: &mut Mailbox, _: Handle) -> Result<(), Error> {
        loop {
            match mailbox.recv() {
                Message::StartLeaderMode => self.leader_mode = true,
                Message::EndLeaderMode => {
                    self.pool.clear();
                    self.leader_mode = false;
                },
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
                    self.state.send(Message::TransactionList(Box::new((slot, txs))));
                },
                _ => {}
            }
        }
    }
}
