use std::{thread, time::Duration};

use kanal::Receiver;
use bincode::{de::Decoder, enc::Encoder, error::{DecodeError, EncodeError}, Decode, Encode};

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
}