use super::*;

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
