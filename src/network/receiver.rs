use std::sync::Arc;

use tokio::net::UdpSocket;

use crate::{process::{Handle, Mailbox, Message, Process}, util::{DefaultInitVec, Error}};

use super::{models::Note, MTU};

pub struct Receiver {
    socket: Arc<UdpSocket>,
    transmitter: Handle,
    assembler: Handle,
    tx_pool: Handle,
    open_pool: Handle,
}

impl Receiver {
    // Create a new instance of the Receiver struct
    pub async fn new(
        socket: Arc<UdpSocket>, 
        transmitter: Handle,
        assembler: Handle,
        tx_pool: Handle,
        open_pool: Handle,
    ) -> Result<Self, Error> {
        Ok(Self {
            socket,
            transmitter,
            assembler,
            tx_pool,
            open_pool,
        })
    }
}

impl Process for Receiver {
    const NAME: &'static str = "Receiver";
    // Run the receiver
    async fn run(&mut self, _: &mut Mailbox, _: Handle) -> Result<(), Error> {
        let socket = self.socket.clone();

        // Spawn a task to receive notes from the socket
        let mut buf = Vec::default_init(MTU);
        loop {
            let n = match socket.recv_from(&mut buf).await {
                Ok((n, _)) => n,
                Err(e) => match e.kind() {
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted => {
                        continue
                    }
                    _ => return Err::<(), Error>(Error::from(e)),
                },
            };
            let bytes = &buf[..n];
            let note = match Note::deserialize(bytes, MTU) {
                Ok(note) => note,
                Err(e) => continue
            };
            match note {
                Note::Tel(tel_note) => {
                    self.transmitter.send(Message::TelemetryNote(tel_note)).await;
                }
                Note::Shred(shred_note) => {
                    self.assembler.send(Message::ShredNote(shred_note)).await;
                }
                Note::Transaction(tx) => {
                    self.tx_pool.send(Message::Transaction(tx)).await;
                }
                Note::Open(open) => {
                    self.open_pool.send(Message::Open(open)).await;
                }
            }
        }
    }
}
