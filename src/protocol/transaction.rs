use crate::{
    error,
    keys::{Difficulty, Hash, Public, Signature, Work},
    util::Error,
};

use super::{Amount, TransactionKind};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
#[repr(C)]
pub struct Transaction {
    pub nonce: u64,
    pub from: Public,
    pub kind: TransactionKind,
    pub balance: Amount,
    pub amount: Amount,
    pub to: Public,
    pub work: Work,
    pub signature: Signature,
}

impl Transaction {
    pub fn verify_and_hash(&self) -> Result<Hash, Error> {
        match self.kind {
            TransactionKind::Transfer => {
                if self.amount == Amount::zero() {
                    return Err(error!("normal tx must transfer > 0"));
                }
            }
            TransactionKind::Open => {
                if self.amount != Amount::zero() {
                    return Err(error!("open tx can't transfer"));
                }
                if self.balance != Amount::zero() {
                    return Err(error!("open accounts start with a zero balance"));
                }
                if self.nonce != 0 {
                    return Err(error!("open tx must have a nonce of 0"));
                }
            }
            TransactionKind::ChangeRepresentative => {
                if self.amount != Amount::zero() {
                    return Err(error!("change representative tx must not transfer"));
                }
            }
        };
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
