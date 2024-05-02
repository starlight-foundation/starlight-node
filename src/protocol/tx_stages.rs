use bincode::{Decode, Encode};
use crate::keys::Hash;

use super::Tx;

#[derive(Encode, Decode)]
#[repr(C)]
pub struct TxFull {
    pub tx: Tx,
    pub hash: Hash,
    pub from_index: u64,
    pub to_index: u64
}

#[derive(Encode, Decode)]
#[repr(C)]
pub struct TxHalf {
    pub tx: Tx,
    pub hash: Hash,
    from_index: u64,
    to_index: u64
}
impl TxHalf {
    pub fn provide(mut self: Box<Self>, from_index: u64, to_index: u64) -> Box<TxFull> {
        self.from_index = from_index;
        self.to_index = to_index;
        unsafe {
            std::mem::transmute(self)
        }
    }
}

#[derive(Encode, Decode)]
#[repr(C)]
pub struct TxEmpty {
    pub tx: Tx,
    hash: Hash,
    from_index: u64,
    to_index: u64
}
impl TxEmpty {
    pub fn boxed(tx: Tx) -> Box<Self> {
        Box::new(Self {
            tx,
            hash: Hash::zero(),
            from_index: 0,
            to_index: 0
        })
    }
    pub fn provide(mut self: Box<Self>, hash: Hash) -> Box<TxHalf> {
        self.hash = hash;
        unsafe {
            std::mem::transmute(self)
        }
    }
}