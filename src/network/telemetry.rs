use crate::blocks::Slot;
use crate::keys::Difficulty;
use crate::keys::Hash;
use crate::keys::Public;
use crate::keys::Signature;
use crate::keys::Work;
use serde::{Deserialize, Serialize};

use super::Logical;
use super::Version;

#[derive(Serialize, Deserialize, Clone, Copy)]
#[repr(C)]
pub(crate) struct Telemetry {
    pub slot: Slot,
    pub logical: Logical,
    pub version: Version,
}

impl Telemetry {
    pub fn hash(&self) -> Hash {
        let mut bytes = [0u8; 20];
        bytes[0..8].copy_from_slice(&self.slot.to_bytes());
        bytes[8..14].copy_from_slice(&self.logical.to_bytes());
        bytes[14..20].copy_from_slice(&self.version.to_bytes());
        Hash::digest(&bytes)
    }
}
