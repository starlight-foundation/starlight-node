use std::{sync::Arc, net::UdpSocket};
use crate::{process::{Handle, Mailbox, Message, Process}, util::{DefaultInitVec, Error}};
use super::{models::Note, MTU};
use rand::seq::SliceRandom;

pub struct Receiver {
    socket: Arc<UdpSocket>,
    transmitter: Handle,
    assembler: Handle,
    tx_pools: Vec<Handle>,
    open_pool: Handle,
}

impl Receiver {
    // Create a new instance of the Receiver struct
    pub fn new(
        socket: Arc<UdpSocket>, 
        transmitter: Handle,
        assembler: Handle,
        tx_pools: Vec<Handle>,
        open_pool: Handle,
    ) -> Self {
        Self {
            socket,
            transmitter,
            assembler,
            tx_pools,
            open_pool,
        }
    }
}

impl Process for Receiver {
    const NAME: &'static str = "Receiver";
    const RESTART_ON_CRASH: bool = true;

    // Run the receiver
    fn run(&mut self, _: Mailbox, _: Handle) -> Result<(), Error> {
        let socket = self.socket.clone();

        // Spawn a task to receive notes from the socket
        let mut buf = Vec::default_init(MTU);
        loop {
            let n = match socket.recv_from(&mut buf) {
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
                Note::TelemetryNote(tel_note) => {
                    self.transmitter.send(Message::TelemetryNote(tel_note));
                }
                Note::ShredNote(shred_note) => {
                    self.assembler.send(Message::ShredNote(shred_note));
                }
                Note::Transaction(tx) => {
                    self.tx_pools.choose(&mut rand::thread_rng()).unwrap().send(
                        Message::Transaction(tx)
                    );
                }
                Note::Open(open) => {
                    self.open_pool.send(Message::Open(open));
                }
            }
        }
    }
}
