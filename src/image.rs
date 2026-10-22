use std::collections::BTreeMap;

use crate::lex::Scope;
use crate::rational::Rational;

#[derive(Clone, Debug)]
pub struct Image {
    pub banners: Vec<BannerRt>,
}

#[derive(Clone, Debug)]
pub struct BannerRt {
    pub name: String,
    pub pity_key: Option<String>,
    pub progress_key: Option<String>,
    pub states: Vec<StateRt>,
    pub pools: Vec<(String, Vec<String>)>,
    pub items: Vec<(String, String)>,
    pub entry_rarity: Vec<(String, u128)>,
    pub entry_tags: BTreeMap<String, Vec<String>>,
    pub selectors: Vec<Sel>,
    pub distributions: Vec<Dist>,
    pub actions: Vec<ActionRt>,
    pub guarantees: Vec<Guar>,
    pub updates: Vec<UpdateRt>,
}

#[derive(Clone, Debug)]
pub struct StateRt {
    pub name: String,
    pub scope: Scope,
    pub count: Option<CountRt>,
    pub bool_initial: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct CountRt {
    pub initial: u128,
    pub upper: Option<u128>,
}

#[derive(Clone, Debug)]
pub enum Sel {
    Uniform(String),
    Weighted(Vec<(Rational, usize)>),
    If(CondRt, usize, usize),
    Ref(usize),
}

#[derive(Clone, Debug)]
pub struct Dist {
    pub rows: Vec<DistRt>,
}

#[derive(Clone, Debug)]
pub struct DistRt {
    pub rarity: u128,
    pub amount: AmountRt,
    pub selector: usize,
}

#[derive(Clone, Debug)]
pub enum AmountRt {
    Fixed(ChanceRt),
    Rest,
    Weight(Rational),
}

#[derive(Clone, Debug)]
pub enum ChanceRt {
    Const(Rational),
    Linear {
        state: String,
        start: u128,
        base: Rational,
        step: Rational,
        cap: Rational,
        upper: Option<u128>,
    },
}

#[derive(Clone, Debug)]
pub struct ActionRt {
    pub name: String,
    pub lo: u128,
    pub hi: u128,
    pub distribution: usize,
}

#[derive(Clone, Debug)]
pub struct Guar {
    pub name: String,
    pub when: CondRt,
    pub effect: EffectRt,
}

#[derive(Clone, Debug)]
pub enum EffectRt {
    Item(String),
    Rarity(u128),
    FloorRescale(u128),
    FloorPromote(u128),
}

#[derive(Clone, Debug)]
pub struct UpdateRt {
    pub when: CondRt,
    pub ops: Vec<OpRt>,
}

#[derive(Clone, Debug)]
pub enum OpRt {
    SetBool(String, bool),
    Add(String, u128),
    Reset(String),
    Emit(String, Vec<ArgRt>),
}

#[derive(Clone, Debug)]
pub enum ArgRt {
    Count(u128),
    Rarity(u128),
    Bool(bool),
    CountState(String),
    BoolState(String),
    Result,
    Grade,
    ItemSlot(String),
}

#[derive(Clone, Debug)]
pub enum CondRt {
    Bool(bool),
    CountEq(String, u128),
    CountGe(String, u128),
    CountMod(String, u128, u128),
    BoolEq(String, bool),
    GradeEq(u128),
    GradeGe(u128),
    InPool(String),
    Not(Box<CondRt>),
    And(Box<CondRt>, Box<CondRt>),
    Or(Box<CondRt>, Box<CondRt>),
}
