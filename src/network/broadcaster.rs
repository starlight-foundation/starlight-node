use std::{net::UdpSocket, sync::Arc};

use crate::{process::{Handle, Mailbox, Message, Process, ProcessInfallible}, util::Error};

pub struct Broadcaster {
    socket: Arc<UdpSocket>
}

impl Broadcaster {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self { socket }
    }
}

impl ProcessInfallible for Broadcaster {
    fn run(&mut self, mut mailbox: Mailbox, handle: Handle) -> ! {
        loop {
            let msg = match mailbox.recv() {
                Message::Broadcast(v) => v,
                _ => continue                
            };
            let endpoints = msg.0;
            let msg = msg.1;
            for ep in endpoints.iter() {
                _ = self.socket.send_to(&msg, ep.to_socket_addr());
            }
        }
    }
}