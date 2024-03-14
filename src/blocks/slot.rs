use std::time::SystemTime;

use serde::{Deserialize, Serialize};

const GENESIS_UNIX_TIMESTAMP: u64 = 1710290840;
const SLOT_TIME_MS: u64 = 500;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Slot(u64);
impl Slot {
    pub fn genesis() -> Slot {
        Slot(0)
    }
    pub fn now() -> Slot {
        let genesis_ms = GENESIS_UNIX_TIMESTAMP * 1000;
        let now_ms = SystemTime::now().duration_since(
            SystemTime::UNIX_EPOCH
        ).unwrap().as_millis() as u64;
        Slot((now_ms - genesis_ms) / SLOT_TIME_MS)
    }
    pub fn saturating_sub(self, other: Slot) -> u64 {
        self.0.saturating_sub(other.0)
    }
    pub fn elapsed(self) -> Option<u64> {
        let now = Slot::now();
        if self > now {
            None
        } else {
            Some(now - self)
        }
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