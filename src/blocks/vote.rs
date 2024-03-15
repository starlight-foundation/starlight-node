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
        const VOTE_HASH_INPUT_SIZE: usize =
            std::mem::size_of::<Vote>() - std::mem::size_of::<Signature>();
        let msg: &[u8; VOTE_HASH_INPUT_SIZE] = unsafe { std::mem::transmute(self) };
        let vote_hash = Hash::of_slice(msg);
        self.from.verify(&vote_hash, &self.signature)?;
        Ok(vote_hash)
    }
}
