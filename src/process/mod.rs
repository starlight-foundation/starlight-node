mod message;
mod handle;
mod mailbox;
mod oncebox;
pub use message::Message;
pub use handle::Handle;
pub use mailbox::Mailbox;
pub use oncebox::Oncebox;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use crate::network::Endpoint;
use crate::util::{self, Error, UninitVec};
use crate::{log_error, log_warn};

const SLEEP_MS_BEFORE_RETRY: u64 = 20;

pub fn sleep(dur: Duration) {
    std::thread::sleep(dur);
}

pub trait Process {
    const NAME: &'static str;
    const RESTART_ON_CRASH: bool;
    fn run(&mut self, mailbox: Mailbox, handle: Handle) -> Result<(), Error>;
}

/// Spawns a `Process`. Processes are the building blocks upon which
/// the program is built. They are single-threaded, and communicate
/// via message-passing: anyone with a `Handle` to a given `Process`
/// can send messages to it, which the process can receive by calling
/// `recv` on its `Mailbox`.
pub fn spawn<P: Process + Send + 'static>(mut process: P) -> Handle {
    let (tx, rx) = kanal::unbounded();
    let handle = Handle(tx.clone());
    thread::spawn(move || {
        let handle = Handle(tx);
        let mailbox = Mailbox(rx);
        loop {
            match process.run(mailbox.clone(), handle.clone()) {
                Ok(_) => break,
                Err(e) => {
                    if !P::RESTART_ON_CRASH {
                        break;
                    }
                    log_error!("process {} failed: {}", P::NAME, e);
                    thread::sleep(Duration::from_millis(SLEEP_MS_BEFORE_RETRY));
                    log_warn!("restarting process {}", P::NAME);
                }
            }
        }
        handle.deactivate();
    });
    handle
}

pub trait ProcessSolitary {
    const NAME: &'static str;
    const RESTART_ON_CRASH: bool;
    fn run(&mut self) -> Result<(), Error>;
}

/// Spawn a solitary process, which never receives messages.
pub fn spawn_solitary<P: ProcessSolitary + Send + 'static>(mut process: P) {
    thread::spawn(move || {
        loop {
            match process.run() {
                Ok(_) => break,
                Err(e) => {
                    if !P::RESTART_ON_CRASH {
                        break;
                    }
                    log_error!("process {} failed: {}", P::NAME, e);
                    thread::sleep(Duration::from_millis(SLEEP_MS_BEFORE_RETRY));
                    log_warn!("restarting process {}", P::NAME);
                }
            }
        }
    });
}

pub trait ProcessEndless {
    fn run(&mut self, mailbox: Mailbox, handle: Handle) -> !;
}

/// Spawns an endless process, which never finishes or errors.
pub fn spawn_endless<P: ProcessEndless + Send + 'static>(mut process: P) -> Handle {
    let (tx, rx) = kanal::unbounded();
    let handle = Handle(tx.clone());
    thread::spawn(move || {
        let handle = Handle(tx);
        let mailbox = Mailbox(rx);
        process.run(mailbox, handle);
    });
    handle
}

pub trait ProcessSolitaryEndless {
    fn run(&mut self) -> !;
}

/// Spawns a solitary, endless process, which never finishes, errors, or receives messages.
pub fn spawn_solitary_endless<P: ProcessSolitaryEndless + Send + 'static>(mut process: P) {
    thread::spawn(move || process.run());
}

fn recv_message(socket: &mut TcpStream) -> Result<Option<(Handle, Message)>, Error> {
    let mut len = [0u8; 4];
    socket.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    // safety: no uninitialized bytes are read
    let mut buf = unsafe { Vec::uninit(len) };
    socket.read_exact(&mut buf)?;
    Ok(util::decode_from_slice(&buf).ok())
}

fn send_message(socket: &mut TcpStream, msg: &Message) -> Result<(), Error> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u32.to_le_bytes());
    if util::encode_into_writer(&mut buf, &msg).is_err() {
        return Ok(())
    }
    let len = buf.len() as u32;
    buf[0..4].copy_from_slice(&len.to_le_bytes());
    socket.write_all(&mut buf)?;
    Ok(())
}

/// Connect to a remote process, specified by the given TCP `Endpoint`,
/// and returns a handle to it.
/// In case of network error, delivery of messages is not guaranteed.
pub fn connect_remote(ep: Endpoint) -> Handle {
    let (tx, rx) = kanal::unbounded();
    let handle = Handle(tx.clone());
    thread::spawn(move || {
        let mut last_msg = None;
        loop {
            if let Ok(mut socket) = TcpStream::connect(ep.to_socket_addr()) {
                if let Ok(mut socket_clone) = socket.try_clone() {
                    thread::spawn(move || {
                        while let Ok(msg_maybe) = recv_message(&mut socket_clone) {
                            if let Some((handle, msg)) = msg_maybe {
                                handle.send(msg);
                            }
                        }
                    });
                    for msg in last_msg.take().into_iter().chain(rx.clone()) {
                        if send_message(&mut socket, &msg).is_err() {
                            last_msg = Some(msg);
                            break;
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    handle
}