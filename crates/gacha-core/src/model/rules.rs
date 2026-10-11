use super::*;

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
    Conditional {
        when: Condition,
        then_selector: SelectorId,
        else_selector: SelectorId,
    },
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
    pub draws: u32,
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
