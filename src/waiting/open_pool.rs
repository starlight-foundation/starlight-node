use std::hash::{Hash, Hasher};
use crate::{process::{Handle, Mailbox, Message, Process}, protocol::{Open, Verified}, util::Error};
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
    pool: Mempool<Entry>,
    state: Handle,
    leader_mode: bool
}

impl OpenPool {
    pub fn new(size: usize, state: Handle) -> Self {
        Self {
            pool: Mempool::new(size),
            state,
            leader_mode: false
        }
    }
}

impl Process for OpenPool {
    const NAME: &'static str = "OpenPool";
    const RESTART_ON_CRASH: bool = true;

    fn run(&mut self, mailbox: &mut Mailbox, _: Handle) -> Result<(), Error> {
        loop {
            match mailbox.recv() {
                Message::StartLeaderMode => self.leader_mode = true,
                Message::EndLeaderMode => {
                    self.pool.clear();
                    self.leader_mode = false;
                },
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
                    self.state.send(Message::OpenList(Box::new((slot, opens))));
                },
                _ => {}
            }
        }
    }
}
