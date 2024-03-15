use super::{CenterMapValue, Logical};
use crate::blocks::Amount;
use crate::blocks::Slot;
use crate::network::version::Version;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct Peer {
    pub weight: Amount,
    pub last_contact: Slot,
    pub logical: Logical,
    pub version: Version,
}
impl CenterMapValue<Amount> for Peer {
    fn priority(&self) -> Amount {
        self.weight
    }
}
