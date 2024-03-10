use crate::{blocks::Hash, keys::{public::Public, signature::Signature}, pow::{Difficulty, Work}};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Transaction {
    pub nonce: u64,
    pub from: Public,
    pub balance: u64,
    pub amount: u64,
    pub to: Public,
    pub work: Work,
    pub signature: Signature
}

impl Transaction {
    pub fn verify_and_hash(&self) -> Result<Hash, ()> {
        const TX_HASH_INPUT_SIZE: usize = std::mem::size_of::<Transaction>()
            - std::mem::size_of::<Signature>();
        let tx_msg: &[u8; TX_HASH_INPUT_SIZE] = unsafe { std::mem::transmute(self) };
        let tx_hash = Hash::of_slice(tx_msg);
        self.from.verify(&tx_hash.0, &self.signature)?;
        const WORK_HASH_INPUT_SIZE: usize = std::mem::size_of::<u64>()
            + std::mem::size_of::<Public>();
        let work_msg: &[u8; WORK_HASH_INPUT_SIZE] = unsafe { std::mem::transmute(self) };
        let work_hash = Hash::of_slice(work_msg);
        self.work.verify(&work_hash, &Difficulty::BASE)?;
        Ok(tx_hash)
    }
}

