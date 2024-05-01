mod config;
#[macro_use]
pub mod log;

use crate::network::{Assembler, Broadcaster, Endpoint, Receiver, Transmitter};
use crate::process::{self, Handle};
use crate::protocol::{Amount, Scheduler};
use crate::rpc::RpcServer;
use crate::state::{Block, State};
use crate::waiting::{OpenPool, TxPool};
use crate::{
    keys::{Identity, Private, Seed},
    util::{Error, Version},
};
use config::Config;
use nanoserde::{DeJson, SerJson};
use std::net::{TcpListener, UdpSocket};
use std::process::exit;
use std::sync::Arc;
use std::thread;
use std::{
    fs::{self, File},
    io::Write
};

const VERSION: Version = Version::new(0, 1, 0);
const CONFIG_FILE: &str = "config.json";

pub fn start() {
    log_info!("Starting Starlight node version {}", VERSION);
    
    // Initialize the configuration for the Starlight node
    let config = match fs::read_to_string(CONFIG_FILE) {
        Ok(config) => {
            log_info!("Loaded config from {}", CONFIG_FILE);
            match DeJson::deserialize_json(&config) {
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
                f.write_all(SerJson::serialize_json(&config).as_bytes())?;
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

    // Derive node identity from the configuration
    let private = config.node_seed.derive(0);
    let public = private.to_public();
    log_info!("Using public key {}", public);
    log_info!("Using address {}", public.to_address());

    // Setup network identity and UDP socket for communication
    let id = Identity { private, public };
    let network_socket = Arc::new(match UdpSocket::bind(
        config.node_bind_endpoint.to_socket_addr()
    ) {
        Ok(socket) => socket,
        Err(e) => {
            log_error!("Failed to bind to {}: {}", config.node_bind_endpoint, e);
            exit(1);
        }
    });

    // Start the network broadcaster and transmitter process
    let broadcaster = process::spawn_endless(Broadcaster::new(
        network_socket.clone()
    ));
    let transmitter = process::spawn_endless(Transmitter::new(
        network_socket.clone(),
        config.node_external_endpoint,
        id,
        Arc::new(config.initial_peers),
        config.max_less_peers,
        config.max_greater_peers,
        Box::new(|_| Amount::from_raw(1)),
        VERSION,
        config.allow_peers_with_private_ip_addresses,
        config.allow_peers_with_node_external_ip_address,
        broadcaster
    ));

    // Initialize blockchain state
    let genesis = Block::genesis(id);
    let state = process::spawn(match State::new(
        id,
        &config.data_dir,
        Arc::new(genesis)
    ) {
        Ok(state) => state,
        Err(e) => {
            log_error!("Failed to create state: {}", e);
            exit(1);
        }
    });

    // Initialize and start the RPC server
    let rpc_socket = match TcpListener::bind(
        config.rpc_endpoint.to_socket_addr()
    ) {
        Ok(socket) => socket,
        Err(e) => {
            log_error!("Failed to bind to {}: {}", config.rpc_endpoint, e);
            exit(1);
        }
    };
    let rpc = RpcServer::new(state.clone(), rpc_socket);
    process::spawn_solitary(rpc);
    log_info!("RPC listening on tcp://{}", config.rpc_endpoint);
    
    // Connect to database
    let db = process::connect_remote(config.db_endpoint);

    // Initialize transaction pools
    let n_cores = thread::available_parallelism().unwrap().get();
    let tx_pools: Vec<Handle> = (0..n_cores).map(|_| process::spawn(TxPool::new(
        config.tx_pool_size / n_cores,
        db.clone()
    ))).collect();
    let open_pool = process::spawn(OpenPool::new(config.open_pool_size, state));

    // Create scheduler to synchronize open pool and transaction pools
    let notified = Some(open_pool.clone()).into_iter().chain(tx_pools.iter().cloned()).collect();
    process::spawn_solitary_endless(Scheduler::new(notified));

    // Create assembler
    let assembler = process::spawn(Assembler::new());

    // Start the network receiver process
    process::spawn(Receiver::new(
        network_socket,
        transmitter,
        assembler,
        tx_pools,
        open_pool
    ));
    log_info!("SLP listening on udp://{}", config.node_bind_endpoint);
    log_info!(
        "SLP external endpoint is udp://{}",
        config.node_external_endpoint
    );
    if config.node_external_endpoint.addr == [127, 0, 0, 1] {
        log_warn!("SLP external endpoint is localhost; this node will not be able to communicate over the network");
    }
}