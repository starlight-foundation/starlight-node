use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct Version {
    major: u16,
    minor: u16,
    patch: u16
}
impl Version {
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch
        }
    }
    pub fn is_compatible(self, other: Version) -> bool {
        self.major == other.major
    }
    pub fn unknown() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0
        }
    }
    pub fn is_unknown(self) -> bool {
        self.major == 0 && self.minor == 0 && self.patch == 0
    }
}

