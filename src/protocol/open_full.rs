use bincode::{Decode, Encode};

use crate::keys::Hash;

use super::Open;

#[derive(Encode, Decode)]
pub struct OpenFull {
    pub open: Open,
    pub hash: Hash
}

impl OpenFull {
    pub fn new(open: Open, hash: Hash) -> Self {
        Self { open, hash }
    }
}