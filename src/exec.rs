use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::image::*;
use crate::lex::Scope;
use crate::rational::Rational;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeError {
    pub code: &'static str,
    pub message: String,
}

impl RuntimeError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<Diagnostic> for RuntimeError {
    fn from(error: Diagnostic) -> Self {
        Self::new(
            error.code,
            format!(
                "{}:{}:{}: 期望 {}，实际 {}",
                error.file, error.line, error.column, error.expected, error.actual
            ),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionInfo {
    pub name: String,
    pub min_times: u128,
    pub max_times: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateInfo {
    pub name: String,
    pub scope: String,
    pub kind: String,
}

impl StateInfo {
    pub(crate) fn from_state(state: &StateRt) -> Self {
        Self {
            name: state.name.clone(),
            scope: scope_name(state.scope).to_string(),
            kind: if state.count.is_some() {
                "count"
            } else {
                "bool"
            }
            .to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotState {
    pub scope: String,
    pub group: String,
    pub name: String,
    pub count: Option<u128>,
    pub flag: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct Draw {
    pub guarantee: Option<String>,
    pub branches: Vec<Branch>,
}

#[derive(Clone, Debug)]
pub struct Branch {
    pub probability: Rational,
    pub entry: String,
    pub rarity: u128,
    pub events: Vec<Fired>,
}

#[derive(Clone, Debug)]
pub struct Fired {
    pub name: String,
    pub args: Vec<FiredArg>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FiredArg {
    Count(u128),
    Bool(bool),
    Rarity(u128),
    Item(String),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Memory {
    slots: BTreeMap<String, SlotVal>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SlotVal {
    Count(u128),
    Bool(bool),
}

pub struct Session {
    image: Arc<Image>,
    memory: Memory,
}

impl Session {
    pub fn new(image: Arc<Image>) -> Self {
        let mut memory = Memory {
            slots: BTreeMap::new(),
        };
        for banner in &image.banners {
            for state in &banner.states {
                let key = slot_key(banner, state);
                memory.slots.entry(key).or_insert_with(|| {
                    if let Some(count) = &state.count {
                        SlotVal::Count(count.initial)
                    } else {
                        SlotVal::Bool(state.bool_initial.unwrap_or(false))
                    }
                });
            }
        }
        Self { image, memory }
    }

    pub(crate) fn image(&self) -> &Image {
        &self.image
    }

    pub(crate) fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn set_count(&mut self, banner: &str, name: &str, value: u128) -> Result<(), RuntimeError> {
        let (banner, state) = self.counter(banner, name)?;
        if state
            .count
            .as_ref()
            .and_then(|count| count.upper)
            .is_some_and(|limit| value > limit)
        {
            return Err(RuntimeError::new(
                "E_COUNTER_RANGE",
                format!("{name} 超出上界"),
            ));
        }
        let key = slot_key(banner, state);
        self.memory.slots.insert(key, SlotVal::Count(value));
        Ok(())
    }

    pub fn set_bool(&mut self, banner: &str, name: &str, value: bool) -> Result<(), RuntimeError> {
        let (banner, state) = self.flag(banner, name)?;
        let key = slot_key(banner, state);
        self.memory.slots.insert(key, SlotVal::Bool(value));
        Ok(())
    }

    pub fn get_count(&self, banner: &str, name: &str) -> Result<u128, RuntimeError> {
        let (banner, state) = self.counter(banner, name)?;
        Ok(match self.memory.slots.get(&slot_key(banner, state)) {
            Some(SlotVal::Count(value)) => *value,
            _ => 0,
        })
    }

    pub fn get_bool(&self, banner: &str, name: &str) -> Result<bool, RuntimeError> {
        let (banner, state) = self.flag(banner, name)?;
        Ok(match self.memory.slots.get(&slot_key(banner, state)) {
            Some(SlotVal::Bool(value)) => *value,
            _ => false,
        })
    }

    pub fn snapshot(&self) -> Vec<SlotState> {
        let mut seen = BTreeSet::new();
        let mut slots = Vec::new();
        for banner in &self.image.banners {
            for state in &banner.states {
                let key = slot_key(banner, state);
                if !seen.insert(key.clone()) {
                    continue;
                }
                let (count, flag) = match self.memory.slots.get(&key) {
                    Some(SlotVal::Count(value)) => (Some(*value), None),
                    Some(SlotVal::Bool(value)) => (None, Some(*value)),
                    None => (None, None),
                };
                slots.push(SlotState {
                    scope: scope_name(state.scope).to_string(),
                    group: group_of(banner, state),
                    name: state.name.clone(),
                    count,
                    flag,
                });
            }
        }
        slots
    }

    pub fn restore(&mut self, slots: &[SlotState]) -> Result<(), RuntimeError> {
        for slot in slots {
            let scope = parse_scope(&slot.scope)?;
            let found = self.image.banners.iter().find_map(|banner| {
                banner
                    .states
                    .iter()
                    .find(|state| {
                        state.name == slot.name
                            && state.scope == scope
                            && group_of(banner, state) == slot.group
                    })
                    .map(|state| (banner, state))
            });
            let Some((banner, state)) = found else {
                return Err(RuntimeError::new(
                    "E_STATE",
                    format!("无法恢复状态 {} {} {}", slot.scope, slot.group, slot.name),
                ));
            };
            let key = slot_key(banner, state);
            match (state.count.is_some(), slot.count, slot.flag) {
                (true, Some(value), _) => {
                    if state
                        .count
                        .as_ref()
                        .and_then(|count| count.upper)
                        .is_some_and(|limit| value > limit)
                    {
                        return Err(RuntimeError::new(
                            "E_COUNTER_RANGE",
                            format!("{} 超出上界", slot.name),
                        ));
                    }
                    self.memory.slots.insert(key, SlotVal::Count(value));
                }
                (false, _, Some(value)) => {
                    self.memory.slots.insert(key, SlotVal::Bool(value));
                }
                _ => {
                    return Err(RuntimeError::new(
                        "E_STATE_TYPE",
                        format!("{} 的值类型不符", slot.name),
                    ));
                }
            }
        }
        Ok(())
    }

    fn counter<'b>(
        &'b self,
        banner: &str,
        name: &str,
    ) -> Result<(&'b BannerRt, &'b StateRt), RuntimeError> {
        let (banner, state) = self.slot(banner, name)?;
        if state.count.is_none() {
            return Err(RuntimeError::new(
                "E_STATE_TYPE",
                format!("{name} 不是计数器"),
            ));
        }
        Ok((banner, state))
    }

    fn flag<'b>(
        &'b self,
        banner: &str,
        name: &str,
    ) -> Result<(&'b BannerRt, &'b StateRt), RuntimeError> {
        let (banner, state) = self.slot(banner, name)?;
        if state.count.is_some() {
            return Err(RuntimeError::new(
                "E_STATE_TYPE",
                format!("{name} 不是布尔状态"),
            ));
        }
        Ok((banner, state))
    }

    fn slot<'b>(
        &'b self,
        banner_name: &str,
        name: &str,
    ) -> Result<(&'b BannerRt, &'b StateRt), RuntimeError> {
        let banner = find_banner_opt(&self.image, banner_name)
            .ok_or_else(|| RuntimeError::new("E_BANNER", format!("未知卡池 {banner_name}")))?;
        let state = find_state_opt(banner, name)
            .ok_or_else(|| RuntimeError::new("E_STATE", format!("未知状态 {name}")))?;
        Ok((banner, state))
    }

    pub fn draw(&self, banner: &str, action: &str) -> Result<Draw, RuntimeError> {
        let full = self.expand(banner, action)?;
        Ok(Draw {
            guarantee: full.guarantee,
            branches: full
                .branches
                .into_iter()
                .map(|branch| branch.public)
                .collect(),
        })
    }

    pub fn apply(
        &mut self,
        banner: &str,
        action: &str,
        entry: &str,
    ) -> Result<Branch, RuntimeError> {
        let full = self.expand(banner, action)?;
        let has_mass = full
            .branches
            .iter()
            .any(|branch| !branch.public.probability.is_zero());
        let chosen = full
            .branches
            .into_iter()
            .find(|branch| branch.public.entry == entry && !branch.public.probability.is_zero())
            .ok_or_else(|| {
                if has_mass {
                    RuntimeError::new("E_ENTRY", format!("条目 {entry} 不在本次正概率分支中"))
                } else {
                    RuntimeError::new("E_DRAW_EMPTY_MASS", "本次抽取没有正概率分支")
                }
            })?;
        self.memory = chosen.memory;
        Ok(chosen.public)
    }

    pub fn sample_with_seed(
        &mut self,
        banner: &str,
        action: &str,
        seed: u64,
    ) -> Result<Branch, RuntimeError> {
        let mut rng = Rng::new(seed);
        self.sample(banner, action, &mut rng)
    }

    pub fn sample(
        &mut self,
        banner: &str,
        action: &str,
        rng: &mut Rng,
    ) -> Result<Branch, RuntimeError> {
        let full = self.expand(banner, action)?;
        let index = sample_index(&full.branches, rng);
        let chosen = full.branches.into_iter().nth(index).expect("分支");
        self.memory = chosen.memory;
        Ok(chosen.public)
    }

    fn expand(&self, banner_name: &str, action_name: &str) -> Result<FullDraw, RuntimeError> {
        expand_with(&self.image, &self.memory, banner_name, action_name)
    }
}

fn expand_with(
    image: &Image,
    memory: &Memory,
    banner_name: &str,
    action_name: &str,
) -> Result<FullDraw, RuntimeError> {
    let banner = find_banner_opt(image, banner_name)
        .ok_or_else(|| RuntimeError::new("E_BANNER", format!("未知卡池 {banner_name}")))?;
    let action = banner
        .actions
        .iter()
        .find(|action| action.name == action_name)
        .ok_or_else(|| RuntimeError::new("E_ACTION", format!("未知动作 {action_name}")))?;
    let dist = &banner.distributions[action.distribution];
    let mut guarantee = None;
    let mut rarity_probs = rarity_distribution(banner, dist, memory)?;
    for rule in &banner.guarantees {
        if eval_cond(banner, &rule.when, memory, None) {
            guarantee = Some(rule.name.clone());
            match &rule.effect {
                EffectRt::Item(slot) => {
                    let entry = bound_item(banner, slot);
                    let rarity = entry_rarity(banner, entry);
                    return Ok(finish(
                        banner,
                        memory,
                        guarantee,
                        vec![(entry.to_string(), rarity, Rational::one())],
                    ));
                }
                EffectRt::Rarity(rarity) => {
                    rarity_probs = vec![(*rarity, Rational::one(), selector_for(dist, *rarity))];
                }
                EffectRt::FloorRescale(k) => {
                    let mass = rarity_probs
                        .iter()
                        .filter(|(rarity, _, _)| *rarity >= *k)
                        .try_fold(Rational::zero(), |acc, (_, p, _)| acc.checked_add(*p))
                        .expect("质量");
                    if mass.is_zero() {
                        return Err(RuntimeError::new(
                            "E_DRAW_EMPTY_MASS",
                            "条件重抽的可用质量为零",
                        ));
                    }
                    rarity_probs = rarity_probs
                        .into_iter()
                        .filter(|(rarity, _, _)| *rarity >= *k)
                        .map(|(rarity, p, sel)| (rarity, p.checked_div(mass).expect("重抽"), sel))
                        .collect();
                }
                EffectRt::FloorPromote(k) => {
                    let higher = rarity_probs
                        .iter()
                        .filter(|(rarity, _, _)| *rarity > *k)
                        .try_fold(Rational::zero(), |acc, (_, p, _)| acc.checked_add(*p))
                        .expect("质量");
                    let mut next = rarity_probs
                        .into_iter()
                        .filter(|(rarity, _, _)| *rarity > *k)
                        .collect::<Vec<_>>();
                    next.push((
                        *k,
                        Rational::one().checked_sub(higher).expect("门槛"),
                        selector_for(dist, *k),
                    ));
                    rarity_probs = next;
                }
            }
            break;
        }
    }
    if guarantee.is_none() {
        rarity_probs = rarity_probs
            .into_iter()
            .map(|(rarity, p, _)| (rarity, p, selector_for(dist, rarity)))
            .collect();
    }
    let mut items = BTreeMap::<(String, u128), Rational>::new();
    for (rarity, probability, selector) in rarity_probs {
        if probability.is_zero() {
            continue;
        }
        let selector = selector.expect("选择器");
        for (entry, share) in expand_selector(banner, selector, memory) {
            let weight = probability.checked_mul(share).expect("条目概率");
            let slot = items.entry((entry, rarity)).or_insert(Rational::zero());
            *slot = slot.checked_add(weight).expect("条目概率");
        }
    }
    let merged = items
        .into_iter()
        .filter(|(_, p)| !p.is_zero())
        .map(|((entry, rarity), p)| (entry, rarity, p))
        .collect();
    Ok(finish(banner, memory, guarantee, merged))
}

pub(crate) fn step(
    image: &Image,
    memory: &Memory,
    banner: &str,
    action: &str,
) -> Result<FullDraw, RuntimeError> {
    expand_with(image, memory, banner, action)
}

pub(crate) fn reset_action(
    image: &Image,
    memory: &mut Memory,
    banner_name: &str,
) -> Result<(), RuntimeError> {
    let banner = find_banner_opt(image, banner_name)
        .ok_or_else(|| RuntimeError::new("E_BANNER", format!("未知卡池 {banner_name}")))?;
    for state in &banner.states {
        if state.scope != Scope::Action {
            continue;
        }
        let key = slot_key(banner, state);
        let value = if let Some(count) = &state.count {
            SlotVal::Count(count.initial)
        } else {
            SlotVal::Bool(state.bool_initial.unwrap_or(false))
        };
        memory.slots.insert(key, value);
    }
    Ok(())
}

fn finish(
    banner: &BannerRt,
    pre: &Memory,
    guarantee: Option<String>,
    items: Vec<(String, u128, Rational)>,
) -> FullDraw {
    let branches = items
        .into_iter()
        .map(|(entry, rarity, probability)| {
            let (memory, events) = apply_updates(banner, pre, &entry, rarity);
            FullBranch {
                public: Branch {
                    probability,
                    entry,
                    rarity,
                    events,
                },
                memory,
            }
        })
        .collect();
    FullDraw {
        guarantee,
        branches,
    }
}

pub(crate) struct FullDraw {
    pub guarantee: Option<String>,
    pub branches: Vec<FullBranch>,
}

pub(crate) struct FullBranch {
    pub public: Branch,
    pub memory: Memory,
}

fn rarity_distribution(
    banner: &BannerRt,
    dist: &Dist,
    memory: &Memory,
) -> Result<Vec<(u128, Rational, Option<usize>)>, RuntimeError> {
    let mut fixed = Vec::new();
    let mut q = Rational::zero();
    let mut rest = None;
    let mut weights = Vec::new();
    for row in &dist.rows {
        match &row.amount {
            AmountRt::Fixed(node) => {
                let p = eval_chance(banner, node, memory);
                q = q.checked_add(p).expect("固定概率");
                fixed.push((row.rarity, p, Some(row.selector)));
            }
            AmountRt::Rest => rest = Some((row.rarity, row.selector)),
            AmountRt::Weight(weight) => weights.push((row.rarity, *weight, row.selector)),
        }
    }
    let remainder = Rational::one().checked_sub(q).expect("剩余概率");
    if let Some((rarity, selector)) = rest {
        fixed.push((rarity, remainder, Some(selector)));
    } else if !weights.is_empty() {
        let total = weights
            .iter()
            .try_fold(Rational::zero(), |acc, (_, w, _)| acc.checked_add(*w))
            .expect("权重");
        for (rarity, weight, selector) in weights {
            let share = weight.checked_div(total).expect("权重");
            fixed.push((
                rarity,
                remainder.checked_mul(share).expect("权重"),
                Some(selector),
            ));
        }
    }
    Ok(fixed)
}

fn selector_for(dist: &Dist, rarity: u128) -> Option<usize> {
    dist.rows
        .iter()
        .find(|row| row.rarity == rarity)
        .map(|row| row.selector)
}

fn expand_selector(banner: &BannerRt, index: usize, memory: &Memory) -> Vec<(String, Rational)> {
    match &banner.selectors[index] {
        Sel::Uniform(pool) => {
            let members = pool_members(banner, pool);
            let share = Rational::new(1, members.len() as u128);
            members.iter().map(|entry| (entry.clone(), share)).collect()
        }
        Sel::Weighted(branches) => {
            let total = branches
                .iter()
                .try_fold(Rational::zero(), |acc, (w, _)| acc.checked_add(*w))
                .expect("权重");
            let mut merged = BTreeMap::new();
            for (weight, child) in branches {
                let scale = weight.checked_div(total).expect("权重");
                for (entry, probability) in expand_selector(banner, *child, memory) {
                    let add = probability.checked_mul(scale).expect("混合");
                    let slot = merged.entry(entry).or_insert(Rational::zero());
                    *slot = slot.checked_add(add).expect("混合");
                }
            }
            merged.into_iter().collect()
        }
        Sel::If(cond, yes, no) => {
            let next = if eval_cond(banner, cond, memory, None) {
                *yes
            } else {
                *no
            };
            expand_selector(banner, next, memory)
        }
        Sel::Ref(index) => expand_selector(banner, *index, memory),
    }
}

fn apply_updates(
    banner: &BannerRt,
    pre: &Memory,
    entry: &str,
    rarity: u128,
) -> (Memory, Vec<Fired>) {
    let flags: Vec<bool> = banner
        .updates
        .iter()
        .map(|update| eval_cond(banner, &update.when, pre, Some((entry, rarity))))
        .collect();
    let mut memory = pre.clone();
    let mut events = Vec::new();
    for (update, on) in banner.updates.iter().zip(flags) {
        if !on {
            continue;
        }
        for op in &update.ops {
            match op {
                OpRt::SetBool(name, value) => {
                    let key = state_key(banner, name);
                    memory.slots.insert(key, SlotVal::Bool(*value));
                }
                OpRt::Add(name, delta) => {
                    let key = state_key(banner, name);
                    let state = find_state(banner, name);
                    if let Some(SlotVal::Count(current)) = memory.slots.get_mut(&key) {
                        let next = current.saturating_add(*delta);
                        *current = match state.count.as_ref().and_then(|c| c.upper) {
                            Some(limit) => next.min(limit),
                            None => next,
                        };
                    }
                }
                OpRt::Reset(name) => {
                    let key = state_key(banner, name);
                    let state = find_state(banner, name);
                    let initial = state.count.as_ref().map(|c| c.initial).unwrap_or(0);
                    memory.slots.insert(key, SlotVal::Count(initial));
                }
                OpRt::Emit(name, args) => {
                    let args = args
                        .iter()
                        .map(|arg| eval_arg(banner, &memory, entry, rarity, arg))
                        .collect();
                    events.push(Fired {
                        name: name.clone(),
                        args,
                    });
                }
            }
        }
    }
    (memory, events)
}

fn eval_arg(
    banner: &BannerRt,
    memory: &Memory,
    entry: &str,
    rarity: u128,
    arg: &ArgRt,
) -> FiredArg {
    match arg {
        ArgRt::Count(value) => FiredArg::Count(*value),
        ArgRt::Rarity(value) => FiredArg::Rarity(*value),
        ArgRt::Bool(value) => FiredArg::Bool(*value),
        ArgRt::CountState(name) => FiredArg::Count(count_of(banner, memory, name)),
        ArgRt::BoolState(name) => FiredArg::Bool(bool_of(banner, memory, name)),
        ArgRt::Result => FiredArg::Item(entry.to_string()),
        ArgRt::Grade => FiredArg::Rarity(rarity),
        ArgRt::ItemSlot(slot) => FiredArg::Item(bound_item(banner, slot).to_string()),
    }
}

fn eval_cond(
    banner: &BannerRt,
    cond: &CondRt,
    memory: &Memory,
    result: Option<(&str, u128)>,
) -> bool {
    match cond {
        CondRt::Bool(value) => *value,
        CondRt::Not(inner) => !eval_cond(banner, inner, memory, result),
        CondRt::And(left, right) => {
            eval_cond(banner, left, memory, result) && eval_cond(banner, right, memory, result)
        }
        CondRt::Or(left, right) => {
            eval_cond(banner, left, memory, result) || eval_cond(banner, right, memory, result)
        }
        CondRt::CountEq(name, value) => count_of(banner, memory, name) == *value,
        CondRt::CountGe(name, value) => count_of(banner, memory, name) >= *value,
        CondRt::CountMod(name, modulus, remainder) => {
            count_of(banner, memory, name) % modulus == *remainder
        }
        CondRt::BoolEq(name, value) => bool_of(banner, memory, name) == *value,
        CondRt::GradeEq(value) => result.is_some_and(|(_, rarity)| rarity == *value),
        CondRt::GradeGe(value) => result.is_some_and(|(_, rarity)| rarity >= *value),
        CondRt::InPool(pool) => result
            .is_some_and(|(entry, _)| pool_members(banner, pool).iter().any(|item| item == entry)),
    }
}

fn eval_chance(banner: &BannerRt, node: &ChanceRt, memory: &Memory) -> Rational {
    match node {
        ChanceRt::Const(value) => *value,
        ChanceRt::Linear {
            state,
            start,
            base,
            step,
            cap,
            ..
        } => {
            let count = count_of(banner, memory, state);
            let steps = if count < *start { 0 } else { count - start + 1 };
            let raw = base
                .checked_add(step.checked_mul(Rational::new(steps, 1)).expect("步长"))
                .expect("曲线");
            raw.min(*cap)
        }
    }
}

fn count_of(banner: &BannerRt, memory: &Memory, name: &str) -> u128 {
    match memory.slots.get(&state_key(banner, name)) {
        Some(SlotVal::Count(value)) => *value,
        _ => 0,
    }
}

fn bool_of(banner: &BannerRt, memory: &Memory, name: &str) -> bool {
    match memory.slots.get(&state_key(banner, name)) {
        Some(SlotVal::Bool(value)) => *value,
        _ => false,
    }
}

fn state_key(banner: &BannerRt, name: &str) -> String {
    slot_key(banner, find_state(banner, name))
}

fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::Pity => "pity",
        Scope::Progress => "progress",
        Scope::Banner => "banner",
        Scope::Action => "action",
    }
}

fn parse_scope(scope: &str) -> Result<Scope, RuntimeError> {
    match scope {
        "pity" => Ok(Scope::Pity),
        "progress" => Ok(Scope::Progress),
        "banner" => Ok(Scope::Banner),
        "action" => Ok(Scope::Action),
        _ => Err(RuntimeError::new("E_STATE", format!("未知状态域 {scope}"))),
    }
}

fn group_of(banner: &BannerRt, state: &StateRt) -> String {
    match state.scope {
        Scope::Pity => banner.pity_key.clone().unwrap_or_default(),
        Scope::Progress => banner.progress_key.clone().unwrap_or_default(),
        Scope::Banner | Scope::Action => banner.name.clone(),
    }
}

fn slot_key(banner: &BannerRt, state: &StateRt) -> String {
    let group = match state.scope {
        Scope::Pity => banner.pity_key.clone().unwrap_or_default(),
        Scope::Progress => banner.progress_key.clone().unwrap_or_default(),
        Scope::Banner | Scope::Action => banner.name.clone(),
    };
    format!("{:?}|{group}|{}", state.scope, state.name)
}

fn find_banner_opt<'a>(image: &'a Image, name: &str) -> Option<&'a BannerRt> {
    image.banners.iter().find(|banner| banner.name == name)
}

fn find_state_opt<'a>(banner: &'a BannerRt, name: &str) -> Option<&'a StateRt> {
    banner.states.iter().find(|state| state.name == name)
}

fn find_state<'a>(banner: &'a BannerRt, name: &str) -> &'a StateRt {
    banner
        .states
        .iter()
        .find(|state| state.name == name)
        .expect("状态")
}

