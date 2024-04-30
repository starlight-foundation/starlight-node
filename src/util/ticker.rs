use std::time::Duration;

use crate::process::{self, Handle, Mailbox, Message, ProcessEndless};

/// Sends `Message::Tick` repeatedly to `dst` at the interval specified by `interval`.
/// The first message will be sent immediately after the process is started.
pub struct Ticker {
    dst: Handle,
    interval: Duration
}

impl Ticker {
    pub fn new(dst: Handle, interval: Duration) -> Self {
        Self { dst, interval }
    }
}

impl ProcessEndless for Ticker {
    fn run(&mut self, _: Mailbox, _: Handle) -> ! {
        loop {
            self.dst.send(Message::Tick);
            process::sleep(self.interval);
        }
    }
}