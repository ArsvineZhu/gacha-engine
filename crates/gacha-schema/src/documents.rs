use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Character,
    Weapon,
    LightCone,
    WEngine,
    Arc,
    Resource,
    Token,
    #[default]
    Other,
}

impl ItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Weapon => "weapon",
            Self::LightCone => "light_cone",
            Self::WEngine => "w_engine",
            Self::Arc => "arc",
            Self::Resource => "resource",
            Self::Token => "token",
            Self::Other => "other",
        }
    }
}

/// 机制与符号槽位定义；具体名单由卡池清单提供。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulePack {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sources: Vec<SourceDef>,
    #[serde(default)]
    pub source_documents: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<AssumptionDef>,
    #[serde(default)]
    pub state: Vec<StateDef>,
    pub distributions: Vec<DistributionDef>,
    pub selectors: Vec<SelectorDef>,
    pub banners: Vec<BannerDef>,
}

/// 独立内容身份快照；概率与保底由规则包定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentCatalog {
    pub schema_version: u32,
    pub id: String,
    pub game: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sources: Vec<SourceDef>,
    #[serde(default)]
    pub source_documents: Vec<String>,
    pub items: Vec<ItemDef>,
}

/// 本地卡池组装清单，显式列出每个池的可抽成员。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub game: String,
    #[serde(default)]
    pub description: String,
    pub rules: String,
    pub catalogs: Vec<String>,
    pub pools: Vec<PoolDef>,
    #[serde(default)]
    pub item_bindings: BTreeMap<String, String>,
    #[serde(default)]
    pub banner_bindings: BTreeMap<String, BannerBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BannerBinding {
    pub id: String,
    pub pity_group: String,
    pub progress_group: String,
}
