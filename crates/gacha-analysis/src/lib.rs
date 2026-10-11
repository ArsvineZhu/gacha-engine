use gacha_core::{
    CompiledGame, EngineError, ItemId, Outcome, Result, ScopeContext, StateStore, enumerate_step,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod first_hit;
mod probability;
mod query;
mod targets;
pub use first_hit::{analyze_first_hit, analyze_nth_hit, analyze_nth_hit_prepared};
pub use probability::probability_of_item_within;
pub use query::*;
pub use targets::target_mask;
use targets::{compile_target, validate_quantiles};
