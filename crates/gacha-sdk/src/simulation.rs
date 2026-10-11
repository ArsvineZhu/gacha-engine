use crate::*;
use gacha_core::SplitMix64;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SamplePoint {
    pub draw: u32,
    pub count: u32,
    pub probability: f64,
    pub cumulative_probability: f64,
}
#[derive(Debug, Clone, Serialize)]
pub struct SimulationReport {
    pub trials: u32,
    pub seed: u64,
    pub not_hit_trials: u32,
    pub mean_actions_if_hit: Option<f64>,
    pub distribution: Vec<SamplePoint>,
    pub preparation: PreparationStats,
}
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonReport {
    pub maximum_cdf_error: f64,
    pub six_sigma_passed: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct StudyReport {
    pub analysis: AnalysisReport,
    pub simulation: SimulationReport,
    pub comparison: ComparisonReport,
}

impl Engine {
    pub fn simulate(
        &self,
        request: &AnalysisRequest,
        trials: u32,
        seed: u64,
    ) -> Result<SimulationReport> {
        if trials == 0 || request.copies == 0 {
            return Err(EngineError::new("trials and copies must be positive"));
        }
        let query = self.query(request)?;
        let size = self.action_size(&request.banner, &request.action)?;
        let mask = gacha_analysis::target_mask(self.runtime.game(), &request.target)?;
        let mut counts = vec![0_u32; query.draws as usize];
        let mut rng = SplitMix64::new(seed);
        for _ in 0..trials {
            let mut state = request.state.clone();
            let mut hits = 0;
            'trial: for action in 0..request.actions {
                let branches = self.runtime.sample_action(
                    &request.banner,
                    &request.action,
                    &mut state,
                    &request.context,
                    &mut rng,
                )?;
                for (position, branch) in branches.iter().enumerate() {
                    hits += u32::from(mask[branch.outcome.item.0]);
                    if hits >= request.copies {
                        counts[(action * size) as usize + position] += 1;
                        break 'trial;
                    }
                }
            }
        }
        let mut cumulative = 0;
        let mut weighted_actions = 0.0;
        let distribution = counts
            .into_iter()
            .enumerate()
            .map(|(i, count)| {
                cumulative += count;
                let draw = i as u32 + 1;
                weighted_actions += f64::from(count) * f64::from(draw.div_ceil(size));
                SamplePoint {
                    draw,
                    count,
                    probability: f64::from(count) / f64::from(trials),
                    cumulative_probability: f64::from(cumulative) / f64::from(trials),
                }
            })
            .collect();
        Ok(SimulationReport {
            trials,
            seed,
            not_hit_trials: trials - cumulative,
            mean_actions_if_hit: (cumulative > 0).then(|| weighted_actions / f64::from(cumulative)),
            distribution,
            preparation: self.preparation(),
        })
    }

    pub fn study(&self, request: &AnalysisRequest, trials: u32, seed: u64) -> Result<StudyReport> {
        let analysis = self.analyze(request)?;
        let simulation = self.simulate(request, trials, seed)?;
        let mut maximum_cdf_error: f64 = 0.0;
        let mut six_sigma_passed = true;
        for (theory, sample) in analysis
            .draw_distribution
            .distribution
            .iter()
            .zip(&simulation.distribution)
        {
            let p = theory.cumulative_probability;
            let error = (p - sample.cumulative_probability).abs();
            maximum_cdf_error = maximum_cdf_error.max(error);
            let limit = 6.0 * (p * (1.0 - p) / f64::from(trials)).sqrt() + 1.0 / f64::from(trials);
            six_sigma_passed &= error <= limit;
        }
        Ok(StudyReport {
            analysis,
            simulation,
            comparison: ComparisonReport {
                maximum_cdf_error,
                six_sigma_passed,
            },
        })
    }
}
