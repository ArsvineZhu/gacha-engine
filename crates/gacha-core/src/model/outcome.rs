use super::*;

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

#[derive(Debug, Clone)]
pub struct OutcomeChoice {
    pub probability: Rational,
    pub outcome: Outcome,
}

#[derive(Debug, Clone)]
pub struct OutcomePlan {
    pub choices: Vec<OutcomeChoice>,
    pub guarantees: Vec<String>,
    pub sampling: Vec<Rational>,
}
