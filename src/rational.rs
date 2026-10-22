#[derive(Clone, Copy, Debug, Eq)]
pub struct Rational {
    num: u128,
    den: u128,
}

impl PartialEq for Rational {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let left = widen(self.num, other.den);
        let right = widen(other.num, self.den);
        left.cmp(&right)
    }
}

impl Rational {
    pub fn new(num: u128, den: u128) -> Self {
        assert!(den > 0, "zero denominator");
        let g = gcd(num, den);
        Self {
            num: num / g,
            den: den / g,
        }
    }

    pub fn zero() -> Self {
        Self { num: 0, den: 1 }
    }

    pub fn one() -> Self {
        Self { num: 1, den: 1 }
    }

    pub fn numer(&self) -> u128 {
        self.num
    }

    pub fn denom(&self) -> u128 {
        self.den
    }

    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        let num = self
            .num
            .checked_mul(other.den)?
            .checked_add(other.num.checked_mul(self.den)?)?;
        let den = self.den.checked_mul(other.den)?;
        Some(Self::new(num, den))
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        let num = self
            .num
            .checked_mul(other.den)?
            .checked_sub(other.num.checked_mul(self.den)?)?;
        let den = self.den.checked_mul(other.den)?;
        Some(Self::new(num, den))
    }

    pub fn checked_mul(self, other: Self) -> Option<Self> {
        let (ln, rd) = cancel(self.num, other.den);
        let (rn, ld) = cancel(other.num, self.den);
        Some(Self::new(ln.checked_mul(rn)?, ld.checked_mul(rd)?))
    }

    pub fn checked_div(self, other: Self) -> Option<Self> {
        if other.num == 0 {
            return None;
        }
        self.checked_mul(Self::new(other.den, other.num))
    }

    pub fn min(self, other: Self) -> Self {
        if self <= other { self } else { other }
    }
}

impl std::fmt::Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

pub(crate) fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn cancel(num: u128, den: u128) -> (u128, u128) {
    let g = gcd(num, den);
    (num / g, den / g)
}

fn widen(a: u128, b: u128) -> u128 {
    a.checked_mul(b)
        .unwrap_or_else(|| panic!("rational comparison overflow"))
}
