use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePack {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sources: Vec<SourceDef>,
    #[serde(default)]
    pub assumptions: Vec<AssumptionDef>,
    #[serde(default)]
    pub state: Vec<StateDef>,
    #[serde(default)]
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub pools: Vec<PoolDef>,
    #[serde(default)]
    pub distributions: Vec<DistributionDef>,
    #[serde(default)]
    pub selectors: Vec<SelectorDef>,
    #[serde(default)]
    pub banners: Vec<BannerDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDef {
    pub id: String,
    pub kind: EvidenceKind,
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub verified_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Official,
    Empirical,
    Community,
    Inferred,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssumptionDef {
    pub id: String,
    pub status: AssumptionStatus,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionStatus {
    Official,
    Inferred,
    Experimental,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDef {
    pub id: String,
    pub kind: StateKind,
    pub scope: StateScope,
    pub initial: StateInitial,
    #[serde(default)]
    pub max: Option<u64>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateKind {
    Counter,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StateInitial {
    Counter(u64),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub rarity: u8,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolDef {
    pub id: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionDef {
    pub id: String,
    pub entries: Vec<DistributionEntry>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionEntry {
    pub rarity: u8,
    #[serde(flatten)]
    pub probability: ProbabilityExpr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProbabilityExpr {
    Constant {
        value: String,
    },
    LinearAfter {
        state: String,
        base: String,
        after: u64,
        increment: String,
        #[serde(default)]
        cap: Option<String>,
    },
    Table {
        state: String,
        values: BTreeMap<String, String>,
        #[serde(default)]
        default: Option<String>,
    },
    Remainder,
    ShareOfRemainder {
        weight: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorDef {
    pub id: String,
    #[serde(flatten)]
    pub selector: SelectorExpr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectorExpr {
    Pool {
        pool: String,
    },
    Weighted {
        branches: Vec<WeightedSelectorBranch>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedSelectorBranch {
    pub weight: String,
    pub selector: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerDef {
    pub id: String,
    pub pity_group: String,
    pub progress_group: String,
    #[serde(default)]
    pub actions: Vec<ActionDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: String,
    pub distribution: String,
    pub selectors: BTreeMap<String, String>,
    #[serde(default)]
    pub guarantees: Vec<GuaranteeDef>,
    #[serde(default)]
    pub transitions: Vec<TransitionRuleDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuaranteeDef {
    pub id: String,
    #[serde(default)]
    pub priority: i32,
    pub when: ConditionDef,
    pub effect: GuaranteeEffectDef,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GuaranteeEffectDef {
    ForceItem {
        item: String,
    },
    ForceRarity {
        rarity: u8,
    },
    MinRarity {
        rarity: u8,
        strategy: MinRarityStrategy,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinRarityStrategy {
    ConditionalCurrent,
    PreserveHigherFillFloor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ConditionDef {
    Always,
    All {
        conditions: Vec<ConditionDef>,
    },
    Any {
        conditions: Vec<ConditionDef>,
    },
    Not {
        condition: Box<ConditionDef>,
    },
    StateCounterGte {
        state: String,
        value: u64,
    },
    StateCounterEq {
        state: String,
        value: u64,
    },
    StateBoolEq {
        state: String,
        value: bool,
    },
    StateModuloEq {
        state: String,
        modulus: u64,
        value: u64,
    },
    OutcomeRarityEq {
        rarity: u8,
    },
    OutcomeRarityGte {
        rarity: u8,
    },
    OutcomeItemInPool {
        pool: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRuleDef {
    pub id: String,
    pub when: ConditionDef,
    #[serde(default)]
    pub updates: Vec<StateUpdateDef>,
    #[serde(default)]
    pub events: Vec<EventTemplateDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateUpdateDef {
    Increment {
        state: String,
        #[serde(default = "one")]
        by: u64,
    },
    Reset {
        state: String,
    },
    SetCounter {
        state: String,
        value: u64,
    },
    SetBool {
        state: String,
        value: bool,
    },
}

fn one() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTemplateDef {
    pub kind: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub amount: Option<u64>,
}
