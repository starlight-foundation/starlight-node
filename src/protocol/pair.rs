use super::Slot;
use crate::keys::Hash;

pub struct Pair {
    pub slot: Slot,
    pub block: Hash,
}
