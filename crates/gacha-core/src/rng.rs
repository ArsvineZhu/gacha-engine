#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    pub fn next_53_bits(&mut self) -> u64 {
        self.next_u64() >> 11
    }

    /// 在 [0, upper) 生成均匀整数，避免取模偏差。
    pub fn below_u128(&mut self, upper: u128) -> crate::Result<u128> {
        if upper == 0 {
            return Err(crate::EngineError::new("random bound must be positive"));
        }
        let threshold = upper.wrapping_neg() % upper;
        loop {
            let value = (u128::from(self.next_u64()) << 64) | u128::from(self.next_u64());
            if value >= threshold {
                return Ok(value % upper);
            }
        }
    }
}
