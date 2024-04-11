use crate::{
    error,
    keys::{Difficulty, Hash, Public, Signature, Work},
    util::{self, Archived, Error},
};

use super::{Amount, TransactionKind};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
#[repr(C)]
pub struct Transaction {
    pub nonce: u64,
    pub from: Public,
    pub kind: Archived<TransactionKind, u64>,
    pub amount: Amount,
    pub to: Public,
    pub work: Work,
    pub signature: Signature,
}

impl Transaction {
    pub fn verify_and_hash(&self) -> Result<Hash, Error> {
        match self.kind.get() {
            TransactionKind::Transfer => {
                if self.amount == Amount::zero() {
                    return Err(error!("normal tx must transfer > 0"));
                }
            }
            TransactionKind::Open => {
                if self.amount != Amount::zero() {
                    return Err(error!("open tx can't transfer"));
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
            TransactionKind::Unknown => {
                return Err(error!("unknown tx kind"));
            }
        };
        let bytes = util::view_as_bytes(self);
        // include `nonce` and `from`
        let work_hash = Hash::digest(&bytes[0..40]);
        // include everything up to `signature`
        let tx_hash = Hash::digest(&bytes[0..96]);
        self.work.verify(&work_hash, Difficulty::BASE)?;
        self.from.verify(&tx_hash, &self.signature)?;
        Ok(tx_hash)
    }
}
