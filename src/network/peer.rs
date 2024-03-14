use serde::{Serialize, Deserialize};
use crate::blocks::Slot;
use crate::network::version::Version;
use crate::blocks::Amount;
use std::net::SocketAddrV4;
use super::CenterMapValue;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct Peer {
    pub weight: Amount,
    pub last_contact: Slot,
    pub address: SocketAddrV4,
    pub version: Version
}
impl CenterMapValue<Amount> for Peer {
    fn priority(&self) -> Amount {
        self.weight
    }
}