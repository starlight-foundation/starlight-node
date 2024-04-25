mod message;
pub use message::Message;

use std::future::Future;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::util::Error;
use crate::log_error;

pub struct Mailbox(Receiver<Message>);

impl Mailbox {
    pub async fn recv(&mut self) -> Message {
        self.0.recv().await.expect("process has been forgotten!")
    }
}

#[derive(Clone)]
pub struct Handle(Sender<Message>);

impl Handle {
    pub async fn send(&self, msg: Message) {
        let _ = self.0.send(msg).await;
    }
}

const BUF_SIZE: usize = 1024;

pub trait Process {
    const NAME: &'static str;
    fn run(&mut self, mailbox: &mut Mailbox, handle: Handle) -> impl Future<Output = Result<(), Error>> + Send;
}

pub fn spawn<P: Process + Send>(process: P) -> Handle {
    let (tx, rx) = channel(BUF_SIZE);
    let handle = Handle(tx.clone());
    tokio::spawn(async move {
        let handle = Handle(tx);
        let mailbox = Mailbox(rx);
        loop {
            match process.run(&mut mailbox, handle.clone()).await {
                Ok(_) => break,
                Err(e) => log_error!("process {} failed: {}", P::NAME, e),
            }
        }
    });
    handle
}