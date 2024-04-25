use serde::{Deserialize, Serialize};

use crate::{keys::{Difficulty, Hash, Public, Signature, Work}, util::{self, Error}};

use super::Verifiable;

#[repr(C)]
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Open {
    pub account: Public,
    pub representative: Public,
    pub work: Work,
    pub signature: Signature
}

impl Verifiable for Open {
    fn verify_and_hash(&self) -> Result<Hash, Error> {
        let bytes = util::view_as_bytes(self);
        // include `account` and `representative`
        let work_hash = Hash::digest(&bytes[0..64]);
        // include everything up to `signature`
        let tx_hash = Hash::digest(&bytes[0..70]);
        self.work.verify(&work_hash, Difficulty::BASE)?;
        self.account.verify(&tx_hash, &self.signature)?;
        Ok(tx_hash)
    }
}

