use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetSpec {
    Item { id: String },
    AnyItem { ids: Vec<String> },
    RarityAtLeast { rarity: u8 },
    Tag { tag: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstHitQuery {
    pub banner: String,
    pub action: String,
    pub target: TargetSpec,
    /// 底层抽取次数；多抽动作的区间必须包含完整动作。
    pub draws: u32,
    #[serde(default)]
    pub quantiles: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstHitPoint {
    pub draw: u32,
    pub first_hit_probability: f64,
    pub cumulative_probability: f64,
    pub survival_probability: f64,
    pub surviving_states: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantileResult {
    pub quantile: f64,
    pub draw: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstHitAnalysis {
    pub horizon: u32,
    pub probability_within: f64,
    pub probability_not_hit: f64,
    pub expected_draw_if_hit: Option<f64>,
    pub expected_draws_capped_at_horizon: f64,
    pub peak_surviving_states: usize,
    pub final_surviving_states: usize,
    pub distribution: Vec<FirstHitPoint>,
    pub quantiles: Vec<QuantileResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityResult {
    pub probability: f64,
    pub surviving_states: usize,
}
