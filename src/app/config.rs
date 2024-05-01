use std::str::FromStr;

use nanoserde::{DeJson, SerJson};

use crate::{keys::Seed, network::Endpoint};

#[derive(SerJson, DeJson)]
pub struct Config {
    pub node_bind_endpoint: Endpoint,
    pub node_external_endpoint: Endpoint,
    pub rpc_endpoint: Endpoint,
    pub db_endpoint: Endpoint,
    pub node_seed: Seed,
    pub initial_peers: Vec<Endpoint>,
    pub max_less_peers: usize,
    pub max_greater_peers: usize,
    pub allow_peers_with_private_ip_addresses: bool,
    pub allow_peers_with_node_external_ip_address: bool,
    pub tx_pool_size: usize,
    pub open_pool_size: usize,
    pub vote_pool_size: usize,
    pub data_dir: String
}

impl Config {
    pub fn new() -> Self {
        Self {
            node_bind_endpoint: Endpoint::from_str("0.0.0.0:41594").unwrap(),
            node_external_endpoint: Endpoint::from_str("127.0.0.1:41594").unwrap(),
            rpc_endpoint: Endpoint::from_str("127.0.0.1:41595").unwrap(),
            db_endpoint: Endpoint::from_str("127.0.0.1:41596").unwrap(),
            node_seed: Seed::random(),
            initial_peers: Vec::new(),
            max_less_peers: 250,
            max_greater_peers: 250,
            allow_peers_with_private_ip_addresses: false,
            allow_peers_with_node_external_ip_address: false,
            tx_pool_size: 50_000,
            open_pool_size: 25,
            vote_pool_size: 1_000,
            data_dir: "./data".to_string()
        }
    }
}
