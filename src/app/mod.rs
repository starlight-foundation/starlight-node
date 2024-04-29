mod config;
#[macro_use]
pub mod log;

use crate::network::{Assembler, Endpoint, Receiver, Transmitter};
use crate::process;
use crate::protocol::Amount;
use crate::rpc::RpcServer;
use crate::state::{Block, State};
use crate::waiting::{OpenPool, TxPool};
use crate::{
    keys::{Identity, Private, Seed},
    util::{Error, Version},
};
use config::Config;
use std::net::UdpSocket;
use std::sync::Arc;
use std::{
    fs::{self, File},
    io::Write
};

const VERSION: Version = Version::new(0, 1, 0);
const CONFIG_FILE: &str = "config.toml";

pub async fn start() {
    log_info!("Starting Starlight node version {}", VERSION);
    let config = match fs::read_to_string(CONFIG_FILE) {
        Ok(config) => {
            log_info!("Loaded config from {}", CONFIG_FILE);
            match serde_json::from_str(&config) {
                Ok(config) => config,
                Err(e) => {
                    log_error!("Failed to parse config: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(_) => {
            let config = Config::new();
            match (|| -> Result<(), Error> {
                let mut f = File::create(CONFIG_FILE)?;
                f.write_all(serde_json::to_string(&config).unwrap().as_bytes())?;
                Ok(())
            })() {
                Ok(_) => {
                    log_warn!("Failed to load config; created new at {}", CONFIG_FILE);
                }
                Err(e) => {
                    log_warn!("Failed to create config file at {}: {}", CONFIG_FILE, e);
                }
            }
            config
        }
    };
    let private = config.node_seed.derive(0);
    let public = private.to_public();
    log_info!("Using public key {}", public);
    log_info!("Using address {}", public.to_address());
    let rpc = RpcServer::new(config.rpc_endpoint);
    process::spawn(rpc);
    log_info!("RPC listening on tcp://{}", config.rpc_endpoint);
    let id = Identity { private, public };
    let socket = match UdpSocket::bind(
        config.node_bind_endpoint.to_socket_addr()
    ) {
        Ok(socket) => socket,
        Err(e) => {
            log_error!("Failed to bind to {}: {}", config.node_bind_endpoint, e);
            std::process::exit(1);
        }
    };
    let socket = Arc::new(socket);
    let transmitter = process::spawn(Transmitter::new(
        socket.clone(),
        config.node_external_endpoint,
        id,
        Arc::new(config.initial_peers),
        config.max_less_peers,
        config.max_greater_peers,
        Box::new(|_| Amount::from_raw(1)),
        VERSION,
        config.allow_peers_with_private_ip_addresses,
        config.allow_peers_with_node_external_ip_address,
    ));
    let genesis = Block::genesis(private);
    let state = match State::new(
        id,
        &config.data_dir,
        Arc::new(genesis)
    ) {
        Ok(state) => state,
        Err(e) => {
            log_error!("Failed to create state: {}", e);
            std::process::exit(1);
        }
    };
    let state = process::spawn(state);
    let tx_pool = process::spawn(TxPool::new(config.tx_pool_size, state.clone()));
    let open_pool = process::spawn(OpenPool::new(config.open_pool_size, state));
    let assembler = process::spawn(Assembler::new());
    process::spawn(Receiver::new(
        socket,
        transmitter,
        assembler,
        tx_pool,
        open_pool
    ));
    log_info!("SLP listening on udp://{}", config.node_bind_endpoint);
    log_info!(
        "SLP external endpoint is udp://{}",
        config.node_external_endpoint
    );
    if config.node_external_endpoint.addr == [127, 0, 0, 1] {
        log_warn!("SLP external endpoint is localhost; this node will not be able to communicate over the Internet");
    }
}
