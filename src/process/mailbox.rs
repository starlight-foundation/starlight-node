use std::{thread, time::Duration};

use kanal::Receiver;

use super::Message;

#[derive(Clone)]
pub struct Mailbox(pub(super) Receiver<Message>);

impl Mailbox {
    pub fn recv(&mut self) -> Message {
        match self.0.recv() {
            Ok(msg) => msg,
            Err(_) => loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
    pub fn recv_timeout(&mut self, timeout: Duration) -> Option<Message> {
        self.0.recv_timeout(timeout).ok()
    }
}