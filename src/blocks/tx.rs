use crate::{
    error,
    keys::{Difficulty, Hash, Public, Signature, Work},
    util::Error,
};

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
    pub fn verify_and_hash(&self) -> Result<Hash, Error> {
        if self.kind == TxKind::Normal && self.amount == Amount::zero() {
            return Err(error!("normal tx must transfer > 0"));
        }
        let mut bytes = [0u8; 96];
        bytes[0..8].copy_from_slice(&self.nonce.to_le_bytes());
        bytes[8..40].copy_from_slice(self.from.as_bytes());
        bytes[40..48].copy_from_slice(&self.kind.to_bytes());
        bytes[48..56].copy_from_slice(&self.balance.to_bytes());
        bytes[56..64].copy_from_slice(&self.amount.to_bytes());
        bytes[64..96].copy_from_slice(self.to.as_bytes());
        let work_hash = Hash::digest(&bytes[0..40]);
        let tx_hash = Hash::digest(&bytes);
        self.work.verify(&work_hash, Difficulty::BASE)?;
        self.from.verify(&tx_hash, &self.signature)?;
        Ok(tx_hash)
    }
}
