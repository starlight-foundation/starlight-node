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
    intercom::{Mailbox, Message, Process},
};

use super::{models::TelemetryNote, CenterMap, Endpoint, Note, Peer, Shred, ShredNote};

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


pub struct NetworkConfig {
    pub bind_ep: Endpoint,
    pub visible_ep: Endpoint,
    pub id: Identity,
    pub initial_peers: Arc<Vec<Endpoint>>,
    pub max_less: usize,
    pub max_greater: usize,
    pub get_weight: Box<dyn Fn(&Public) -> Amount>,
    pub version: Version,
    pub allow_peers_with_private_ip_addresses: bool,
    pub allow_peers_with_node_external_ip_address: bool,
}

pub struct Network {
    mailbox: Mailbox,
    config: NetworkConfig,
    socket: Arc<UdpSocket>,
    peers: CenterMap<Public, Amount, Peer>,
    leader_mode: bool
}

impl Network {
    // Create a new instance of the Network struct
    pub async fn new(config: NetworkConfig) -> Result<Self, Error> {
        Ok(Self {
            mailbox: Process::Network.take_mailbox().unwrap(),
            peers: CenterMap::new((config.get_weight)(&config.id.public), config.max_less, config.max_greater),
            socket: Arc::new(UdpSocket::bind(config.bind_ep.to_socket_addr()).await?),
            config,
            leader_mode: false
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
        let initial_peers = self.config.initial_peers.clone();
        tokio::spawn(async move {
            for logical in initial_peers.iter() {
                _ = socket.send_to(&bytes, logical.to_socket_addr()).await;
            }
        });
    }

    // Handle incoming telemetry messages
    fn on_tel_note(&mut self, tel_note: Box<TelemetryNote>) {
        // Filter out invalids
        if tel_note.verify().is_err() {
            return;
        }
        // Don't accept telemetry from myself :)
        if tel_note.from == self.config.id.public {
            return;
        }
        // Check if the telemetry message version is compatible
        if !tel_note.version.is_compatible(self.config.version) {
            return;
        }
        // if we aren't allowed to contact private IPs
        if !self.config.allow_peers_with_private_ip_addresses && !tel_note.ep.is_external() {
            return;
        }
        // if we aren't allowed to communicate with our own IP
        if !self.config.allow_peers_with_node_external_ip_address
            && tel_note.ep.addr == self.config.visible_ep.addr
        {
            return;
        }

        let now = Slot::now();
        let should_broadcast = match self.peers.get_mut(&tel_note.from) {
            Some(peer) => {
                // Update the peer's information if enough time has passed since the last update
                if now.saturating_sub(peer.last_contact) >= PEER_UPDATE_INTERVAL {
                    peer.version = tel_note.version;
                    peer.endpoint = tel_note.ep;
                    peer.last_contact = now;
                    true
                } else {
                    false
                }
            }
            None => {
                // Insert a new peer if it doesn't exist
                self.peers.insert(
                    tel_note.from,
                    Peer {
                        version: tel_note.version,
                        endpoint: tel_note.ep,
                        weight: (self.config.get_weight)(&tel_note.from),
                        last_contact: now,
                    },
                )
            }
        };

        // Broadcast the telemetry message to other peers if necessary
        if should_broadcast {
            let msg = Note::Tel(tel_note);
            let bytes = Arc::new(msg.serialize(MTU));
            self.broadcast_fanout(bytes);
        }
    }

    // Send telemetry messages at regular intervals
    fn on_interval(&mut self) {
        // Update my personal weight
        self.peers.update_center((self.config.get_weight)(&self.config.id.public));

        // Create a new telemetry message
        let tel_note = Box::new(TelemetryNote::new(
            self.config.id.private,
            Slot::now(),
            self.config.visible_ep,
            self.config.version,
        ));
        let msg = Note::Tel(tel_note);
        let bytes = Arc::new(msg.serialize(MTU));

        // Broadcast the telemetry message to initial peers or a subset of peers
        if self.peers.is_empty() {
            self.broadcast_initial_peers(bytes);
        } else {
            self.broadcast_fanout(bytes);
        }
    }

    async fn on_msg(&mut self, msg: Message) {
        match msg {
            Message::TelemetryInterval => self.on_interval(),
            Message::StartLeaderMode => self.leader_mode = true,
            Message::EndLeaderMode => self.leader_mode = false,
            // Shred notes from the network
            Message::NewShredNote(shred_note) => {
                // Send to the Restorer for reassembly
                Process::Restorer.send(Message::NewShredNote(shred_note)).await;
            },
            // Shred notes sent back from `Restorer`
            Message::PleaseBroadcast(shred_note) => {
                // Broadcast the shred message to a subset of peers
                let bytes = Arc::new(Note::Shred(shred_note).serialize(MTU));
                self.broadcast_fanout(bytes);
            },
            Message::TelemetryNote(tel_note) => self.on_tel_note(tel_note),
            // Only process transactions in leader mode
            Message::Transaction(tx) if self.leader_mode => {
                Process::TxPool.send(Message::Transaction(tx)).await;
            },
            _ => {}
        }
    }

    // Run the network
    pub async fn run(mut self) -> Result<(), Error> {
        let socket = self.socket.clone();

        // Spawn a task to receive notes from the socket
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
                if let Ok(note) = Note::deserialize(bytes, MTU) {
                    let msg = match note {
                        Note::Tel(tel_note) => Message::TelemetryNote(tel_note),
                        Note::Shred(shred_note) => Message::NewShredNote(shred_note),
                        Note::Transaction(tx) => Message::Transaction(tx)
                    };
                    Process::Network.send(msg).await;
                }
            }
        });

        // Spawn a task to send messages at an interval
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(PEER_UPDATE_INTERVAL));
            loop {
                Process::Network.send(Message::TelemetryInterval).await;
                interval.tick().await;
            }
        });

        loop {
            let msg = self.mailbox.recv().await;
            self.on_msg(msg).await;
        }
    }
}