use crate::{EngineError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: u128,
    den: u128,
}

impl Rational {
    pub const ZERO: Self = Self { num: 0, den: 1 };
    pub const ONE: Self = Self { num: 1, den: 1 };

    pub fn new(num: u128, den: u128) -> Result<Self> {
        if den == 0 {
            return Err(EngineError::new("rational denominator cannot be zero"));
        }
        if num == 0 {
            return Ok(Self::ZERO);
        }
        let g = gcd(num, den);
        Ok(Self {
            num: num / g,
            den: den / g,
        })
    }

    pub fn from_u64(value: u64) -> Self {
        Self {
            num: value as u128,
            den: 1,
        }
    }

    pub fn numerator(self) -> u128 {
        self.num
    }

    pub fn denominator(self) -> u128 {
        self.den
    }

    pub fn is_zero(self) -> bool {
        self.num == 0
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        let left = self
            .num
            .checked_mul(rhs.den)
            .ok_or_else(|| EngineError::new("rational addition overflow"))?;
        let right = rhs
            .num
            .checked_mul(self.den)
            .ok_or_else(|| EngineError::new("rational addition overflow"))?;
        let num = left
            .checked_add(right)
            .ok_or_else(|| EngineError::new("rational addition overflow"))?;
        let den = self
            .den
            .checked_mul(rhs.den)
            .ok_or_else(|| EngineError::new("rational addition overflow"))?;
        Self::new(num, den)
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self> {
        if self < rhs {
            return Err(EngineError::new("rational subtraction would be negative"));
        }
        let left = self
            .num
            .checked_mul(rhs.den)
            .ok_or_else(|| EngineError::new("rational subtraction overflow"))?;
        let right = rhs
            .num
            .checked_mul(self.den)
            .ok_or_else(|| EngineError::new("rational subtraction overflow"))?;
        let num = left
            .checked_sub(right)
            .ok_or_else(|| EngineError::new("rational subtraction underflow"))?;
        let den = self
            .den
            .checked_mul(rhs.den)
            .ok_or_else(|| EngineError::new("rational subtraction overflow"))?;
        Self::new(num, den)
    }

    pub fn checked_mul(self, rhs: Self) -> Result<Self> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }
        let g1 = gcd(self.num, rhs.den);
        let g2 = gcd(rhs.num, self.den);
        let a = self.num / g1;
        let d = rhs.den / g1;
        let c = rhs.num / g2;
        let b = self.den / g2;
        let num = a
            .checked_mul(c)
            .ok_or_else(|| EngineError::new("rational multiplication overflow"))?;
        let den = b
            .checked_mul(d)
            .ok_or_else(|| EngineError::new("rational multiplication overflow"))?;
        Self::new(num, den)
    }

    pub fn checked_div(self, rhs: Self) -> Result<Self> {
        if rhs.is_zero() {
            return Err(EngineError::new("division by zero rational"));
        }
        self.checked_mul(Self::new(rhs.den, rhs.num)?)
    }

    pub fn min(self, rhs: Self) -> Self {
        if self <= rhs { self } else { rhs }
    }

    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

impl FromStr for Rational {
    type Err = EngineError;

    fn from_str(input: &str) -> Result<Self> {
        let s = input.trim();
        if s.is_empty() {
            return Err(EngineError::new("empty probability"));
        }
        if s.starts_with('-') {
            return Err(EngineError::new("negative probabilities are not supported"));
        }
        if let Some((n, d)) = s.split_once('/') {
            let num = n
                .trim()
                .parse::<u128>()
                .map_err(|_| EngineError::new(format!("invalid rational numerator: {s}")))?;
            let den = d
                .trim()
                .parse::<u128>()
                .map_err(|_| EngineError::new(format!("invalid rational denominator: {s}")))?;
            return Self::new(num, den);
        }
        if let Some((whole, frac)) = s.split_once('.') {
            if whole.is_empty() && frac.is_empty() {
                return Err(EngineError::new(format!(
                    "invalid decimal probability: {s}"
                )));
            }
            let whole_num = if whole.is_empty() {
                0
            } else {
                whole
                    .parse::<u128>()
                    .map_err(|_| EngineError::new(format!("invalid decimal probability: {s}")))?
            };
            if frac.is_empty() {
                return Self::new(whole_num, 1);
            }
            if !frac.bytes().all(|b| b.is_ascii_digit()) {
                return Err(EngineError::new(format!(
                    "invalid decimal probability: {s}"
                )));
            }
            let scale = checked_pow10(frac.len())?;
            let frac_num = frac
                .parse::<u128>()
                .map_err(|_| EngineError::new(format!("invalid decimal probability: {s}")))?;
            let num = whole_num
                .checked_mul(scale)
                .and_then(|v| v.checked_add(frac_num))
                .ok_or_else(|| EngineError::new("decimal probability overflow"))?;
            return Self::new(num, scale);
        }
        let num = s
            .parse::<u128>()
            .map_err(|_| EngineError::new(format!("invalid probability: {s}")))?;
        Self::new(num, 1)
    }
}

impl Display for Rational {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        match (
            self.num.checked_mul(other.den),
            other.num.checked_mul(self.den),
        ) {
            (Some(left), Some(right)) => left.cmp(&right),
            _ => self.to_f64().total_cmp(&other.to_f64()),
        }
    }
}

impl Serialize for Rational {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Rational {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

fn checked_pow10(exp: usize) -> Result<u128> {
    let mut value = 1_u128;
    for _ in 0..exp {
        value = value
            .checked_mul(10)
            .ok_or_else(|| EngineError::new("decimal scale overflow"))?;
    }
    Ok(value)
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimal_exactly() {
        let p = Rational::from_str("0.008").expect("parse");
        assert_eq!(p, Rational::new(1, 125).expect("ratio"));
    }

    #[test]
    fn arithmetic_is_exact() {
        let a = Rational::from_str("0.008").expect("parse");
        let b = Rational::from_str("0.05").expect("parse");
        let result = a.checked_add(b).expect("add");
        assert_eq!(result, Rational::from_str("0.058").expect("parse"));
    }
}
