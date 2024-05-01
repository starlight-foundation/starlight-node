use crate::process::{Handle, Message, ProcessSolitaryEndless};

use super::{Clock, Slot};

pub struct Scheduler {
    clock: Clock,
    notified: Vec<Handle>
}

impl Scheduler {
    pub fn new(notified: Vec<Handle>) -> Self {
        Self { clock: Clock::new(), notified }
    }
    fn start_leader_mode(&self) {
        for h in self.notified.iter() {
            h.send(Message::StartLeaderMode);
        }
    }
    fn end_leader_mode(&self) {
        for h in self.notified.iter() {
            h.send(Message::EndLeaderMode);
        }
    }
    fn new_leader_slot(&self, slot: Slot) {
        for h in self.notified.iter() {
            h.send(Message::NewLeaderSlot(slot));
        }
    }
}

impl ProcessSolitaryEndless for Scheduler {
    fn run(&mut self) -> ! {
        // cyclic example
        loop {
            // start leader mode!
            self.start_leader_mode();
            // my leader slots
            for _ in 0..4 {
                let slot = self.clock.tick();
                self.new_leader_slot(slot);
            }
            // end leader mode!
            self.end_leader_mode();
            // wait a few slots til we start again
            for _ in 0..4 {
                self.clock.tick();
            }
        }
    }
}