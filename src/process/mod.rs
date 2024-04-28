mod message;
pub use message::Message;

use std::future::Future;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;
use crate::util::Error;
use crate::log_error;

pub struct Mailbox(Receiver<Message>);

impl Mailbox {
    pub async fn recv(&mut self) -> Message {
        self.0.recv().expect("process has been forgotten!")
    }
}

#[derive(Clone)]
pub struct Handle(SyncSender<Message>);

impl Handle {
    pub async fn send(&self, msg: Message) {
        let _ = self.0.send(msg).await;
    }
}

const BUF_SIZE: usize = 1024;

pub trait Process {
    const NAME: &'static str;
    fn run(&mut self, mailbox: &mut Mailbox, handle: Handle) -> Result<(), Error>;
}

pub fn spawn<P: Process + Send>(process: P) -> Handle {
    let (tx, rx) = sync_channel(BUF_SIZE);
    let handle = Handle(tx.clone());
    thread::spawn(move || {
        let handle = Handle(tx);
        let mailbox = Mailbox(rx);
        loop {
            match process.run(&mut mailbox, handle.clone()) {
                Ok(_) => break,
                Err(e) => log_error!("process {} failed: {}", P::NAME, e),
            }
        }
    });
    handle
}