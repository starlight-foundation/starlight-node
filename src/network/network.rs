use std::{sync::Arc, time::Duration};

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::{
    net::UdpSocket,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{
    blocks::{Amount, Slot},
    error,
    keys::{Private, Public, Signature},
    util::{self, Error},
};

use super::{config, models::TelemetryMsg, CenterMap, Logical, Msg, Peer, Shred, ShredMsg};

pub struct Network {
    logical: Logical,
    public: Public,
    private: Private,
    socket: Arc<UdpSocket>,
    peers: CenterMap<Public, Amount, Peer>,
    initial_peers: Vec<Logical>,
    get_weight: Box<dyn Fn(&Public) -> Amount>,
    shred_msg_tx: UnboundedSender<Box<ShredMsg>>,
    shred_msg_rx: UnboundedReceiver<Box<ShredMsg>>,
}

impl Network {
    // Create a new instance of the Network struct
    pub async fn new(
        logical: Logical,
        public: Public,
        private: Private,
        initial_peers: Vec<Logical>,
        half_max_peers: usize,
        get_weight: Box<dyn Fn(&Public) -> Amount>,
        shred_msg_tx: UnboundedSender<Box<ShredMsg>>,
        shred_msg_rx: UnboundedReceiver<Box<ShredMsg>>,
    ) -> Result<Self, Error> {
        Ok(Self {
            logical,
            public,
            private,
            socket: Arc::new(UdpSocket::bind(logical.to_socket_addr()).await?),
            peers: CenterMap::new(get_weight(&public), half_max_peers),
            initial_peers,
            get_weight,
            shred_msg_tx,
            shred_msg_rx,
        })
    }

    // Broadcast a message to a subset of peers using fanout
    fn broadcast_fanout(&mut self, msg: Arc<Vec<u8>>) {
        let mut peer_count = self.peers.len();
        let mut broadcast_left = config::fanout(peer_count);
        let mut rng = rand::thread_rng();
        let now = Slot::now();
        while broadcast_left > 0 && peer_count > 0 {
            let i = rng.gen_range(0..peer_count);
            let peer = &self.peers[i];
            if now.saturating_sub(peer.last_contact) >= config::PEER_TIMEOUT {
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

    // Broadcast a message to initial peers
    fn broadcast_initial_peers(&self, bytes: Arc<Vec<u8>>) {
        for &logical in &self.initial_peers {
            let socket = self.socket.clone();
            let bytes = bytes.clone();
            tokio::spawn(async move {
                _ = socket.send_to(&bytes, logical.to_socket_addr()).await;
            });
        }
    }

    // Handle incoming telemetry messages
    fn on_tel_msg(&mut self, tel_msg: Box<TelemetryMsg>) {
        // Check if the telemetry message version is compatible
        if !tel_msg.version.is_compatible(config::VERSION) {
            return;
        }

        let now = Slot::now();
        let should_broadcast = match self.peers.get_mut(&tel_msg.from) {
            Some(peer) => {
                // Update the peer's information if enough time has passed since the last update
                if now.saturating_sub(peer.last_contact) >= config::PEER_UPDATE {
                    peer.version = tel_msg.version;
                    peer.logical = tel_msg.logical;
                    peer.last_contact = now;
                    true
                } else {
                    false
                }
            }
            None => {
                // Insert a new peer if it doesn't exist
                self.peers.insert(
                    tel_msg.from,
                    Peer {
                        version: tel_msg.version,
                        logical: tel_msg.logical,
                        weight: (self.get_weight)(&tel_msg.from),
                        last_contact: now,
                    },
                )
            }
        };

        // Broadcast the telemetry message to other peers if necessary
        if should_broadcast {
            let msg = Msg::Tel(tel_msg);
            let bytes = Arc::new(msg.serialize());
            self.broadcast_fanout(bytes);
        }
    }

    // Handle incoming shred messages
    fn on_shred_msg(&mut self, shred: Box<ShredMsg>) {
        // Send the shred message to the shred message channel
        _ = self.shred_msg_tx.send(shred);
    }

    // Handle incoming messages
    fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Tel(tel_msg) => self.on_tel_msg(tel_msg),
            Msg::Shred(shred_msg) => self.on_shred_msg(shred_msg),
        }
    }

    // Send telemetry messages at regular intervals
    fn on_interval(&mut self) {
        // Create a new telemetry message
        let tel_msg = Box::new(TelemetryMsg::sign_new(
            self.private,
            Slot::now(),
            self.logical,
            config::VERSION,
        ));
        let msg = Msg::Tel(tel_msg);
        let bytes = Arc::new(msg.serialize());

        // Broadcast the telemetry message to initial peers or a subset of peers
        if self.peers.is_empty() {
            self.broadcast_initial_peers(bytes);
        } else {
            self.broadcast_fanout(bytes);
        }
    }

    // Handle incoming shred messages from the receiver
    fn on_shred_rx(&mut self, shred_msg: Box<ShredMsg>) {
        // Broadcast the shred message to a subset of peers
        let bytes = Arc::new(Msg::Shred(shred_msg).serialize());
        self.broadcast_fanout(bytes);
    }

    // Run the network
    pub async fn run(mut self) -> Result<(), Error> {
        let mut interval = tokio::time::interval(Duration::from_secs(config::PEER_UPDATE));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let socket = self.socket.clone();

        // Spawn a task to receive messages from the socket
        tokio::spawn(async move {
            let mut buf = [0; config::MTU];
            loop {
                let n = match socket.recv_from(&mut buf).await {
                    Ok((n, _)) => n,
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted => {
                            continue
                        }
                        _ => return Err::<(), Error>(Error::from(e)),
                    },
                };
                let bytes = &buf[..n];
                if let Ok(msg) = Msg::deserialize(bytes) {
                    if msg.verify().is_ok() {
                        tx.send(msg)?;
                    }
                }
            }
        });

        loop {
            tokio::select! {
                // Handle interval ticks for sending telemetry messages
                _ = interval.tick() => self.on_interval(),
                // Handle incoming messages from the socket
                msg = rx.recv() => self.on_msg(msg.ok_or(error!("socket crashed"))?),
                // Handle incoming shred messages from the receiver
                v_maybe = self.shred_msg_rx.recv() => {
                    let shred_msg = v_maybe.ok_or(error!("shred_rx gone"))?;
                    self.on_shred_rx(shred_msg);
                }
            }
        }
    }
}
