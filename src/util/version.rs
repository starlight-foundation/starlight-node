use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn to_bytes(&self) -> [u8; 6] {
        let mut bytes = [0u8; 6];
        bytes[0..2].copy_from_slice(&self.major.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.minor.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.patch.to_le_bytes());
        bytes
    }

    pub fn is_compatible(self, other: Version) -> bool {
        self.major == other.major
    }

    pub fn unknown() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
        }
    }

    pub fn is_unknown(self) -> bool {
        self.major == 0 && self.minor == 0 && self.patch == 0
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version string: {}", s));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}
