use std::time::Duration;

use crate::{process::{Handle, Mailbox, Process}, util::Error};

pub struct Assembler {

}

impl Assembler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Process for Assembler {
    const NAME: &'static str = "Assembler";

    async fn run(&mut self, mailbox: &mut Mailbox, handle: Handle) -> Result<(), Error> {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

