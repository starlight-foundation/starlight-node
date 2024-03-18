use std::{sync::Arc, time::Duration};

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

use crate::{
    blocks::{Amount, Slot}, error, keys::{Private, Public, Signature}, util::{self, Error}
};

use super::{CenterMap, Logical, Peer, Shred, Version};

const VERSION: Version = Version::new(0, 1, 0);
const MTU: usize = 1280;
const PEER_UPDATE: u64 = 15;
const PEER_TIMEOUT: u64 = 2 * PEER_UPDATE;
const MAGIC_NUMBER: [u8; 8] = [0x3f, 0xd1, 0x0f, 0xe2, 0x5e, 0x76, 0xfa, 0xe6];
fn fanout(n: usize) -> usize {
    if n < 8 {
        n
    } else if n < 16 {
        n / 2
    } else if n < 32 {
        n / 3
    } else if n < 64 {
        n / 4
    } else {
        (n as f64).powf(0.58) as usize
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ShredBroadcast {
    slot: Slot,
    shred: Shred,
}

#[derive(Serialize, Deserialize, Clone)]
struct Telemetry {
    slot: Slot,
    logical: Logical,
    version: Version,
}

#[derive(Serialize, Deserialize, Clone)]
enum MsgData {
    Telemetry(Telemetry),
    ShredBroadcast(ShredBroadcast)
}

#[derive(Serialize, Deserialize, Clone)]
struct Msg {
    from: Public,
    signature: Signature,
    data: MsgData,
}
impl Msg {
    fn serialize(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::with_capacity(MTU);
        bytes.extend_from_slice(&MAGIC_NUMBER);
        util::serialize_into(&mut bytes, self)?;
        Ok(bytes)
    }
    fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 8
        || bytes[0..8] != MAGIC_NUMBER
        || bytes.len() > MTU {
            return Err(error!("invalid message"));
        }
        Ok(util::deserialize(&bytes[8..])?)
    }
    fn verify(&self) -> Result<(), ()> {
        let hash = util::hash(&self.data);
        self.from.verify(&hash, &self.signature)
    }
}

pub struct Network {
    logical: Logical,
    public: Public,
    private: Private,
    socket: Arc<UdpSocket>,
    peers: CenterMap<Public, Amount, Peer>,
    initial_peers: Vec<Logical>,
    get_weight: Box<dyn Fn(&Public) -> Amount>,
    need_shred: Box<dyn Fn(Slot, usize, usize) -> bool>,
}

impl Network {
    pub async fn new(
        logical: Logical,
        public: Public,
        private: Private,
        initial_peers: Vec<Logical>,
        half_max_peers: usize,
        get_weight: Box<dyn Fn(&Public) -> Amount>,
        need_shred: Box<dyn Fn(Slot, usize, usize) -> bool>
    ) -> Result<Self, Error> {
        Ok(Self {
            logical,
            public,
            private,
            socket: Arc::new(UdpSocket::bind(logical.to_socket_addr()).await?),
            peers: CenterMap::new(get_weight(&public), half_max_peers),
            initial_peers,
            get_weight,
            need_shred
        })
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

    fn broadcast_initial_peers(&self, bytes: Arc<Vec<u8>>) {
        for &logical in &self.initial_peers {
            let socket = self.socket.clone();
            let bytes = bytes.clone();
            tokio::spawn(async move {
                _ = socket.send_to(&bytes, logical.to_socket_addr()).await;
            });
        }
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

    fn on_shred_broadcast(&mut self, from: Public, b: ShredBroadcast) -> bool {
        if !self.need_shred(b.slot, b.shred.get_batch_index(), b.shred.get_shred_index()) {
            return false;
        }
        self.shreds.insert(b.slot, b.shred);
        true
    }

    fn on_msg(&mut self, msg: Msg) {
        let should_broadcast = match msg.data {
            MsgData::Telemetry(tel) => self.on_telemetry(msg.from, tel),
            MsgData::ShredBroadcast(broadcast) => self.on_shred_broadcast(msg.from, broadcast),
        };
        if !should_broadcast {
            return;
        }
        let bytes = Arc::new(match msg.serialize() {
            Ok(bytes) => bytes,
            Err(_) => return,
        });
        self.broadcast_fanout(bytes);
    }

    fn on_interval(&mut self) {
        let tel = Telemetry {
            logical: self.logical,
            version: VERSION,
            slot: Slot::now(),
        };
        let mut bytes = Vec::with_capacity(MTU);
        bytes.extend_from_slice(&MAGIC_NUMBER);
        bincode::serialize_into(&mut bytes, &Msg {
            from: self.public,
            signature: self.private.sign(&tel.hash()),
            data: MsgData::Telemetry(tel),
        }).unwrap();
        let bytes = Arc::new(bytes);
        if self.peers.len() == 0 {
            self.broadcast_initial_peers(bytes);
        } else {
            self.broadcast_fanout(bytes);
        }
    }

    pub async fn run(mut self) -> Result<(), Error> {
        let mut interval = tokio::time::interval(Duration::from_secs(PEER_UPDATE));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let socket = self.socket.clone();
        tokio::spawn(async move {
            let mut buf = [0; MTU];
            loop {
                let n = match socket.recv_from(&mut buf).await {
                    Ok((n, _)) => n,
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock => continue,
                        std::io::ErrorKind::Interrupted => continue,
                        _ => {
                            return Err::<(), Error>(Error::from(e));
                        }
                    },
                };
                let bytes = &buf[..n];
                let msg = match Msg::deserialize(bytes) {
                    Ok(msg) => msg,
                    Err(_) => continue,
                };
                if msg.verify().is_err() {
                    continue;
                }
                tx.send(msg)?;
            }
        });
        loop {
            tokio::select! {
                _ = interval.tick() => self.on_interval(),
                msg = rx.recv() => self.on_msg(msg.ok_or(error!("socket crashed"))?),
            }
        }
    }
}
