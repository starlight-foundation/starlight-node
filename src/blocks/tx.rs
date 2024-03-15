use crate::keys::{Difficulty, Hash, HashBuilder, Public, Signature, Work};

use super::{Amount, TxKind};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Tx {
    pub nonce: u64,
    pub from: Public,
    pub kind: TxKind,
    pub balance: Amount,
    pub amount: Amount,
    pub to: Public,
    pub work: Work,
    pub signature: Signature,
}

impl Tx {
    pub fn verify_and_hash(&self) -> Result<Hash, ()> {
        let (work_hash, tx_hash) = {
            let mut hb = HashBuilder::new();
            hb.update(&self.nonce.to_le_bytes());
            hb.update(self.from.as_bytes());
            let work_hash = hb.finalize();
            hb.update(&self.kind.to_bytes());
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
