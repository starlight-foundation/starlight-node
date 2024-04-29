mod message;
pub use message::Message;

use std::future::Future;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;
use std::time::Duration;
use crate::util::Error;
use crate::{log_error, log_warn};

const BUF_SIZE: usize = 1024;
const SLEEP_MS_BEFORE_RETRY: usize = 20;

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

pub trait Process {
    const NAME: &'static str;
    const RESTART_ON_CRASH: bool;
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
                Err(e) => {
                    if !P::RESTART_ON_CRASH {
                        break;
                    }
                    log_error!("process {} failed: {}", P::NAME, e);
                    thread::sleep(Duration::from_millis(SLEEP_MS_BEFORE_RETRY));
                    log_warn!("restarting process {}", P::NAME);
                }
            }
        }
    });
    handle
}