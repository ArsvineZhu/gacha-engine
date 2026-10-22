use std::collections::{BTreeMap, BTreeSet};

use crate::exec::{self, FullDraw, Memory, RuntimeError};
use crate::image::{BannerRt, Image};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Item(String),
    Any(Vec<String>),
    RarityAtLeast(u128),
    Tag(String),
}

#[derive(Clone, Debug)]
pub struct HitQuery {
    pub banner: String,
    pub action: String,
    pub times: u32,
    pub actions: u32,
    pub copies: u32,
    pub target: Target,
    pub quantiles: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct HitPoint {
    pub index: u32,
    pub hit_probability: f64,
    pub cumulative_probability: f64,
    pub survival_probability: f64,
    pub surviving_states: u64,
}

#[derive(Clone, Debug)]
pub struct Quantile {
    pub quantile: f64,
    pub index: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct HitReport {
    pub draws: u32,
    pub actions: u32,
    pub times: u32,
    pub probability_within: f64,
    pub probability_not_hit: f64,
    pub expected_draw_if_hit: Option<f64>,
    pub expected_draws_capped: f64,
    pub expected_action_if_hit: Option<f64>,
    pub expected_actions_capped: f64,
    pub peak_surviving_states: u64,
    pub final_surviving_states: u64,
    pub draws_distribution: Vec<HitPoint>,
    pub actions_distribution: Vec<HitPoint>,
    pub draw_quantiles: Vec<Quantile>,
    pub action_quantiles: Vec<Quantile>,
}

/// 首次命中是第 1 次命中。单步分支来自执行器的同一条转移，多步质量用 f64 汇总。
pub fn analyze(
    image: &Image,
    memory: &Memory,
    query: &HitQuery,
) -> Result<HitReport, RuntimeError> {
    if query.copies == 0 {
        return Err(RuntimeError::new("E_RANGE", "命中次数必须为正"));
    }
    if query.actions == 0 || query.times == 0 {
        return Err(RuntimeError::new("E_RANGE", "动作次数必须为正"));
    }
    let horizon = query
        .times
        .checked_mul(query.actions)
        .ok_or_else(|| RuntimeError::new("E_RANGE", "分析区间过大"))?;
    let banner = image
        .banners
        .iter()
        .find(|banner| banner.name == query.banner)
        .ok_or_else(|| RuntimeError::new("E_BANNER", format!("未知卡池 {}", query.banner)))?;
    let action = banner
        .actions
        .iter()
        .find(|action| action.name == query.action)
        .ok_or_else(|| RuntimeError::new("E_ACTION", format!("未知动作 {}", query.action)))?;
    let times = u128::from(query.times);
    if times < action.lo || times > action.hi {
        return Err(RuntimeError::new(
            "E_RANGE",
            format!(
                "动作 {} 的次数必须在 {} 到 {} 之间",
                action.name, action.lo, action.hi
            ),
        ));
    }
    let target = compile_target(banner, &query.target)?;
    validate_quantiles(&query.quantiles)?;

    let mut active = BTreeMap::<(Memory, u32), f64>::new();
    active.insert((memory.clone(), 0), 1.0);
    let mut draws = Vec::with_capacity(horizon as usize);
    let mut action_points = Vec::with_capacity(query.actions as usize);
    let mut cumulative = 0.0;
    let mut weighted_draws = 0.0;
    let mut weighted_actions = 0.0;
    let mut action_hit = 0.0;
    let mut peak = active.len() as u64;

    for draw in 1..=horizon {
        if (draw - 1) % query.times == 0 {
            let mut merged = BTreeMap::<(Memory, u32), f64>::new();
            for ((mut state, hits), mass) in active {
                exec::reset_action(image, &mut state, &query.banner)?;
                *merged.entry((state, hits)).or_insert(0.0) += mass;
            }
            active = merged;
        }
        let mut next = BTreeMap::<(Memory, u32), f64>::new();
        let mut hit = 0.0;
        for ((state, hits), mass) in active {
            if mass == 0.0 {
                continue;
            }
            let step = exec::step(image, &state, &query.banner, &query.action)?;
            absorb(
                &target,
                &step,
                hits,
                mass,
                query.copies,
                &mut hit,
                &mut next,
            )?;
        }
        if !hit.is_finite() || hit < 0.0 {
            return Err(RuntimeError::new("E_ANALYSIS", "命中概率无效"));
        }
        cumulative += hit;
        weighted_draws += hit * f64::from(draw);
        action_hit += hit;
        let survival = next.values().copied().sum::<f64>();
        if !survival.is_finite() || survival < 0.0 {
            return Err(RuntimeError::new("E_ANALYSIS", "存活概率无效"));
        }
        peak = peak.max(next.len() as u64);
        draws.push(HitPoint {
            index: draw,
            hit_probability: hit,
            cumulative_probability: cumulative.clamp(0.0, 1.0),
            survival_probability: survival.clamp(0.0, 1.0),
            surviving_states: next.len() as u64,
        });
        if draw % query.times == 0 {
            let action_index = draw / query.times;
            weighted_actions += action_hit * f64::from(action_index);
            action_points.push(HitPoint {
                index: action_index,
                hit_probability: action_hit,
                cumulative_probability: cumulative.clamp(0.0, 1.0),
                survival_probability: survival.clamp(0.0, 1.0),
                surviving_states: next.len() as u64,
            });
            action_hit = 0.0;
        }
        active = next;
    }

    let probability_not_hit = active.values().copied().sum::<f64>().clamp(0.0, 1.0);
    let probability_within = (1.0 - probability_not_hit).clamp(0.0, 1.0);
    if !probability_within.is_finite() {
        return Err(RuntimeError::new("E_ANALYSIS", "区间概率无效"));
    }
    let expected_draw_if_hit =
        (probability_within > 0.0).then_some(weighted_draws / probability_within);
    let expected_action_if_hit =
        (probability_within > 0.0).then_some(weighted_actions / probability_within);
    Ok(HitReport {
        draws: horizon,
        actions: query.actions,
        times: query.times,
        probability_within,
        probability_not_hit,
        expected_draw_if_hit,
        expected_draws_capped: weighted_draws + f64::from(horizon) * probability_not_hit,
        expected_action_if_hit,
        expected_actions_capped: weighted_actions + f64::from(query.actions) * probability_not_hit,
        peak_surviving_states: peak,
        final_surviving_states: active.len() as u64,
        draw_quantiles: quantiles_of(&query.quantiles, &draws),
        action_quantiles: quantiles_of(&query.quantiles, &action_points),
        draws_distribution: draws,
        actions_distribution: action_points,
    })
}

fn absorb(
    target: &CompiledTarget,
    step: &FullDraw,
    hits: u32,
    mass: f64,
    copies: u32,
    hit: &mut f64,
    next: &mut BTreeMap<(Memory, u32), f64>,
) -> Result<(), RuntimeError> {
    for branch in &step.branches {
        let branch_mass = mass * branch.public.probability.to_f64();
        if !branch_mass.is_finite() || branch_mass < 0.0 {
            return Err(RuntimeError::new("E_ANALYSIS", "分支质量无效"));
        }
        let matched = target.matches(&branch.public.entry, branch.public.rarity);
        let next_hits = hits + u32::from(matched);
        if next_hits >= copies {
            *hit += branch_mass;
        } else {
            *next
                .entry((branch.memory.clone(), next_hits))
                .or_insert(0.0) += branch_mass;
        }
    }
    Ok(())
}

enum CompiledTarget {
    Items(BTreeSet<String>),
    RarityAtLeast(u128),
}

impl CompiledTarget {
    fn matches(&self, entry: &str, rarity: u128) -> bool {
        match self {
            Self::Items(items) => items.contains(entry),
            Self::RarityAtLeast(minimum) => rarity >= *minimum,
        }
    }
}

fn compile_target(banner: &BannerRt, target: &Target) -> Result<CompiledTarget, RuntimeError> {
    match target {
        Target::Item(id) => {
            if !banner.entry_tags.contains_key(id) {
                return Err(RuntimeError::new("E_ENTRY", format!("未知条目 {id}")));
            }
            Ok(CompiledTarget::Items(BTreeSet::from([id.clone()])))
        }
        Target::Any(ids) => {
            if ids.is_empty() {
                return Err(RuntimeError::new("E_TARGET", "条目集合不能为空"));
            }
            let mut items = BTreeSet::new();
            for id in ids {
                if !banner.entry_tags.contains_key(id) {
                    return Err(RuntimeError::new("E_ENTRY", format!("未知条目 {id}")));
                }
                items.insert(id.clone());
            }
            Ok(CompiledTarget::Items(items))
        }
        Target::RarityAtLeast(rarity) => {
            if *rarity == 0 {
                return Err(RuntimeError::new("E_TARGET", "最低稀有度必须为正"));
            }
            Ok(CompiledTarget::RarityAtLeast(*rarity))
        }
        Target::Tag(tag) => {
            if tag.is_empty() {
                return Err(RuntimeError::new("E_TARGET", "标签不能为空"));
            }
            if !banner
                .entry_tags
                .values()
                .any(|tags| tags.iter().any(|item| item == tag))
            {
                return Err(RuntimeError::new("E_TARGET", format!("未知标签 {tag}")));
            }
            let items = banner
                .entry_tags
                .iter()
                .filter(|(_, tags)| tags.iter().any(|item| item == tag))
                .map(|(name, _)| name.clone())
                .collect();
            Ok(CompiledTarget::Items(items))
        }
    }
}

fn validate_quantiles(quantiles: &[f64]) -> Result<(), RuntimeError> {
    for quantile in quantiles {
        if !quantile.is_finite() || *quantile <= 0.0 || *quantile > 1.0 {
            return Err(RuntimeError::new(
                "E_RANGE",
                format!("分位数必须在 (0, 1] 内，实际 {quantile}"),
            ));
        }
    }
    Ok(())
}

fn quantiles_of(quantiles: &[f64], points: &[HitPoint]) -> Vec<Quantile> {
    const EPSILON: f64 = 1e-12;
    quantiles
        .iter()
        .copied()
        .map(|quantile| Quantile {
            quantile,
            index: points
                .iter()
                .find(|point| point.cumulative_probability + EPSILON >= quantile)
                .map(|point| point.index),
        })
        .collect()
}
