use crate::engine::{SelectorTables, finish_outcome, resolve_outcomes, static_selector_tables};
use crate::*;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
mod dependencies;
mod parameters;
mod trace;
pub use trace::*;

#[derive(Debug, Clone, Default, Serialize)]
pub struct PreparationStats {
    pub static_selectors: usize,
    pub numeric_tables: usize,
    pub cached_plans: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_clears: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PlanKey {
    banner: usize,
    action: usize,
    values: Vec<Rational>,
    guards: Vec<bool>,
}

#[derive(Default)]
struct Cache {
    plans: BTreeMap<PlanKey, Arc<OutcomePlan>>,
    stats: PreparationStats,
}

/// 不可变规则及有界概率计划缓存；状态更新与事件始终读取当前完整状态。
pub struct PreparedRuntime {
    game: CompiledGame,
    tables: SelectorTables,
    inputs: Vec<Vec<parameters::DecisionInputs>>,
    outcome_classes: Vec<Vec<Vec<Vec<bool>>>>,
    capacity: usize,
    cache: Mutex<Cache>,
}

impl PreparedRuntime {
    pub fn new(game: CompiledGame) -> Result<Self> {
        Self::with_capacity(game, 4096)
    }

    pub fn with_capacity(game: CompiledGame, capacity: usize) -> Result<Self> {
        let tables = static_selector_tables(&game)?;
        let inputs = game
            .banners
            .iter()
            .map(|b| {
                b.actions
                    .iter()
                    .map(|a| parameters::DecisionInputs::compile(&game, b, a))
                    .collect()
            })
            .collect();
        let outcome_classes = game
            .banners
            .iter()
            .map(|b| {
                b.actions
                    .iter()
                    .map(|a| dependencies::outcome_classes(&game, a))
                    .collect()
            })
            .collect();
        Ok(Self {
            game,
            tables,
            inputs,
            outcome_classes,
            capacity,
            cache: Mutex::new(Cache::default()),
        })
    }

    pub fn game(&self) -> &CompiledGame {
        &self.game
    }

    pub fn stats(&self) -> PreparationStats {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        PreparationStats {
            static_selectors: self.tables.len(),
            numeric_tables: self.inputs.iter().flatten().map(|i| i.dense_tables()).sum(),
            cached_plans: cache.plans.len(),
            ..cache.stats.clone()
        }
    }

    fn action(&self, banner: &str, action: &str) -> Result<(usize, usize)> {
        let (id, b) = self
            .game
            .banner(banner)
            .ok_or_else(|| EngineError::new(format!("unknown banner: {banner}")))?;
        let index = *b
            .action_lookup
            .get(action)
            .ok_or_else(|| EngineError::new(format!("unknown action: {action}")))?;
        Ok((id.0, index))
    }

    fn plan(
        &self,
        banner: usize,
        action: usize,
        state: &StateStore,
        context: &ScopeContext,
    ) -> Result<Arc<OutcomePlan>> {
        let b = &self.game.banners[banner];
        let (values, guards) = self.inputs[banner][action].values(&self.game, b, state, context)?;
        let key = PlanKey {
            banner,
            action,
            values,
            guards,
        };
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(plan) = cache.plans.get(&key).cloned() {
                cache.stats.cache_hits += 1;
                return Ok(plan);
            }
            cache.stats.cache_misses += 1;
        }
        let mut plan = resolve_outcomes(
            &self.game,
            b,
            &b.actions[action],
            state,
            context,
            Some(&self.tables),
        )?;
        let mut remaining = Rational::ONE;
        for choice in &plan.choices {
            plan.sampling
                .push(choice.probability.checked_div(remaining)?);
            remaining = remaining.checked_sub(choice.probability)?;
        }
        let plan = Arc::new(plan);
        if self.capacity > 0 {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if cache.plans.len() >= self.capacity {
                cache.plans.clear();
                cache.stats.cache_clears += 1;
            }
            cache.plans.insert(key, plan.clone());
        }
        Ok(plan)
    }

