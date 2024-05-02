use std::hash::{Hash, Hasher};
use crate::{process::{self, Handle, Mailbox, Message, Process}, protocol::TxHalf, util::Error};
use super::{Mempool, TxFiller};

struct Entry(Box<TxHalf>);
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.0.tx.nonce == other.0.tx.nonce
        && self.0.tx.from == other.0.tx.from
    }
}
impl Eq for Entry {}
impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.tx.nonce.hash(state);
        self.0.tx.from.hash(state);
    }
}

pub struct TxPool {
    pool: Mempool<Entry>,
    db: Handle,
    state: Handle,
    leader_mode: bool
}

impl TxPool {
    pub fn new(size: usize, db: Handle, state: Handle) -> Self {
        Self {
            pool: Mempool::new(size),
            db,
            state,
            leader_mode: false
        }
    }
}

impl Process for TxPool {
    const NAME: &'static str = "TxPool";
    const RESTART_ON_CRASH: bool = true;

    fn run(&mut self, mut mailbox: Mailbox, _: Handle) -> Result<(), Error> {
        loop {
            match mailbox.recv() {
                Message::StartLeaderMode => self.leader_mode = true,
                Message::EndLeaderMode => {
                    self.pool.clear();
                    self.leader_mode = false;
                },
                Message::TxEmpty(tx_empty) => {
                    let hash = match tx_empty.tx.verify_and_hash() {
                        Ok(v) => v,
                        Err(_) => continue
                    };
                    let tx_half = tx_empty.provide(hash);
                    let difficulty = tx_half.tx.work.difficulty(&tx_half.hash);
                    self.pool.insert(Entry(tx_half), difficulty);
                },
                Message::NewLeaderSlot(slot) => {
                    let tx_half_list = self.pool.drain(|x| x.0);
                    process::spawn(TxFiller::new(
                        tx_half_list,
                        self.db.clone(),
                        self.state.clone(),
                    ));
                },
                _ => {}
            }
        }
    }
}
