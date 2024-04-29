use std::{io::Read, net::TcpStream};

use crate::{error, process::{Handle, Mailbox, Process}, util::Error};

use super::Command;

pub struct RpcReceiver {
    stream: TcpStream
}

impl Process for RpcReceiver {
    const NAME: &'static str = "RpcReceiver";
    const RESTART_ON_CRASH: bool = false;
    
    fn run(&mut self, mailbox: &mut Mailbox, _: Handle) -> Result<(), Error> {
        let buf = Vec::default_init(4096);
        loop {
            let mut len = [0u8; 4];
            self.stream.read_exact(&mut len)?;
            let len = u32::from_le_bytes(len);
            if len > buf.len() {
                return Err(error!("too big"));
            }
            self.stream.read_exact(&mut buf[..len]);
            let stream = self.stream.try_clone()?;
            let cmd: Box<(u64, Command)> = bincode::deserialize(&buf[..len])?;
        }
    }

}