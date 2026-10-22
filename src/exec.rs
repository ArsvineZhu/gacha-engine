use std::collections::BTreeMap;

use crate::image::*;
use crate::lex::Scope;
use crate::rational::Rational;

#[derive(Clone, Debug)]
pub struct RuntimeError {
    pub code: &'static str,
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

#[derive(Clone)]
struct Memory {
    slots: BTreeMap<String, SlotVal>,
}

#[derive(Clone)]
enum SlotVal {
    Count(u128),
    Bool(bool),
}

pub struct Session<'a> {
    image: &'a Image,
    memory: Memory,
}

impl<'a> Session<'a> {
    pub fn new(image: &'a Image) -> Self {
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

    pub fn set_count(&mut self, banner: &str, name: &str, value: u128) {
        let banner = find_banner(self.image, banner);
        let state = find_state(banner, name);
        let key = slot_key(banner, state);
        self.memory.slots.insert(key, SlotVal::Count(value));
    }

    pub fn set_bool(&mut self, banner: &str, name: &str, value: bool) {
        let banner = find_banner(self.image, banner);
        let state = find_state(banner, name);
        let key = slot_key(banner, state);
        self.memory.slots.insert(key, SlotVal::Bool(value));
    }

    pub fn get_count(&self, banner: &str, name: &str) -> u128 {
        let banner = find_banner(self.image, banner);
        let state = find_state(banner, name);
        match self.memory.slots.get(&slot_key(banner, state)) {
            Some(SlotVal::Count(value)) => *value,
            _ => 0,
        }
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
        let chosen = full
            .branches
            .into_iter()
            .find(|branch| branch.public.entry == entry && !branch.public.probability.is_zero())
            .ok_or(RuntimeError {
                code: "E_DRAW_EMPTY_MASS",
            })?;
        self.memory = chosen.memory;
        Ok(chosen.public)
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
        let banner = find_banner(self.image, banner_name);
        let action = banner
            .actions
            .iter()
            .find(|action| action.name == action_name)
            .expect("动作");
        let dist = &banner.distributions[action.distribution];
        let mut guarantee = None;
        let mut rarity_probs = rarity_distribution(banner, dist, &self.memory)?;
        for rule in &banner.guarantees {
            if eval_cond(banner, &rule.when, &self.memory, None) {
                guarantee = Some(rule.name.clone());
                match &rule.effect {
                    EffectRt::Item(slot) => {
                        let entry = bound_item(banner, slot);
                        let rarity = entry_rarity(banner, entry);
                        return Ok(self.finish(
                            banner,
                            guarantee,
                            vec![(entry.to_string(), rarity, Rational::one())],
                        ));
                    }
                    EffectRt::Rarity(rarity) => {
                        rarity_probs =
                            vec![(*rarity, Rational::one(), selector_for(dist, *rarity))];
                    }
                    EffectRt::FloorRescale(k) => {
                        let mass = rarity_probs
                            .iter()
                            .filter(|(rarity, _, _)| *rarity >= *k)
                            .try_fold(Rational::zero(), |acc, (_, p, _)| acc.checked_add(*p))
                            .expect("质量");
                        if mass.is_zero() {
                            return Err(RuntimeError {
                                code: "E_DRAW_EMPTY_MASS",
                            });
                        }
                        rarity_probs = rarity_probs
                            .into_iter()
                            .filter(|(rarity, _, _)| *rarity >= *k)
                            .map(|(rarity, p, sel)| {
                                (rarity, p.checked_div(mass).expect("重抽"), sel)
                            })
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
            for (entry, share) in expand_selector(banner, selector, &self.memory) {
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
        Ok(self.finish(banner, guarantee, merged))
    }

    fn finish(
        &self,
        banner: &BannerRt,
        guarantee: Option<String>,
        items: Vec<(String, u128, Rational)>,
    ) -> FullDraw {
        let branches = items
            .into_iter()
            .map(|(entry, rarity, probability)| {
                let (memory, events) = apply_updates(banner, &self.memory, &entry, rarity);
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
}

struct FullDraw {
    guarantee: Option<String>,
    branches: Vec<FullBranch>,
}

struct FullBranch {
    public: Branch,
    memory: Memory,
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

fn slot_key(banner: &BannerRt, state: &StateRt) -> String {
    let group = match state.scope {
        Scope::Pity => banner.pity_key.clone().unwrap_or_default(),
        Scope::Progress => banner.progress_key.clone().unwrap_or_default(),
        Scope::Banner | Scope::Action => banner.name.clone(),
    };
    format!("{:?}|{group}|{}", state.scope, state.name)
}

fn find_banner<'a>(image: &'a Image, name: &str) -> &'a BannerRt {
    image
        .banners
        .iter()
        .find(|banner| banner.name == name)
        .expect("卡池")
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
