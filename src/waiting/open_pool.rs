use std::hash::{Hash, Hasher};
use crate::{intercom::{Mailbox, Message, Process}, protocol::{Open, Verified}};
use super::Mempool;

struct Entry(Box<Verified<Open>>);
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.0.val.account == other.0.val.account
    }
}
impl Eq for Entry {}
impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.val.account.hash(state);
    }
}

pub struct OpenPool {
    mailbox: Mailbox,
    leader_mode: bool,
    pool: Mempool<Entry>
}

impl OpenPool {
    pub fn new(size: usize) -> Self {
        Self {
            mailbox: Process::OpenPool.take_mailbox().unwrap(),
            leader_mode: false,
            pool: Mempool::new(size)
        }
    }
    pub async fn run(mut self) {
        loop {
            match self.mailbox.recv().await {
                Message::Open(open) => {
                    let open_verified = match Verified::new(*open) {
                        Ok(verified) => verified,
                        Err(_) => continue
                    };
                    let difficulty = open_verified.val.work.difficulty(&open_verified.hash);
                    self.pool.insert(Entry(Box::new(open_verified)), difficulty);
                },
                Message::NewLeaderSlot(slot) => {
                    let opens = self.pool.drain(|x| x.0);
                    Process::Chain.send(Message::OpenList(Box::new((slot, opens))));
                },
                _ => {}
            }
        }
    }
}
