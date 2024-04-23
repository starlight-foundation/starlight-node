use tokio::sync::mpsc::Receiver;

use super::Message;

pub struct Mailbox(pub(super) Receiver<Message>);

impl Mailbox {
    pub async fn recv(&mut self) -> Message {
        self.0.recv().await.unwrap()
    }
}

