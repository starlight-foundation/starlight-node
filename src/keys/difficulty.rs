// Derived from the pow module of github.com/feeless/feeless@978eba7.
use crate::error;
use crate::util::Error;
use crate::util::{deserialize_from_str, expect_len, to_hex};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;

#[derive(Eq, PartialEq, Clone, Copy)]
pub struct Difficulty(u64);

impl Difficulty {
    /// fffffff800000000
    pub const BASE: Self = Self(18446744039349813248);
    const LEN: usize = 8;
    const HEX_LEN: usize = Self::LEN * 2;

    pub const fn new(v: u64) -> Self {
        Self(v)
    }

    pub fn from_le_fixed(s: &[u8; Self::LEN]) -> Self {
        Difficulty(u64::from_le_bytes(*s))
    }

    pub fn from_be_slice(s: &[u8]) -> Result<Self, Error> {
        let b = <[u8; Self::LEN]>::try_from(s).or(Err(error!("wrong difficulty len")))?;
        Ok(Difficulty(u64::from_be_bytes(b)))
    }

    pub fn from_le_slice(s: &[u8]) -> Result<Self, Error> {
        let b = <[u8; Self::LEN]>::try_from(s)?;
        Ok(Difficulty(u64::from_le_bytes(b)))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Debug for Difficulty {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_hex(&self.0.to_be_bytes()))
    }
}

impl FromStr for Difficulty {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        expect_len(s.len(), Self::HEX_LEN, "Difficulty")?;
        let mut slice = [0u8; Self::LEN];
        hex::decode_to_slice(s, &mut slice)
            .map_err(|source| error!("can't decode hex: {}", source))?;
        Ok(Difficulty::from_be_slice(&slice).unwrap())
    }
}

impl PartialOrd for Difficulty {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Serialize for Difficulty {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(to_hex(&self.0.to_be_bytes()).as_str())
    }
}

impl<'de> Deserialize<'de> for Difficulty {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_from_str(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(
            Difficulty::from_str("ffffffc000000000").unwrap().as_u64(),
            18446743798831644672u64
        );
        assert_eq!(
            Difficulty::BASE,
            Difficulty::from_str("fffffff800000000").unwrap()
        )
    }
}
