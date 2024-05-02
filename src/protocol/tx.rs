use crate::{
    error,
    keys::{Difficulty, Hash, Public, Signature, Work},
    util::{self, Archived, Error},
};

use super::Amount;
use bincode::{Encode, Decode};

/// A transaction, either a normal or change representative transaction.
/// When `amount` != `Amount::zero()`:
/// - Funds equal to `amount` are transferred from `from` to `to`.
/// Else:
/// - The representative of `from` is changed to `to`.
#[derive(Encode, Decode, Clone, Copy, Debug)]
#[repr(C)]
pub struct Tx {
    pub nonce: u64,
    pub from: Public,
    pub amount: Amount,
    pub to: Public,
    pub work: Work,
    pub signature: Signature,
}

impl Tx {
    pub fn is_change_representative(&self) -> bool {
        self.amount == Amount::zero()
    }
    pub fn verify_and_hash(&self) -> Result<Hash, Error> {
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