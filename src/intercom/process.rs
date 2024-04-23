use std::{array::from_fn, sync::Mutex};

use tokio::sync::mpsc::{self, Sender, Receiver};

use super::{Mailbox, Message};

const BUF_SIZE: usize = 256;

struct Processes<const N: usize> {
    senders: [Sender<Message>; N],
    receivers: [Mutex<Option<Receiver<Message>>>; N]
}

impl<const N: usize> Processes<N> {
    fn new() -> Self {
        let mut senders = from_fn(|_| mpsc::channel(0).0);
        let mut receivers = from_fn(|_| Mutex::new(None));
        for i in 0..N {
            let (tx, rx) = mpsc::channel(N);
            senders[i] = tx;
            receivers[i].lock().unwrap().replace(rx); 
        }
        Self {
            senders,
            receivers,
        }
    }
}

#[static_init::dynamic]
static PROCESSES: Processes<7> = Processes::new();

#[derive(Clone, Copy)]
pub enum Process {
    Scheduler = 0,
    Network = 1,
    OpenPool = 2,
    TxPool = 3,
    Restorer = 4,
    Bank = 5,
    Chain = 6
}

impl Process {
    pub fn take_mailbox(self) -> Option<Mailbox> {
        Some(Mailbox(PROCESSES.receivers[self as usize].lock().unwrap().take()?))
    }
    pub async fn broadcast(msg: Message) {
        for sender in PROCESSES.senders.iter() {
            sender.send(msg.clone()).await.unwrap();
        }
    }
    pub async fn send(self, msg: Message) {
        PROCESSES.senders[self as usize].send(msg).await.unwrap()
    }
}