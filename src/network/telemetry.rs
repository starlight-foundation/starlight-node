use crate::blocks::Difficulty;
use crate::blocks::Hash;
use crate::blocks::Slot;
use crate::blocks::Work;
use crate::keys::Signature;
use crate::keys::Public;
use std::net::SocketAddrV4;
use serde::{Serialize, Deserialize};

use super::Logical;
use super::Version;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct Telemetry {
    pub public: Public,
    pub logical: Logical,
    pub version: Version,
    pub slot: Slot,
    work: Work,
    signature: Signature
}

impl Telemetry {
    pub fn verify(&self) -> Result<(), ()> {
        const MSG_SIZE: usize = std::mem::size_of::<Public>()
            + std::mem::size_of::<Logical>()
            + std::mem::size_of::<Version>()
            + std::mem::size_of::<Slot>();
        let msg: &[u8; MSG_SIZE] = unsafe {
            std::mem::transmute(self)
        };
        let hash = Hash::of_slice(&msg);
        self.work.verify(&hash, Difficulty::BASE)?;
        self.public.verify(&hash, &self.signature)?;
        Ok(())
    }
}

