use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{keys::Seed, network::Endpoint};

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(serialize_with = "crate::util::serialize_to_display", deserialize_with = "crate::util::deserialize_from_string")]
    pub rpc_endpoint: Endpoint,
    #[serde(serialize_with = "crate::util::serialize_to_display", deserialize_with = "crate::util::deserialize_from_string")]
    pub node_bind_endpoint: Endpoint,
    #[serde(serialize_with = "crate::util::serialize_to_display", deserialize_with = "crate::util::deserialize_from_string")]
    pub node_external_endpoint: Endpoint,
    #[serde(serialize_with = "crate::util::serialize_to_display", deserialize_with = "crate::util::deserialize_from_string")]
    pub node_seed: Seed,
    #[serde(serialize_with = "crate::util::serialize_list_to_display", deserialize_with = "crate::util::deserialize_list_from_string")]
    pub initial_peers: Vec<Endpoint>,
    pub max_less_peers: usize,
    pub max_greater_peers: usize,
    pub allow_peers_with_private_ip_addresses: bool,
}

impl Config {
    pub fn new() -> Self {
        Self {
            rpc_endpoint: Endpoint::from_str("127.0.0.1:41594").unwrap(),
            node_bind_endpoint: Endpoint::from_str("0.0.0.0:44039").unwrap(),
            node_external_endpoint: Endpoint::from_str("127.0.0.1:44039").unwrap(),
            node_seed: Seed::random(),
            initial_peers: Vec::new(),
            max_less_peers: 250,
            max_greater_peers: 250,
            allow_peers_with_private_ip_addresses: false,
        }
    }
}