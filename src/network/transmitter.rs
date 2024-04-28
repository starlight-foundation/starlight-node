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
    process::{Handle, Mailbox, Message, Process},
};

use super::{models::TelemetryNote, CenterMap, Endpoint, Note, Peer, Shred, ShredNote};

pub const MTU: usize = 1280;
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

pub struct Transmitter {
    socket: Arc<UdpSocket>,
    visible_ep: Endpoint,
    id: Identity,
    initial_peers: Arc<Vec<Endpoint>>,
    max_less: usize,
    max_greater: usize,
    get_weight: Box<dyn Fn(&Public) -> Amount + Send>,
    version: Version,
    allow_peers_with_private_ip_addresses: bool,
    allow_peers_with_node_external_ip_address: bool,
    peers: CenterMap<Public, Amount, Peer>
}

impl Transmitter {
    pub fn new(
        socket: Arc<UdpSocket>,
        visible_ep: Endpoint,
        id: Identity,
        initial_peers: Arc<Vec<Endpoint>>,
        max_less: usize,
        max_greater: usize,
        get_weight: Box<dyn Fn(&Public) -> Amount + Send>,
        version: Version,
        allow_peers_with_private_ip_addresses: bool,
        allow_peers_with_node_external_ip_address: bool,
    ) -> Self {
        let weight = get_weight(&id.public);
        Self {
            socket,
            visible_ep,
            id,
            initial_peers,
            max_less,
            max_greater,
            get_weight,
            version,
            allow_peers_with_private_ip_addresses,
            allow_peers_with_node_external_ip_address,
            peers: CenterMap::new(weight, max_less, max_greater),
        }
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

    // Send telemetry messages at regular intervals
    fn on_interval(&mut self) {
        // Update my personal weight
        self.peers.update_center((self.get_weight)(&self.id.public));

        // Create a new telemetry message
        let tel_note = Box::new(TelemetryNote::new(
            self.id.private,
            Slot::now(),
            self.visible_ep,
            self.version,
        ));
        let msg = Note::TelemetryNote(tel_note);
        let bytes = Arc::new(msg.serialize(MTU));

        // Broadcast the telemetry message to initial peers or a subset of peers
        if self.peers.is_empty() {
            self.broadcast_initial_peers(bytes);
        } else {
            self.broadcast_fanout(bytes);
        }
    }

    // Handle incoming telemetry messages
    fn on_tel_note(&mut self, tel_note: Box<TelemetryNote>) {
        // Filter out invalids
        if tel_note.verify().is_err() {
            return;
        }
        // Don't accept telemetry from myself :)
        if tel_note.from == self.id.public {
            return;
        }
        // Check if the telemetry message version is compatible
        if !tel_note.version.is_compatible(self.version) {
            return;
        }
        // if we aren't allowed to contact private IPs
        if !self.allow_peers_with_private_ip_addresses && !tel_note.ep.is_external() {
            return;
        }
        // if we aren't allowed to communicate with our own IP
        if !self.allow_peers_with_node_external_ip_address
            && tel_note.ep.addr == self.visible_ep.addr
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
                        weight: (self.get_weight)(&tel_note.from),
                        last_contact: now,
                    },
                )
            }
        };

        // Broadcast the telemetry message to other peers if necessary
        if should_broadcast {
            let msg = Note::TelemetryNote(tel_note);
            let bytes = Arc::new(msg.serialize(MTU));
            self.broadcast_fanout(bytes);
        }
    }

    async fn on_msg(&mut self, msg: Message) {
        match msg {
            // Shred notes sent back from `Restorer`
            Message::ShredNote(shred_note) => {
                // Broadcast the shred message to a subset of peers
                let bytes = Arc::new(Note::ShredNote(shred_note).serialize(MTU));
                self.broadcast_fanout(bytes);
            },
            Message::TelemetryNote(tel_note) => {
                self.on_tel_note(tel_note);
            },
            _ => {}
        }
    }
}

impl Process for Transmitter {
    const NAME: &'static str = "Transmitter";
    // Run the transmitter
    async fn run(&mut self, mailbox: &mut Mailbox, _: Handle) -> Result<(), Error> {
        // Spawn a task to send messages at an interval
        let mut interval = tokio::time::interval(Duration::from_secs(PEER_UPDATE_INTERVAL));
        loop {
            tokio::select! {
                msg = mailbox.recv() => {
                    self.on_msg(msg).await;
                },
                _ = interval.tick() => {
                    self.on_interval();
                }
            }
        }
    }
}

