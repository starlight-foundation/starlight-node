use std::time::{Duration, Instant, SystemTime};

use bincode::{Encode, Decode};

use super::Epoch;

const GENESIS_TIME_MS: u64 = 1710290840 * 1000;
const SLOT_TIME_MS: u64 = 500;

#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
pub struct Slot(pub(super) u64);
impl Slot {
    pub fn zero() -> Slot {
        Slot(0)
    }
    pub fn to_system_time(self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_millis(GENESIS_TIME_MS + self.0 * SLOT_TIME_MS)
    }
    pub fn from_system_time(time: SystemTime) -> Slot {
        let now_ms = time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Slot((now_ms - GENESIS_TIME_MS) / SLOT_TIME_MS)
    }
    pub fn now() -> Slot {
        Self::from_system_time(SystemTime::now())
    }
    pub fn prev(self) -> Self {
        Self(self.0.saturating_sub(1))
    }
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
    pub fn saturating_sub(self, other: Slot) -> u64 {
        self.0.saturating_sub(other.0)
    }
    pub const fn to_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
    pub const fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(u64::from_le_bytes(bytes))
    }
    pub fn max() -> Self {
        Self(u64::MAX)
    }
    pub fn epoch(self) -> Epoch {
        Epoch(self.0 / Epoch::LEN as u64)
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
