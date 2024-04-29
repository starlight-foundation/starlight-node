use std::{io::ErrorKind, net::TcpListener};

use crate::{error, process::{self, Handle, Mailbox, Process}, util::Error};

use super::RpcReceiver;
pub struct RpcReceiver {
    listener: TcpListener
}

impl RpcReceiver {
    pub fn new(listener: TcpListener) -> Self {
        Self { listener }
    }
}

/// Is this `ErrorKind` fatal for the TCP listening socket?
fn is_tcp_fatal(kind: ErrorKind) -> bool {
    // For reliable operation the application should detect the network
    // errors defined for the protocol after accept() and treat them
    // like EAGAIN by retrying.  In the case of TCP/IP, these are
    // ENETDOWN, EPROTO, ENOPROTOOPT, EHOSTDOWN, ENONET, EHOSTUNREACH,
    // EOPNOTSUPP, and ENETUNREACH.
    // https://man7.org/linux/man-pages/man2/accept.2.html
    match kind {
        ErrorKind::NotFound => true,
        ErrorKind::PermissionDenied => true,
        ErrorKind::ConnectionRefused => false,
        ErrorKind::ConnectionReset => false,
        ErrorKind::ConnectionAborted => false,
        ErrorKind::NotConnected => false,
        ErrorKind::AddrInUse => true,
        ErrorKind::AddrNotAvailable => true,
        ErrorKind::BrokenPipe => false,
        ErrorKind::AlreadyExists => false,
        ErrorKind::WouldBlock => false,
        ErrorKind::InvalidInput => false,
        ErrorKind::InvalidData => false,
        ErrorKind::TimedOut => false,
        ErrorKind::WriteZero => false,
        ErrorKind::Interrupted => false,
        ErrorKind::Unsupported => false,
        ErrorKind::UnexpectedEof => false,
        ErrorKind::OutOfMemory => true,
        ErrorKind::Other => false,
        _ => false,
    }
}   

impl Process for RpcReceiver {
    const NAME: &'static str = "RpcReceiver";
    const RESTART_ON_CRASH: bool = true;

    fn run(&mut self, mailbox: &mut Mailbox, _: Handle) -> Result<(), Error> {
        for stream in self.listener.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(e) if is_tcp_fatal(e.kind()) => return Err(e),
                _ => continue
            };
            process::spawn(RpcReceiver::new(stream));
        }
        Err(error!("tcp listener finished"))
    }
}

