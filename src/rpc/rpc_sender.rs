use std::{io::Write, net::TcpStream};

use crate::{process::{Handle, Mailbox, Message, Process}, util::{self, Error}};

use super::RpcResponse;

pub struct RpcSender {
    stream: TcpStream
}

impl RpcSender {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }
}

impl Process for RpcSender {
    const NAME: &'static str = "RpcSender";
    const RESTART_ON_CRASH: bool = false;
    
    fn run(&mut self, mailbox: &mut Mailbox, handle: Handle) -> Result<(), Error> {
        let mut buf = Vec::with_capacity(4096);
        loop {
            let rpc_response = match mailbox.recv() {
                Message::RpcResponse(v) => v,
                _ => continue
            };
            let (id, response): (u64, RpcResponse) = *rpc_response;
            buf.extend_from_slice(&0u32.to_le_bytes());
            util::encode_into_writer(&mut buf, &response).unwrap();
            let len_bytes = (buf.len() as u32 - 4).to_le_bytes();
            buf[0..4].copy_from_slice(&len_bytes);
            self.stream.write_all(&buf)?;
        }
    }
    
}

