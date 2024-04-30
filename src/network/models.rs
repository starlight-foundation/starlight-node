use bincode::{Decode, Encode};

use crate::{
    error,
    keys::{Hash, HashBuilder, Private, Public, Signature},
    protocol::{Amount, Open, Slot, Transaction},
    util::{self, Error, Version},
};

use super::{center_map::CenterMapValue, endpoint::Endpoint, shred::Shred};

#[derive(Encode, Decode, Clone, Copy)]
#[repr(C)]
pub struct Peer {
    pub weight: Amount,
    pub last_contact: Slot,
    pub endpoint: Endpoint,
    pub version: Version,
}
impl CenterMapValue<Amount> for Peer {
    fn priority(&self) -> Amount {
        self.weight
    }
}

#[derive(Encode, Decode, Clone, Copy)]
#[repr(C)]
pub struct TelemetryNote {
    pub from: Public,
    pub signature: Signature,
    pub slot: Slot,
    pub ep: Endpoint,
    pub version: Version,
}
impl TelemetryNote {
    fn hash_pieces(slot: Slot, ep: Endpoint, version: Version) -> Hash {
        let mut buf = [0u8; 20];
        buf[0..8].copy_from_slice(&slot.to_bytes());
        buf[8..14].copy_from_slice(&ep.to_bytes());
        buf[14..20].copy_from_slice(&version.to_bytes());
        Hash::digest(&buf)
    }
    pub fn new(private: Private, slot: Slot, ep: Endpoint, version: Version) -> Self {
        let mut tel_note = Self {
            from: private.to_public(),
            signature: Signature::zero(),
            slot,
            ep,
            version,
        };
        let bytes = util::view_as_bytes(&tel_note);
        let hash = Hash::digest(&bytes[96..]);
        let signature = private.sign(&hash);
        tel_note.signature = signature;
        tel_note
    }
    pub fn hash(&self) -> Hash {
        Self::hash_pieces(self.slot, self.ep, self.version)
    }
    pub fn verify(&self) -> Result<(), Error> {
        let hash = self.hash();
        self.from.verify(&hash, &self.signature)
    }
}

#[derive(Encode, Decode, Clone)]
#[repr(C)]
pub struct ShredNote {
    pub from: Public,
    pub signature: Signature,
    pub slot: Slot,
    pub shred: Shred,
}
impl ShredNote {
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

const MAGIC_NUMBER: [u8; 7] = [0x3f, 0xd1, 0x0f, 0xe2, 0x5e, 0x76, 0xfa];

#[derive(Encode, Decode, Clone)]
pub enum Note {
    TelemetryNote(Box<TelemetryNote>),
    ShredNote(Box<ShredNote>),
    Transaction(Box<Transaction>),
    Open(Box<Open>)
}
impl Note {
    pub fn serialize(&self, mtu: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(mtu);
        bytes.extend_from_slice(&MAGIC_NUMBER);
        util::encode_into_writer(&mut bytes, self).unwrap();
        bytes
    }
    pub fn deserialize(bytes: &[u8], mtu: usize) -> Result<Self, Error> {
        if bytes.len() < 8 {
            return Err(error!("message too small"));
        }
        if bytes[0..8] != MAGIC_NUMBER {
            return Err(error!("wrong magic number"));
        }
        if bytes.len() > mtu {
            return Err(error!("message too large"));
        }
        util::decode_from_slice(&bytes[8..]).or_else(|_| {
            return Err(error!("invalid message"));
        })
    }
}
