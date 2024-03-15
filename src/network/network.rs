use std::{sync::Arc, time::Duration};

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

use crate::{
    blocks::{Amount, Slot},
    keys::{Private, Public, Signature},
    node::Error,
};

use super::{CenterMap, Logical, Peer, Telemetry, Version};

const VERSION: Version = Version::new(0, 1, 0);
const MTU: usize = 1280;
const PEER_UPDATE: u64 = 15;
const PEER_TIMEOUT: u64 = 2 * PEER_UPDATE;
fn fanout(n: usize) -> usize {
    (n as f64).powf(0.58) as usize
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum MsgData {
    Telemetry(Telemetry),
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct Msg {
    from: Public,
    signature: Signature,
    data: MsgData,
}

pub struct Network {
    logical: Logical,
    public: Public,
    private: Private,
    socket: Arc<UdpSocket>,
    peers: CenterMap<Public, Amount, Peer>,
    initial_peers: Vec<Logical>,
    get_weight: Box<dyn Fn(&Public) -> Amount>,
}

impl Network {
    pub async fn new(
        logical: Logical,
        public: Public,
        private: Private,
        initial_peers: Vec<Logical>,
        half_max_peers: usize,
        get_weight: Box<dyn Fn(&Public) -> Amount>,
    ) -> Result<Self, Error> {
        Ok(Self {
            logical,
            public,
            private,
            socket: Arc::new(UdpSocket::bind(logical.to_socket_addr()).await?),
            peers: CenterMap::new(get_weight(&public), half_max_peers),
            initial_peers,
            get_weight,
        })
    }

    fn on_telemetry(&mut self, from: Public, tel: Telemetry) -> bool {
        if !tel.version.is_compatible(VERSION) {
            return false;
        }

        let now = Slot::now();
        match self.peers.get_mut(&from) {
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
                from,
                Peer {
                    version: tel.version,
                    logical: tel.logical,
                    weight: (self.get_weight)(&from),
                    last_contact: now,
                },
            ),
        }
    }

    fn broadcast_fanout(&mut self, msg: Arc<Vec<u8>>) {
        let mut peer_count = self.peers.len();
        let mut broadcast_left = fanout(peer_count);
        let mut rng = rand::thread_rng();
        let now = Slot::now();
        while broadcast_left > 0 && peer_count > 0 {
            let i = rng.gen_range(0..peer_count);
            let peer = &self.peers[i];
            if now.saturating_sub(peer.last_contact) >= PEER_TIMEOUT {
                self.peers.remove_index(i);
                peer_count -= 1;
                continue;
            }
            let logical = peer.logical;
            let socket = self.socket.clone();
            let msg = msg.clone();
            tokio::spawn(async move {
                _ = socket.send_to(&msg, logical.to_socket_addr()).await;
            });
            broadcast_left -= 1;
        }
    }

    fn on_bytes(&mut self, bytes: &[u8]) {
        let msg: Msg = match bincode::deserialize(bytes) {
            Ok(msg) => msg,
            Err(_) => return,
        };
        let hash = match msg.data {
            MsgData::Telemetry(tel) => tel.hash(),
        };
        if msg.from.verify(&hash, &msg.signature).is_err() {
            return;
        }
        let should_broadcast = match msg.data {
            MsgData::Telemetry(tel) => self.on_telemetry(msg.from, tel),
        };
        if should_broadcast {
            self.broadcast_fanout(Arc::new(bincode::serialize(&msg).unwrap()));
        }
    }

    fn on_interval(&mut self) {
        let tel = Telemetry {
            logical: self.logical,
            version: VERSION,
            slot: Slot::now(),
        };
        let msg = Arc::new(bincode::serialize(&Msg {
            from: self.public,
            signature: self.private.sign(&tel.hash()),
            data: MsgData::Telemetry(tel),
        }).unwrap());
        if self.peers.len() == 0 {
            for &logical in &self.initial_peers {
                let socket = self.socket.clone();
                let msg = msg.clone();
                tokio::spawn(async move {
                    _ = socket.send_to(&msg, logical.to_socket_addr()).await;
                });
            }
            return;
        }
        self.broadcast_fanout(msg);
    }

    pub async fn run(mut self) -> Result<(), Error> {
        let mut interval = tokio::time::interval(
            Duration::from_secs(PEER_UPDATE)
        );
        loop {
            let mut buf = [0; MTU];
            tokio::select! {
                _ = interval.tick() => self.on_interval(),
                v = self.socket.recv_from(&mut buf) => match v {
                    Ok((n, _)) => self.on_bytes(&buf[..n]),
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock => continue,
                        std::io::ErrorKind::Interrupted => continue,
                        _ => {
                            return Err(Error::from(e));
                        }
                    },
                }
            }
        }
    }
}
