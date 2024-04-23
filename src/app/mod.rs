mod config;
#[macro_use]
mod log;

use crate::network::{Endpoint, Network, NetworkConfig};
use crate::protocol::Amount;
use crate::rpc::Rpc;
use crate::{
    keys::{Identity, Private, Seed},
    util::{Error, Version},
};
use config::Config;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{
    fs::{self, File},
    io::Write,
    str::FromStr,
};
use tokio::sync::mpsc;

const VERSION: Version = Version::new(0, 1, 0);
const CONFIG_FILE: &str = "config.toml";

pub async fn start() {
    log_info!("Starting Starlight node version {}", VERSION);
    let config = match fs::read_to_string(CONFIG_FILE) {
        Ok(config) => {
            log_info!("Loaded config from {}", CONFIG_FILE);
            match toml::from_str(&config) {
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
                f.write_all(toml::to_string(&config).unwrap().as_bytes())?;
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
    let rpc = Rpc::new(config.rpc_endpoint);
    tokio::spawn(async move {
        rpc.run().await.unwrap();
    });
    log_info!("RPC listening on http://{}", config.rpc_endpoint);
    let id = Identity { private, public };
    let network = Network::new(NetworkConfig {
        bind_ep: config.node_bind_endpoint,
        visible_ep: config.node_external_endpoint,
        id,
        initial_peers: Arc::new(config.initial_peers),
        max_less: config.max_less_peers,
        max_greater: config.max_greater_peers,
        get_weight: Box::new(|_| Amount::from_raw(1)),
        version: VERSION,
        allow_peers_with_private_ip_addresses: config.allow_peers_with_private_ip_addresses,
        allow_peers_with_node_external_ip_address: config.allow_peers_with_node_external_ip_address,
    })
    .await
    .unwrap();
    log_info!("SLP listening on udp://{}", config.node_bind_endpoint);
    log_info!(
        "SLP external endpoint is udp://{}",
        config.node_external_endpoint
    );
    if config.node_external_endpoint.addr == [127, 0, 0, 1] {
        log_warn!("SLP external endpoint is localhost; this node will not be able to communicate over the Internet");
    }
    network.run().await.unwrap();
}
