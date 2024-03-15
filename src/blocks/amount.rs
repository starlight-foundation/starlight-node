use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use serde::{Deserialize, Serialize};

const UNIT: u64 = 10_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Amount(u64);

impl Amount {
    pub const fn zero() -> Self {
        Amount(0)
    }
    pub const fn from_raw(value: u64) -> Self {
        Amount(value)
    }
    pub const fn to_raw(self) -> u64 {
        self.0
    }
    pub fn to_unit(self) -> f32 {
        self.0 as f32 / UNIT as f32
    }
    pub fn from_unit(value: f32) -> Self {
        Amount((value * UNIT as f32) as u64)
    }
    pub const fn initial_supply() -> Self {
        Amount(i64::MAX as u64)
    }
    pub const fn to_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
    pub const fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(u64::from_le_bytes(bytes))
    }
}

impl Add for Amount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Amount(self.0 + other.0)
    }
}

impl Sub for Amount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Amount(self.0 - other.0)
    }
}

impl Mul for Amount {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Amount(self.0 * other.0)
    }
}

impl Div for Amount {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Amount(self.0 / other.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl MulAssign for Amount {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
    }
}

impl DivAssign for Amount {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
    }
}

impl fmt::Debug for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Amount({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_supply() {
        let supply = Amount::initial_supply();
        assert_eq!(supply.to_raw(), i64::MAX as u64);
    }

    #[test]
    fn test_from_and_to_raw() {
        let raw_value: u64 = 500_000_000;
        let amount = Amount::from_raw(raw_value);
        assert_eq!(amount.to_raw(), raw_value);
    }

    #[test]
    fn test_from_and_to_unit() {
        let unit_value: f32 = 0.5; // Represents 0.5 units
        let amount = Amount::from_unit(unit_value);
        assert_eq!(amount.to_unit(), unit_value);
    }

    #[test]
    fn test_addition() {
        let amount1 = Amount::from_raw(100);
        let amount2 = Amount::from_raw(200);
        let result = amount1 + amount2;
        assert_eq!(result.to_raw(), 300);
    }

    #[test]
    fn test_subtraction() {
        let amount1 = Amount::from_raw(300);
        let amount2 = Amount::from_raw(100);
        let result = amount1 - amount2;
        assert_eq!(result.to_raw(), 200);
    }

    #[test]
    fn test_multiplication() {
        let amount1 = Amount::from_raw(10);
        let amount2 = Amount::from_raw(20);
        let result = amount1 * amount2;
        assert_eq!(result.to_raw(), 200);
    }

    #[test]
    fn test_division() {
        let amount1 = Amount::from_raw(200);
        let amount2 = Amount::from_raw(10);
        let result = amount1 / amount2;
        assert_eq!(result.to_raw(), 20);
    }
}
