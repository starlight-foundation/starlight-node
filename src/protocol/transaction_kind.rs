use serde::{Deserialize, Serialize};

use crate::util::ArchivableTo;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[repr(u64)]
pub enum TransactionKind {
    Transfer = 0,
    ChangeRepresentative = 1,
    Open = 2,
    Unknown = 3,
}
impl ArchivableTo<u64> for TransactionKind {
    fn archive(self) -> u64 {
        self as u64
    }
    fn unarchive(v: u64) -> Self {
        match v {
            0 => Self::Transfer,
            1 => Self::ChangeRepresentative,
            2 => Self::Open,
            _ => Self::Unknown,
        }
    }
}
