use crate::*;
use gacha_analysis::{FirstHitAnalysis, FirstHitQuery, analyze_nth_hit_prepared};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub banner: String,
    pub action: String,
    pub target: TargetSpec,
    /// 完整动作次数，武器申领一次就是十件。
    pub actions: u32,
    #[serde(default = "one")]
    pub copies: u32,
    #[serde(default = "default_quantiles")]
    pub quantiles: Vec<f64>,
    #[serde(default)]
    pub state: StateStore,
    #[serde(default)]
    pub context: ScopeContext,
}
fn one() -> u32 {
    1
}
fn default_quantiles() -> Vec<f64> {
    vec![0.5, 0.9, 0.95, 0.99, 1.0]
}

impl AnalysisRequest {
    pub fn first_hit(banner: &str, action: &str, target: TargetSpec, actions: u32) -> Self {
        Self {
            banner: banner.into(),
            action: action.into(),
            target,
            actions,
            copies: 1,
            quantiles: vec![0.5, 0.9, 0.95, 0.99, 1.0],
            state: StateStore::default(),
            context: ScopeContext::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionPoint {
    pub action: u32,
    pub first_hit_probability: f64,
    pub cumulative_probability: f64,
    pub survival_probability: f64,
}
#[derive(Debug, Clone, Serialize)]
pub struct ActionQuantile {
    pub quantile: f64,
    pub action: Option<u32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisReport {
    pub fingerprint: String,
    pub request: AnalysisRequest,
    pub draws_per_action: u32,
    pub expected_actions_if_hit: Option<f64>,
    pub expected_actions_capped_at_horizon: f64,
    pub action_quantiles: Vec<ActionQuantile>,
    pub action_distribution: Vec<ActionPoint>,
    pub draw_distribution: FirstHitAnalysis,
    pub preparation: PreparationStats,
    pub sources: Vec<SourceDef>,
    pub assumptions: Vec<AssumptionDef>,
    pub numerical_method: String,
}

impl Engine {
    pub(crate) fn query(&self, request: &AnalysisRequest) -> Result<FirstHitQuery> {
        let size = self.action_size(&request.banner, &request.action)?;
        let draws = request
            .actions
            .checked_mul(size)
            .ok_or_else(|| EngineError::new("analysis horizon overflow"))?;
        Ok(FirstHitQuery {
            banner: request.banner.clone(),
            action: request.action.clone(),
            target: request.target.clone(),
            draws,
            quantiles: request.quantiles.clone(),
        })
    }

    pub fn analyze(&self, request: &AnalysisRequest) -> Result<AnalysisReport> {
        let query = self.query(request)?;
        let size = self.action_size(&request.banner, &request.action)?;
        let result = analyze_nth_hit_prepared(
            &self.runtime,
            &request.state,
            &request.context,
            &query,
            request.copies,
        )?;
        let mut action_distribution = Vec::new();
        let mut previous = 0.0;
        for point in &result.distribution {
            if point.draw.is_multiple_of(size) {
                action_distribution.push(ActionPoint {
                    action: point.draw / size,
                    first_hit_probability: point.cumulative_probability - previous,
                    cumulative_probability: point.cumulative_probability,
                    survival_probability: point.survival_probability,
                });
                previous = point.cumulative_probability;
            }
        }
        let weighted = action_distribution
            .iter()
            .map(|p| f64::from(p.action) * p.first_hit_probability)
            .sum::<f64>();
        let action_quantiles = result
            .quantiles
            .iter()
            .map(|q| ActionQuantile {
                quantile: q.quantile,
                action: q.draw.map(|d| d.div_ceil(size)),
            })
            .collect();
        Ok(AnalysisReport {
            fingerprint: self.fingerprint.clone(),
            request: request.clone(),
            draws_per_action: size,
            expected_actions_if_hit: (result.probability_within > 0.0)
                .then(|| weighted / result.probability_within),
            expected_actions_capped_at_horizon: weighted
                + f64::from(request.actions) * result.probability_not_hit,
            action_quantiles,
            action_distribution,
            draw_distribution: result,
            preparation: self.preparation(),
            sources: self.sources().to_vec(),
            assumptions: self.assumptions().to_vec(),
            numerical_method:
                "单步概率为精确有理数，多步质量以 f64 汇总；动作分位数在完整动作边界计算".into(),
        })
    }
}
