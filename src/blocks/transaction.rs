use crate::{
    keys::{Public, Signature, Difficulty, Hash, HashBuilder, Work},
};

use super::Amount;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Transaction {
    pub nonce: u64,
    pub from: Public,
    pub balance: Amount,
    pub amount: Amount,
    pub to: Public,
    pub work: Work,
    pub signature: Signature,
}

impl Transaction {
    pub fn verify_and_hash(&self) -> Result<Hash, ()> {
        let (work_hash, tx_hash) = {
            let mut hb = HashBuilder::new();
            hb.update(&self.nonce.to_le_bytes());
            hb.update(self.from.as_bytes());
            let work_hash = hb.finalize();
            hb.update(&self.balance.to_bytes());
            hb.update(&self.amount.to_bytes());
            hb.update(self.to.as_bytes());
            let tx_hash = hb.finalize();
            (work_hash, tx_hash)
        };
        self.work.verify(&work_hash, Difficulty::BASE)?;
        self.from.verify(&tx_hash, &self.signature)?;
        Ok(tx_hash)
    }
}
