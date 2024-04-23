use crate::{
    error,
    keys::{Difficulty, Hash, Public, Signature, Work},
    util::{self, Archived, Error},
};

use super::{Amount, Verifiable};
use serde::{Deserialize, Serialize};

/// A transaction, either a normal or change representative transaction.
/// When `amount` != `Amount::zero()`:
/// - Funds equal to `amount` are transferred from `from` to `to`.
/// Else:
/// - The representative of `from` is changed to `to`.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[repr(C)]
pub struct Transaction {
    pub nonce: u64,
    pub from: Public,
    pub amount: Amount,
    pub to: Public,
    pub work: Work,
    pub signature: Signature,
}

impl Transaction {
    pub fn is_change_representative(&self) -> bool {
        self.amount == Amount::zero()
    }
}

impl Verifiable for Transaction {
    fn verify_and_hash(&self) -> Result<Hash, Error> {
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