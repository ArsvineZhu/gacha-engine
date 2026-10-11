use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDef {
    pub id: String,
    pub rarity: u8,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub kind: ItemKind,
    #[serde(default)]
    pub names: BTreeMap<String, String>,
    #[serde(default)]
    pub native_rarity: String,
    #[serde(default)]
    pub upstream_id: String,
    #[serde(default)]
    pub source_ids: Vec<String>,
    /// 明确不可抽取的内容仍保留在完整目录；缺省不代表属于任何卡池。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gacha_eligible: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolDef {
    pub id: String,
    pub items: Vec<String>,
}
