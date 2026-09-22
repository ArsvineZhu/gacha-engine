use crate::Rational;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoolId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectorId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DistributionId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BannerId(pub usize);

#[derive(Debug, Clone)]
pub struct CompiledGame {
    pub id: String,
    pub version: String,
    pub states: Vec<CompiledStateDef>,
    pub items: Vec<CompiledItem>,
    pub pools: Vec<CompiledPool>,
    pub distributions: Vec<CompiledDistribution>,
    pub selectors: Vec<CompiledSelector>,
    pub banners: Vec<CompiledBanner>,
    pub state_lookup: BTreeMap<String, StateId>,
    pub item_lookup: BTreeMap<String, ItemId>,
    pub pool_lookup: BTreeMap<String, PoolId>,
    pub distribution_lookup: BTreeMap<String, DistributionId>,
    pub selector_lookup: BTreeMap<String, SelectorId>,
    pub banner_lookup: BTreeMap<String, BannerId>,
}

impl CompiledGame {
    pub fn banner(&self, id: &str) -> Option<(BannerId, &CompiledBanner)> {
        let index = *self.banner_lookup.get(id)?;
        Some((index, &self.banners[index.0]))
    }

    pub fn item(&self, id: ItemId) -> &CompiledItem {
        &self.items[id.0]
    }

    pub fn item_id(&self, id: &str) -> Option<ItemId> {
        self.item_lookup.get(id).copied()
    }
}

#[derive(Debug, Clone)]
pub struct CompiledItem {
    pub id: String,
    pub rarity: u8,
    pub tags: Vec<String>,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct CompiledPool {
    pub id: String,
    pub items: Vec<ItemId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Counter,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateScope {
    Account,
    Game,
    PityGroup,
    ProgressGroup,
    Banner,
    Session,
    Batch,
    Pull,
}

#[derive(Debug, Clone)]
pub struct CompiledStateDef {
    pub id: String,
    pub kind: StateKind,
    pub scope: StateScope,
    pub initial: StateValue,
    pub max: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StateValue {
    Counter(u64),
    Boolean(bool),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StateStore {
    #[serde(default)]
    pub slots: BTreeMap<String, StateValue>,
}

#[derive(Debug, Clone)]
pub struct ScopeContext {
    pub session: String,
    pub batch: String,
    pub pull: String,
}

impl Default for ScopeContext {
    fn default() -> Self {
        Self {
            session: "default".to_string(),
            batch: "default".to_string(),
            pull: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledDistribution {
    pub id: String,
    pub entries: Vec<CompiledDistributionEntry>,
}

#[derive(Debug, Clone)]
pub struct CompiledDistributionEntry {
    pub rarity: u8,
    pub probability: ProbabilityExpr,
}

#[derive(Debug, Clone)]
pub enum ProbabilityExpr {
    Constant(Rational),
    LinearAfter {
        state: StateId,
        base: Rational,
        after: u64,
        increment: Rational,
        cap: Rational,
    },
    Table {
        state: StateId,
        values: BTreeMap<u64, Rational>,
        default: Option<Rational>,
    },
    Remainder,
    ShareOfRemainder {
        weight: u64,
    },
}

#[derive(Debug, Clone)]
pub struct CompiledSelector {
    pub id: String,
    pub expr: SelectorExpr,
}

#[derive(Debug, Clone)]
pub enum SelectorExpr {
    Pool(PoolId),
    Weighted(Vec<WeightedSelectorBranch>),
}

#[derive(Debug, Clone)]
pub struct WeightedSelectorBranch {
    pub weight: Rational,
    pub selector: SelectorId,
}

#[derive(Debug, Clone)]
pub struct CompiledBanner {
    pub id: String,
    pub pity_group: String,
    pub progress_group: String,
    pub actions: Vec<CompiledAction>,
    pub action_lookup: BTreeMap<String, usize>,
}

impl CompiledBanner {
    pub fn action(&self, id: &str) -> Option<&CompiledAction> {
        let index = *self.action_lookup.get(id)?;
        self.actions.get(index)
    }
}

#[derive(Debug, Clone)]
pub struct CompiledAction {
    pub id: String,
    pub distribution: DistributionId,
    pub selectors: BTreeMap<u8, SelectorId>,
    pub guarantees: Vec<CompiledGuarantee>,
    pub transitions: Vec<CompiledTransitionRule>,
}

#[derive(Debug, Clone)]
pub struct CompiledGuarantee {
    pub id: String,
    pub priority: i32,
    pub when: Condition,
    pub effect: GuaranteeEffect,
}

#[derive(Debug, Clone)]
pub enum GuaranteeEffect {
    ForceItem(ItemId),
    ForceRarity(u8),
    MinRarity {
        rarity: u8,
        strategy: MinRarityStrategy,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinRarityStrategy {
    ConditionalCurrent,
    PreserveHigherFillFloor,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Always,
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Not(Box<Condition>),
    StateCounterGte {
        state: StateId,
        value: u64,
    },
    StateCounterEq {
        state: StateId,
        value: u64,
    },
    StateBoolEq {
        state: StateId,
        value: bool,
    },
    StateModuloEq {
        state: StateId,
        modulus: u64,
        value: u64,
    },
    OutcomeRarityEq(u8),
    OutcomeRarityGte(u8),
    OutcomeItemInPool(PoolId),
}

#[derive(Debug, Clone)]
pub struct CompiledTransitionRule {
    pub id: String,
    pub when: Condition,
    pub updates: Vec<StateUpdate>,
    pub events: Vec<EventTemplate>,
}

#[derive(Debug, Clone)]
pub enum StateUpdate {
    Increment { state: StateId, by: u64 },
    Reset { state: StateId },
    SetCounter { state: StateId, value: u64 },
    SetBool { state: StateId, value: bool },
}

#[derive(Debug, Clone)]
pub struct EventTemplate {
    pub kind: String,
    pub key: String,
    pub amount: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Outcome {
    pub item: ItemId,
    pub rarity: u8,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub probability: Rational,
    pub outcome: Outcome,
    pub guarantees: Vec<String>,
    pub events: Vec<Event>,
    pub next_state: StateStore,
}
