use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDef {
    pub id: String,
    pub kind: EvidenceKind,
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub verified_at: String,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub sha256: String,
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
