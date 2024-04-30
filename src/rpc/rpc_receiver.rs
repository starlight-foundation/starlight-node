use std::{io::Read, net::TcpStream};

use crate::{error, process::{Handle, Mailbox, Message, Process}, util::{self, DefaultInitVec, Error}};

use super::RpcRequest;

pub struct RpcReceiver {
    destination: Handle,
    rpc_sender: Handle,
    stream: TcpStream
}
impl RpcReceiver {
    pub fn new(destination: Handle, rpc_sender: Handle, stream: TcpStream) -> Self {
        Self { destination, rpc_sender, stream }
    }
}

impl Process for RpcReceiver {
    const NAME: &'static str = "RpcReceiver";
    const RESTART_ON_CRASH: bool = false;
    
    fn run(&mut self, mailbox: Mailbox, _: Handle) -> Result<(), Error> {
        let mut buf = Vec::default_init(4096);
        loop {
            let mut len = [0u8; 4];
            self.stream.read_exact(&mut len)?;
            let len = u32::from_le_bytes(len) as usize;
            if len > buf.len() {
                return Err(error!("too big"));
            }
            self.stream.read_exact(&mut buf[..len])?;
            let stream = self.stream.try_clone()?;
            let (id, cmd): (u64, RpcRequest) = match util::decode_from_slice(&buf[..len]) {
                Ok(v) => v,
                Err(_) => continue
            };
            self.destination.send(Message::RpcRequest(
                Box::new((self.rpc_sender.clone(), id, cmd))
            ));
        }
    }
}