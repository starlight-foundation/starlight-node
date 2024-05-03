use std::time::Duration;

use kanal::{Receiver, Sender};

use super::{Handle, Message};

pub struct Oncebox(Receiver<Message>, Sender<Message>);

impl Oncebox {
    pub fn new() -> Self {
        let (tx, rx) = kanal::bounded(1);
        Self(rx, tx)
    }
    pub fn handle(&self) -> Handle {
        Handle(self.1.clone())
    }
    pub fn recv_timeout(self, timeout: Duration) -> Option<Message> {
        self.0.recv_timeout(timeout).ok()
    }
}