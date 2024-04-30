use bincode::{Encode, Decode};

use super::Slot;

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq)]
pub struct Epoch(pub(super) u64);

impl Epoch {
    pub const LEN: usize = 86400;
    pub const fn zero() -> Self {
        Self(0)
    }
    pub const fn max() -> Self {
        Self(u64::MAX)
    }
    pub const fn to_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
    pub fn get(self, i: usize) -> Option<Slot> {
        if i < Self::LEN {
            Some(Slot(self.0 * Self::LEN as u64 + i as u64))
        } else {
            None
        }
    }
    pub fn index_of(self, slot: Slot) -> Option<usize> {
        if slot.0 >= self.0 * Self::LEN as u64 && slot.0 < (self.0 + 1) * Self::LEN as u64 {
            Some((slot.0 - self.0 * Self::LEN as u64) as usize)
        } else {
            None
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = Slot> {
        (self.0 * Self::LEN as u64..(self.0 + 1) * Self::LEN as u64)
            .into_iter()
            .map(|i| Slot(i))
    }
}
