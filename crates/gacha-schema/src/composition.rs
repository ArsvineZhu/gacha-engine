use super::*;

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
