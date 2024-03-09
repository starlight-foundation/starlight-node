use crate::{blocks::Hash, keys::{encoding::blake2b, public::Public, signature::Signature}, pow::{Difficulty, Work}};

#[derive(Clone, Copy)]
pub struct Transaction {
    pub nonce: u64,
    pub balance: u64,
    pub from: Public,
    pub to: Public,
    pub work: Work,
    pub signature: Signature
}

impl Transaction {
    pub fn validate(&self) -> Result<(), ()> {
        const MSG_SIZE: usize = std::mem::size_of::<Transaction>()
            - std::mem::size_of::<Work>()
            - std::mem::size_of::<Signature>();
        let mut msg = [0u8; MSG_SIZE];
        msg[0..8].copy_from_slice(&self.nonce.to_le_bytes());
        msg[8..16].copy_from_slice(&self.balance.to_le_bytes());
        msg[16..48].copy_from_slice(&self.from.0);
        msg[48..80].copy_from_slice(&self.to.0);
        let hash = Hash::hash(&msg);
        self.from.verify(&hash.0, &self.signature)?;
        self.work.verify(&hash, &Difficulty::BASE)
    }
}

