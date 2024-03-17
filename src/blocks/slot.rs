use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};

const GENESIS_TIME_MS: u64 = 1710290840 * 1000;
const SLOT_TIME_MS: u64 = 500;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Slot(u64);
impl Slot {
    pub fn genesis() -> Slot {
        Slot(0)
    }
    pub fn now() -> Slot {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Slot(now_ms.saturating_sub(GENESIS_TIME_MS) / SLOT_TIME_MS)
    }
    pub fn previous(self) -> Self {
        Self(self.0.saturating_sub(1))
    }
    pub fn saturating_sub(self, other: Slot) -> u64 {
        self.0.saturating_sub(other.0)
    }
    pub fn to_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
    pub fn from_bytes(self, bytes: [u8; 8]) -> Self {
        Self(u64::from_le_bytes(bytes))
    }
    pub fn max() -> Self {
        Self(u64::MAX)
    }
}

impl std::ops::Sub for Slot {
    type Output = u64;

    fn sub(self, other: Slot) -> Self::Output {
        self.0 - other.0
    }
}

impl PartialEq for Slot {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Slot {}

impl PartialOrd for Slot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Slot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
