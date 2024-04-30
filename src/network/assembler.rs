use std::time::Duration;

use crate::{process::{self, Handle, Mailbox, Process}, util::Error};

pub struct Assembler {

}

impl Assembler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Process for Assembler {
    const NAME: &'static str = "Assembler";
    const RESTART_ON_CRASH: bool = true;

    fn run(&mut self, mailbox: &mut Mailbox, handle: Handle) -> Result<(), Error> {
        loop {
            process::sleep(Duration::from_secs(1));
        }
    }
}

