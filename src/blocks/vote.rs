use super::Pair;
use crate::keys::{Hash, Public, Signature};

pub struct Vote {
    pub from: Public,
    pub left: Pair,
    pub right: Pair,
    pub signature: Signature,
}

impl Vote {
    pub fn verify_and_hash(&self) -> Result<Hash, ()> {
        let mut bytes = [0u8; 112];
        bytes[0..32].copy_from_slice(self.from.as_bytes());
        bytes[32..40].copy_from_slice(&self.left.slot.to_bytes());
        bytes[40..72].copy_from_slice(self.left.block.as_bytes());
        bytes[72..80].copy_from_slice(&self.right.slot.to_bytes());
        bytes[80..112].copy_from_slice(self.right.block.as_bytes());
        let vote_hash = Hash::digest(&bytes);
        self.from.verify(&vote_hash, &self.signature)?;
        Ok(vote_hash)
    }
}
