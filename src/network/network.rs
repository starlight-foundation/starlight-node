use rand::Rng;
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, UdpSocket};

use crate::{
    blocks::{Amount, Slot},
    keys::Public,
    node::Error,
};

use super::{CenterMap, Logical, Peer, Telemetry, Version};

const VERSION: Version = Version::new(0, 1, 0);
const MTU: usize = 1280;
const PEER_UPDATE: u64 = 30;
const PEER_TIMEOUT: u64 = 2 * PEER_UPDATE;
fn fanout(n: usize) -> usize {
    (n as f64).sqrt() as usize
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum Message {
    Telemetry(Telemetry),
}

pub struct Network {
    peers: CenterMap<Public, Amount, Peer>,
    socket: UdpSocket,
    my_public: Public,
    initial_peers: Vec<(Public, Logical)>,
    get_weight: Box<dyn Fn(&Public) -> Amount>,
}

impl Network {
    pub fn new(
        listen_logical: Logical,
        my_public: Public,
        initial_peers: Vec<(Public, Logical)>,
        max_peers: usize,
        get_weight: Box<dyn Fn(&Public) -> Amount>,
    ) -> Result<Network, Error> {
        let socket = UdpSocket::bind(listen_logical)?;
        let mut peers = CenterMap::new(get_weight(&my_public), max_peers / 2);
        let now = Slot::now();
        for (public, logical) in initial_peers.iter() {
            peers.insert(
                *public,
                Peer {
                    weight: get_weight(&public),
                    last_contact: now,
                    logical: *logical,
                    version: Version::unknown(),
                },
            );
        }
        Ok(Network {
            peers,
            socket,
            my_public,
            initial_peers,
            get_weight,
        })
    }

    fn broadcast_bytes(&mut self, bytes: &[u8]) {
        let num_peers = fanout(self.peers.len());
        let mut rng = rand::thread_rng();
        let mut peer_count = self.peers.len();
        let now = Slot::now();
        for _ in 0..num_peers {
            let i = rng.gen_range(0..peer_count);
            let peer = &self.peers[i];
            if now.saturating_sub(peer.last_contact) >= PEER_TIMEOUT {
                self.peers.remove_index(i);
                peer_count -= 1;
                continue;
            }
            _ = self.socket.send_to(&bytes, peer.logical);
        }
    }

    fn on_telemetry(&mut self, tel: Telemetry) -> bool {
        if !tel.version.is_compatible(VERSION) {
            return false;
        }
        if tel.verify().is_err() {
            return false;
        }

        let now = Slot::now();
        match self.peers.get_mut(&tel.public) {
            Some(peer) => {
                if now.saturating_sub(peer.last_contact) >= PEER_UPDATE {
                    peer.version = tel.version;
                    peer.logical = tel.logical;
                    peer.last_contact = now;
                    true
                } else {
                    false
                }
            }
            None => self.peers.insert(
                tel.public,
                Peer {
                    version: tel.version,
                    logical: tel.logical,
                    weight: (self.get_weight)(&tel.public),
                    last_contact: now,
                },
            ),
        }
    }

    fn on_message(&mut self, msg: Message) -> bool {
        match msg {
            Message::Telemetry(tel) => self.on_telemetry(tel),
        }
    }

    pub fn run(mut self) -> Result<(), Error> {
        loop {
            let mut buf = [0; MTU];
            let n = match self.socket.recv_from(&mut buf) {
                Ok((n, _)) => n,
                Err(e) => match e.kind() {
                    std::io::ErrorKind::WouldBlock => continue,
                    std::io::ErrorKind::Interrupted => continue,
                    _ => {
                        return Err(Error::from(e));
                    }
                },
            };
            let bytes = &buf[..n];
            let msg: Message = match bincode::deserialize(bytes) {
                Ok(msg) => msg,
                Err(_) => continue,
            };
            let should_broadcast = self.on_message(msg);
            if should_broadcast {
                self.broadcast_bytes(bytes);
            }
        }
    }
}
