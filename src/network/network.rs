use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{blocks::{Amount, Slot}, error::Error, keys::{Public, Signature}};

use super::{CenterMap, Peer, Telemetry, Version};

const VERSION: Version = Version::new(0, 1, 0);
const MTU: usize = 1280;
const PEER_UPDATE: u64 = 30;
const PEER_TIMEOUT: u64 = 2 * PEER_UPDATE;
fn fanout(n: usize) -> usize {
    (n as f64).sqrt() as usize
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum Message {
    Telemetry(Telemetry)
}

pub struct Network {
    peers: CenterMap<Public, Amount, Peer>,
    socket: UdpSocket,
    my_public: Public,
    initial_peers: Vec<(Public, SocketAddrV4)>,
    get_weight: Box<dyn Fn(&Public) -> Amount>
}

impl Network {
    pub fn new(
        host: &str,
        port: u16,
        my_public: Public,
        initial_peers: Vec<(Public, SocketAddrV4)>,
        max_peers: usize,
        get_weight: Box<dyn Fn(&Public) -> Amount>
    ) -> Result<Network, Error> {
        let addr = SocketAddr::new(host.parse()?, port);
        let socket = UdpSocket::bind(addr)?;
        let mut peers = CenterMap::new(
            get_weight(&my_public), max_peers / 2
        );
        let now = Slot::now();
        for (public, address) in initial_peers {
            peers.insert(public, Peer {
                weight: get_weight(&public),
                last_contact: now,
                address,
                version: Version::unknown()
            });
        }
        Ok(Network { peers, socket, my_public, initial_peers, get_weight })
    }

    fn broadcast_message(&mut self, msg: Message) {
        let num_peers = fanout(self.peers.len());
        let mut rng = rand::thread_rng();
        let mut peer_count = self.peers.len();
        let time = unix_time_secs();
        for _ in 0..num_peers {
            let i = rng.gen_range(0..peer_count);
            let peer = &self.peers[i];
            if time.saturating_sub(peer.last_contact) >= PEER_TIMEOUT {
                self.peers.swap_remove_index(i).unwrap();
                peer_count -= 1;
                continue;
            }
            _ = self.socket.send_to(
                &bincode::serialize(&msg).unwrap(), 
                SocketAddr::V4(peer.address)
            );
        }
    }

    fn on_telemetry(&mut self, tel: Telemetry) {
        if !tel.version.is_compatible(VERSION) {
            return;
        }
        if !tel.verify() {
            return;
        }

        let now = Slot::now();
        let should_broadcast = match self.peers.get_mut(&tel.public) {
            Some(peer) => {
                if now.saturating_sub(peer.last_contact) >= PEER_UPDATE {
                    peer.version = tel.version;
                    peer.address = tel.address;
                    peer.last_contact = now;
                    true
                } else {
                    false
                }
            }
            None => {
                self.peers.insert(
                    tel.public,
                    Peer {
                        version: tel.version,
                        address: tel.address,
                        weight: (self.get_weight)(&tel.public),
                        last_contact: now,
                    },
                )
            }
        };
    
        if should_broadcast {
            self.broadcast_message(tel);
        }
    }

    fn on_message(&mut self, msg: Message) {
        match msg {
            Message::Telemetry(tel) => on_telemetry(tel)
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
            let msg: Message = match bincode::deserialize(&buf[..n]) {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("Failed to deserialize message: {}", e);
                    continue;
                }
            };
            self.on_message(msg);
        }
    }
}