    fn normalize(
        &self,
        banner: &str,
        state: &StateStore,
        context: &ScopeContext,
    ) -> Result<StateStore> {
        let mut state = state.clone();
        prune_ephemeral_state(&self.game, banner, &mut state, context)?;
        let (_, b) = self
            .game
            .banner(banner)
            .ok_or_else(|| EngineError::new("unknown banner"))?;
        materialize_state(&self.game, b, &mut state, context)?;
        Ok(state)
    }

    pub fn enumerate_step(
        &self,
        banner: &str,
        action: &str,
        state: &StateStore,
        context: &ScopeContext,
    ) -> Result<Vec<Branch>> {
        let (bi, ai) = self.action(banner, action)?;
        let state = self.normalize(banner, state, context)?;
        let plan = self.plan(bi, ai, &state, context)?;
        let b = &self.game.banners[bi];
        plan.choices
            .iter()
            .map(|choice| {
                finish_outcome(
                    &self.game,
                    b,
                    &b.actions[ai],
                    &state,
                    context,
                    &plan,
                    choice,
                )
            })
            .collect()
    }

    /// 分析专用：按目标命中与全部结果条件的真值合并条目，再执行一次状态更新。
    pub fn enumerate_grouped_step<F: Fn(Outcome) -> bool>(
        &self,
        banner: &str,
        action: &str,
        state: &StateStore,
        context: &ScopeContext,
        target: F,
    ) -> Result<Vec<Branch>> {
        let (bi, ai) = self.action(banner, action)?;
        let state = self.normalize(banner, state, context)?;
        let plan = self.plan(bi, ai, &state, context)?;
        let mut groups = BTreeMap::<(Vec<bool>, bool), OutcomeChoice>::new();
        for choice in &plan.choices {
            let key = (
                self.outcome_classes[bi][ai][choice.outcome.item.0].clone(),
                target(choice.outcome),
            );
            if let Some(current) = groups.get_mut(&key) {
                current.probability = current.probability.checked_add(choice.probability)?;
            } else {
                groups.insert(key, choice.clone());
            }
        }
        let b = &self.game.banners[bi];
        groups
            .values()
            .map(|choice| {
                finish_outcome(
                    &self.game,
                    b,
                    &b.actions[ai],
                    &state,
                    context,
                    &plan,
                    choice,
                )
            })
            .collect()
    }

    pub fn sample_step(
        &self,
        banner: &str,
        action: &str,
        state: &mut StateStore,
        context: &ScopeContext,
        rng: &mut SplitMix64,
    ) -> Result<Branch> {
        let (bi, ai) = self.action(banner, action)?;
        let normalized = self.normalize(banner, state, context)?;
        let plan = self.plan(bi, ai, &normalized, context)?;
        let mut next_rng = rng.clone();
        for (choice, conditional) in plan.choices.iter().zip(&plan.sampling) {
            if *conditional == Rational::ONE
                || next_rng.below_u128(conditional.denominator())? < conditional.numerator()
            {
                let b = &self.game.banners[bi];
                let branch = finish_outcome(
                    &self.game,
                    b,
                    &b.actions[ai],
                    &normalized,
                    context,
                    &plan,
                    choice,
                )?;
                *state = branch.next_state.clone();
                *rng = next_rng;
                return Ok(branch);
            }
        }
        Err(EngineError::new("outcome probabilities do not sum to one"))
    }

    pub fn sample_action(
        &self,
        banner: &str,
        action: &str,
        state: &mut StateStore,
        context: &ScopeContext,
        rng: &mut SplitMix64,
    ) -> Result<Vec<Branch>> {
        let (bi, ai) = self.action(banner, action)?;
        let draws = self.game.banners[bi].actions[ai].draws;
        let mut next = state.clone();
        let mut next_rng = rng.clone();
        reset_action_state(&self.game, &mut next);
        let mut results = Vec::with_capacity(draws as usize);
        for draw in 0..draws {
            let mut ctx = context.clone();
            ctx.pull = format!("{}:{draw}", context.pull);
            results.push(self.sample_step(banner, action, &mut next, &ctx, &mut next_rng)?);
        }
        *state = next;
        *rng = next_rng;
        Ok(results)
    }
}
