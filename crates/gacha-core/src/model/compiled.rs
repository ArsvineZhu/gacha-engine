use super::*;

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
    pub kind: String,
    pub names: BTreeMap<String, String>,
    pub native_rarity: String,
    pub upstream_id: String,
}

#[derive(Debug, Clone)]
pub struct CompiledPool {
    pub id: String,
    pub items: Vec<ItemId>,
}
