use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[repr(u64)]
pub enum TransactionKind {
    Transfer = 0,
    ChangeRepresentative = 1,
    Open = 2,
}
impl TransactionKind {
    pub fn to_bytes(self) -> [u8; 8] {
        (self as u64).to_le_bytes()
    }
}
