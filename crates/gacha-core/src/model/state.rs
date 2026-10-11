use super::*;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
