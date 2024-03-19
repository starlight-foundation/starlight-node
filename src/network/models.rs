use serde::{Deserialize, Serialize};

use crate::{
    blocks::{Amount, Slot},
    error,
    keys::{Hash, HashBuilder, Private, Public, Signature},
    util::{self, Error},
};

use super::{center_map::CenterMapValue, config, logical::Logical, shred::Shred, version::Version};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Peer {
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

#[derive(Serialize, Deserialize, Clone)]
pub struct TelemetryMsg {
    pub from: Public,
    pub signature: Signature,
    pub slot: Slot,
    pub logical: Logical,
    pub version: Version,
}
impl TelemetryMsg {
    fn hash_pieces(slot: Slot, logical: Logical, version: Version) -> Hash {
        let mut buf = [0u8; 20];
        buf[0..8].copy_from_slice(&slot.to_bytes());
        buf[8..14].copy_from_slice(&logical.to_bytes());
        buf[14..20].copy_from_slice(&version.to_bytes());
        Hash::digest(&buf)
    }
    pub fn sign_new(private: Private, slot: Slot, logical: Logical, version: Version) -> Self {
        let hash = Self::hash_pieces(slot, logical, version);
        let signature = private.sign(&hash);
        Self {
            from: private.to_public(),
            signature,
            slot,
            logical,
            version,
        }
    }
    pub fn hash(&self) -> Hash {
        Self::hash_pieces(self.slot, self.logical, self.version)
    }
    pub fn verify(&self) -> Result<(), Error> {
        let hash = self.hash();
        self.from.verify(&hash, &self.signature)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShredMsg {
    pub from: Public,
    pub signature: Signature,
    pub slot: Slot,
    pub shred: Shred,
}
impl ShredMsg {
    pub fn hash(&self) -> Hash {
        let mut hb = HashBuilder::new();
        hb.update(&self.slot.to_bytes());
        self.shred.hash_into(&mut hb);
        hb.finish()
    }
    pub fn verify(&self) -> Result<(), Error> {
        let hash = self.hash();
        self.from.verify(&hash, &self.signature)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Msg {
    Tel(Box<TelemetryMsg>),
    Shred(Box<ShredMsg>),
}
impl Msg {
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(config::MTU);
        bytes.extend_from_slice(&config::MAGIC_NUMBER);
        util::serialize_into(&mut bytes, self);
        bytes
    }
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 8 {
            return Err(error!("message too small"));
        }
        if bytes[0..8] != config::MAGIC_NUMBER {
            return Err(error!("wrong magic number"));
        }
        if bytes.len() > config::MTU {
            return Err(error!("message too large"));
        }
        util::deserialize(&bytes[8..]).or_else(|_| {
            return Err(error!("invalid message"));
        })
    }
    pub fn verify(&self) -> Result<(), Error> {
        match self {
            Msg::Tel(t) => t.verify(),
            Msg::Shred(s) => s.verify(),
        }
    }
}