fn pool_members<'a>(banner: &'a BannerRt, name: &str) -> &'a [String] {
    banner
        .pools
        .iter()
        .find(|(pool, _)| pool == name)
        .map(|(_, members)| members.as_slice())
        .unwrap_or(&[])
}

fn bound_item<'a>(banner: &'a BannerRt, slot: &str) -> &'a str {
    banner
        .items
        .iter()
        .find(|(name, _)| name == slot)
        .map(|(_, entry)| entry.as_str())
        .expect("条目槽")
}

fn entry_rarity(banner: &BannerRt, entry: &str) -> u128 {
    banner
        .entry_rarity
        .iter()
        .find(|(name, _)| name == entry)
        .map(|(_, rarity)| *rarity)
        .expect("稀有度")
}

fn sample_index(branches: &[FullBranch], rng: &mut Rng) -> usize {
    let den = branches.iter().fold(1u128, |acc, branch| {
        lcm(acc, branch.public.probability.denom())
    });
    let weights: Vec<u128> = branches
        .iter()
        .map(|branch| branch.public.probability.numer() * (den / branch.public.probability.denom()))
        .collect();
    let total = weights.iter().sum::<u128>();
    let mut pick = rng.below(total);
    for (index, weight) in weights.iter().enumerate() {
        if pick < *weight {
            return index;
        }
        pick -= weight;
    }
    weights.len() - 1
}

fn lcm(a: u128, b: u128) -> u128 {
    if a == 0 || b == 0 {
        return a.max(b);
    }
    a / crate::rational::gcd(a, b) * b
}

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, limit: u128) -> u128 {
        let limit = u64::try_from(limit).expect("抽样权重");
        let zone = limit.wrapping_neg() % limit;
        loop {
            let value = self.next_u64();
            if value >= zone {
                return u128::from(value % limit);
            }
        }
    }
}
