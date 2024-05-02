use std::hash::{Hash, Hasher};
use crate::{process::{Handle, Mailbox, Message, Process}, protocol::{Open, OpenFull}, util::Error};
use super::Mempool;

struct Entry(Box<OpenFull>);
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.0.open.account == other.0.open.account
    }
}
impl Eq for Entry {}
impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.open.account.hash(state);
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

    fn run(&mut self, mut mailbox: Mailbox, _: Handle) -> Result<(), Error> {
        loop {
            match mailbox.recv() {
                Message::StartLeaderMode => self.leader_mode = true,
                Message::EndLeaderMode => {
                    self.pool.clear();
                    self.leader_mode = false;
                },
                Message::Open(open) => {
                    let hash = match open.verify_and_hash() {
                        Ok(v) => v,
                        _ => continue
                    };
                    let difficulty = open.work.difficulty(&hash);
                    self.pool.insert(Entry(Box::new(OpenFull::new(*open, hash))), difficulty);
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
