use std::{sync::Arc, time::Duration};

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::{
    net::UdpSocket,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{
    error,
    keys::{Identity, Private, Public, Signature},
    protocol::{Amount, Slot, Transaction},
    util::{self, DefaultInitVec, Error, UninitVec, Version},
};

use super::{models::TelemetryMsg, CenterMap, Endpoint, Msg, Peer, Shred, ShredMsg};

const MTU: usize = 1280;
const PEER_UPDATE_INTERVAL: u64 = 15;
const PEER_TIMEOUT_INTERVAL: u64 = 3 * PEER_UPDATE_INTERVAL;
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

pub struct Network {
    visible_ep: Endpoint,
    id: Identity,
    socket: Arc<UdpSocket>,
    peers: CenterMap<Public, Amount, Peer>,
    initial_peers: Arc<Vec<Endpoint>>,
    get_weight: Box<dyn Fn(&Public) -> Amount>,
    transaction_tx: UnboundedSender<Box<Transaction>>,
    shred_msg_tx: UnboundedSender<Box<ShredMsg>>,
    shred_msg_rx: UnboundedReceiver<Box<ShredMsg>>,
    version: Version,
    allow_peers_with_private_ip_addresses: bool,
    allow_peers_with_node_external_ip_address: bool,
}

impl Network {
    // Create a new instance of the Network struct
    pub async fn new(
        bind_ep: Endpoint,
        visible_ep: Endpoint,
        id: Identity,
        initial_peers: Arc<Vec<Endpoint>>,
        max_less: usize,
        max_greater: usize,
        get_weight: Box<dyn Fn(&Public) -> Amount>,
        transaction_tx: UnboundedSender<Box<Transaction>>,
        shred_msg_tx: UnboundedSender<Box<ShredMsg>>,
        shred_msg_rx: UnboundedReceiver<Box<ShredMsg>>,
        version: Version,
        allow_peers_with_private_ip_addresses: bool,
        allow_peers_with_node_external_ip_address: bool,
    ) -> Result<Self, Error> {
        Ok(Self {
            visible_ep,
            id,
            socket: Arc::new(UdpSocket::bind(bind_ep.to_socket_addr()).await?),
            peers: CenterMap::new(get_weight(&id.public), max_less, max_greater),
            initial_peers,
            get_weight,
            transaction_tx,
            shred_msg_tx,
            shred_msg_rx,
            version,
            allow_peers_with_private_ip_addresses,
            allow_peers_with_node_external_ip_address,
        })
    }

    // Broadcast a message to a subset of peers using fanout
    fn broadcast_fanout(&mut self, msg: Arc<Vec<u8>>) {
        let mut peer_count = self.peers.len();
        let mut broadcast_left = fanout(peer_count);
        let mut endpoints = Vec::with_capacity(broadcast_left);
        let mut rng = rand::thread_rng();
        let now = Slot::now();
        while broadcast_left > 0 && peer_count > 0 {
            let i = rng.gen_range(0..peer_count);
            let peer = &self.peers[i];
            if now.saturating_sub(peer.last_contact) >= PEER_TIMEOUT_INTERVAL {
                self.peers.remove_index(i);
                peer_count -= 1;
                continue;
            }
            endpoints.push(peer.endpoint);
            broadcast_left -= 1;
        }
        let socket = self.socket.clone();
        tokio::spawn(async move {
            for endpoint in endpoints.iter() {
                _ = socket.send_to(&msg, endpoint.to_socket_addr()).await;
            }
        });
    }

    // Broadcast a message to initial peers
    fn broadcast_initial_peers(&self, bytes: Arc<Vec<u8>>) {
        let socket = self.socket.clone();
        let initial_peers = self.initial_peers.clone();
        tokio::spawn(async move {
            for logical in initial_peers.iter() {
                _ = socket.send_to(&bytes, logical.to_socket_addr()).await;
            }
        });
    }

    // Handle incoming telemetry messages
    fn on_tel_msg(&mut self, tel_msg: Box<TelemetryMsg>) {
        // Don't accept telemetry from myself :)
        if tel_msg.from == self.id.public {
            return;
        }
        // Check if the telemetry message version is compatible
        if !tel_msg.version.is_compatible(self.version) {
            return;
        }
        // if we aren't allowed to contact private IPs
        if !self.allow_peers_with_private_ip_addresses && !tel_msg.ep.is_external() {
            return;
        }
        // if we aren't allowed to communicate with our own IP
        if !self.allow_peers_with_node_external_ip_address
            && tel_msg.ep.addr == self.visible_ep.addr
        {
            return;
        }

        let now = Slot::now();
        let should_broadcast = match self.peers.get_mut(&tel_msg.from) {
            Some(peer) => {
                // Update the peer's information if enough time has passed since the last update
                if now.saturating_sub(peer.last_contact) >= PEER_UPDATE_INTERVAL {
                    peer.version = tel_msg.version;
                    peer.endpoint = tel_msg.ep;
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
                        endpoint: tel_msg.ep,
                        weight: (self.get_weight)(&tel_msg.from),
                        last_contact: now,
                    },
                )
            }
        };

        // Broadcast the telemetry message to other peers if necessary
        if should_broadcast {
            let msg = Msg::Tel(tel_msg);
            let bytes = Arc::new(msg.serialize(MTU));
            self.broadcast_fanout(bytes);
        }
    }

    // Handle incoming shred messages
    fn on_shred_msg(&self, shred: Box<ShredMsg>) {
        // Send the shred message to the shred message channel
        _ = self.shred_msg_tx.send(shred);
    }

    // Handle incoming transactions
    fn on_transaction(&self, tr: Box<Transaction>) {
        // Send the transaction to the transaction channel
        _ = self.transaction_tx.send(tr);
    }

    // Handle incoming messages
    fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Tel(tel_msg) => self.on_tel_msg(tel_msg),
            Msg::Shred(shred_msg) => self.on_shred_msg(shred_msg),
            Msg::Transaction(tr) => self.on_transaction(tr),
        }
    }

    // Send telemetry messages at regular intervals
    fn on_interval(&mut self) {
        // Update my personal weight
        self.peers.update_center((self.get_weight)(&self.id.public));

        // Create a new telemetry message
        let tel_msg = Box::new(TelemetryMsg::sign_new(
            self.id.private,
            Slot::now(),
            self.visible_ep,
            self.version,
        ));
        let msg = Msg::Tel(tel_msg);
        let bytes = Arc::new(msg.serialize(MTU));

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
        let bytes = Arc::new(Msg::Shred(shred_msg).serialize(MTU));
        self.broadcast_fanout(bytes);
    }

    // Run the network
    pub async fn run(mut self) -> Result<(), Error> {
        let mut interval = tokio::time::interval(Duration::from_secs(PEER_UPDATE_INTERVAL));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let socket = self.socket.clone();

        // Spawn a task to receive messages from the socket
        tokio::spawn(async move {
            let mut buf = Vec::default_init(MTU);
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
                if let Ok(msg) = Msg::deserialize(bytes, MTU) {
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
