use gacha_core::{
    CompiledGame, EngineError, ItemId, Outcome, Result, ScopeContext, StateStore, enumerate_step,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

enum CompiledTarget {
    Items(BTreeSet<ItemId>),
    RarityAtLeast(u8),
    Tag(String),
}

impl CompiledTarget {
    fn matches(&self, game: &CompiledGame, outcome: Outcome) -> bool {
        match self {
            Self::Items(items) => items.contains(&outcome.item),
            Self::RarityAtLeast(rarity) => outcome.rarity >= *rarity,
            Self::Tag(tag) => game
                .item(outcome.item)
                .tags
                .iter()
                .any(|item_tag| item_tag == tag),
        }
    }
}

fn compile_target(game: &CompiledGame, target: &TargetSpec) -> Result<CompiledTarget> {
    match target {
        TargetSpec::Item { id } => {
            let item = game
                .item_id(id)
                .ok_or_else(|| EngineError::new(format!("unknown target item: {id}")))?;
            Ok(CompiledTarget::Items(BTreeSet::from([item])))
        }
        TargetSpec::AnyItem { ids } => {
            if ids.is_empty() {
                return Err(EngineError::new("any_item target must contain at least one item"));
            }
            let mut items = BTreeSet::new();
            for id in ids {
                let item = game
                    .item_id(id)
                    .ok_or_else(|| EngineError::new(format!("unknown target item: {id}")))?;
                items.insert(item);
            }
            Ok(CompiledTarget::Items(items))
        }
        TargetSpec::RarityAtLeast { rarity } => {
            if *rarity == 0 {
                return Err(EngineError::new("rarity_at_least target must be greater than zero"));
            }
            Ok(CompiledTarget::RarityAtLeast(*rarity))
        }
        TargetSpec::Tag { tag } => {
            if tag.trim().is_empty() {
                return Err(EngineError::new("tag target must not be empty"));
            }
            if !game
                .items
                .iter()
                .any(|item| item.tags.iter().any(|item_tag| item_tag == tag))
            {
                return Err(EngineError::new(format!("unknown target tag: {tag}")));
            }
            Ok(CompiledTarget::Tag(tag.clone()))
        }
    }
}

fn validate_quantiles(quantiles: &[f64]) -> Result<()> {
    for quantile in quantiles {
        if !quantile.is_finite() || *quantile <= 0.0 || *quantile > 1.0 {
            return Err(EngineError::new(format!(
                "quantile must be finite and in (0, 1], got {quantile}"
            )));
        }
    }
    Ok(())
}

/// Computes the first-hit distribution for a target over a finite horizon.
///
/// Every one-step transition comes from the same exact rational rule graph used by
/// simulation. Multi-step state mass uses f64 to keep long-horizon state aggregation
/// bounded and practical.
pub fn analyze_first_hit(
    game: &CompiledGame,
    initial_state: &StateStore,
    context: &ScopeContext,
    query: &FirstHitQuery,
) -> Result<FirstHitAnalysis> {
    let target = compile_target(game, &query.target)?;
    validate_quantiles(&query.quantiles)?;

    let mut active = BTreeMap::<StateStore, f64>::new();
    active.insert(initial_state.clone(), 1.0);

    let mut distribution = Vec::with_capacity(query.draws as usize);
    let mut cumulative_hit = 0.0_f64;
    let mut weighted_hit_draws = 0.0_f64;
    let mut peak_surviving_states = active.len();

    for draw in 1..=query.draws {
        let mut draw_context = context.clone();
        draw_context.pull = format!("{}:{draw}", context.pull);

        let mut next = BTreeMap::<StateStore, f64>::new();
        let mut first_hit = 0.0_f64;

        for (state, mass) in active {
            if mass == 0.0 {
                continue;
            }
            let branches = enumerate_step(
                game,
                &query.banner,
                &query.action,
                &state,
                &draw_context,
            )?;
            for branch in branches {
                let branch_mass = mass * branch.probability.to_f64();
                if target.matches(game, branch.outcome) {
                    first_hit += branch_mass;
                } else {
                    *next.entry(branch.next_state).or_insert(0.0) += branch_mass;
                }
            }
        }

        if !first_hit.is_finite() || first_hit < 0.0 {
            return Err(EngineError::new(
                "analysis produced an invalid first-hit probability",
            ));
        }

        cumulative_hit += first_hit;
        weighted_hit_draws += first_hit * f64::from(draw);
        let survival = next.values().copied().sum::<f64>();
        if !survival.is_finite() || survival < 0.0 {
            return Err(EngineError::new(
                "analysis produced an invalid survival probability",
            ));
        }

        peak_surviving_states = peak_surviving_states.max(next.len());
        distribution.push(FirstHitPoint {
            draw,
            first_hit_probability: first_hit,
            cumulative_probability: cumulative_hit.clamp(0.0, 1.0),
            survival_probability: survival.clamp(0.0, 1.0),
            surviving_states: next.len(),
        });
        active = next;
    }

    let probability_not_hit = active.values().copied().sum::<f64>().clamp(0.0, 1.0);
    let probability_within = (1.0 - probability_not_hit).clamp(0.0, 1.0);
    if !probability_within.is_finite() {
        return Err(EngineError::new(
            "analysis produced a non-finite probability",
        ));
    }

    let expected_draw_if_hit = if probability_within > 0.0 {
        Some(weighted_hit_draws / probability_within)
    } else {
        None
    };
    let expected_draws_capped_at_horizon =
        weighted_hit_draws + f64::from(query.draws) * probability_not_hit;

    const QUANTILE_EPSILON: f64 = 1e-12;
    let quantiles = query
        .quantiles
        .iter()
        .copied()
        .map(|quantile| {
            let draw = distribution
                .iter()
                .find(|point| point.cumulative_probability + QUANTILE_EPSILON >= quantile)
                .map(|point| point.draw);
            QuantileResult { quantile, draw }
        })
        .collect();

    Ok(FirstHitAnalysis {
        horizon: query.draws,
        probability_within,
        probability_not_hit,
        expected_draw_if_hit,
        expected_draws_capped_at_horizon,
        peak_surviving_states,
        final_surviving_states: active.len(),
        distribution,
        quantiles,
    })
}

/// Backwards-compatible convenience wrapper for the original single-item query.
pub fn probability_of_item_within(
    game: &CompiledGame,
    banner: &str,
    action: &str,
    initial_state: &StateStore,
    context: &ScopeContext,
    target_item: &str,
    draws: u32,
) -> Result<ProbabilityResult> {
    let query = FirstHitQuery {
        banner: banner.to_string(),
        action: action.to_string(),
        target: TargetSpec::Item {
            id: target_item.to_string(),
        },
        draws,
        quantiles: Vec::new(),
    };
    let result = analyze_first_hit(game, initial_state, context, &query)?;
    Ok(ProbabilityResult {
        probability: result.probability_within,
        surviving_states: result.final_surviving_states,
    })
}
